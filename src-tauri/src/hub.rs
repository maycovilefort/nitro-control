use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::asense::protocol::{parse_caps_device, parse_diag, parse_platform};
use crate::asense::{Asense, AsenseError};
use crate::automation::{lighting_commands, login_commands};
use crate::config::{Config, FanChoice};
use crate::mode::Mode;
use crate::sensors::Sensors;
use crate::state::{diff, Connection, Event, History, Sample, Snapshot};

pub const TICK: Duration = Duration::from_secs(1);
const DIAG_EVERY: Duration = Duration::from_secs(3);
const RECONNECT_EVERY: Duration = Duration::from_secs(2);
const DEFAULT_DEVICE: &str = "zoned-wmi-keyboard";

pub enum Request {
    SetProfile(Mode),
    SetFan(FanChoice),
    /// Final do comando já validado por `platform_tail`.
    SetPlatform(String),
    ReapplyLighting,
}

pub type Reply = Sender<Result<(), String>>;

pub struct Hub<A: Asense> {
    pub(crate) asense: A,
    sensors: Sensors,
    cfg: Arc<Mutex<Config>>,
    history: Arc<Mutex<History>>,
    emit: Box<dyn FnMut(Event) + Send>,
    snap: Snapshot,
    diag_mode: Option<Mode>,
    device: String,
    last_diag: Option<Instant>,
    last_connect_try: Option<Instant>,
    login_applied: bool,
}

impl<A: Asense> Hub<A> {
    pub fn new(asense: A, sensors: Sensors, cfg: Arc<Mutex<Config>>, history: Arc<Mutex<History>>, emit: Box<dyn FnMut(Event) + Send>) -> Self {
        Hub {
            asense,
            sensors,
            cfg,
            history,
            emit,
            snap: Snapshot::default(),
            diag_mode: None,
            device: DEFAULT_DEVICE.into(),
            last_diag: None,
            last_connect_try: None,
            login_applied: false,
        }
    }

    pub fn snapshot(&self) -> &Snapshot {
        &self.snap
    }

    fn toast(&mut self, msg: String) {
        log::warn!("{msg}");
        (self.emit)(Event::Toast(msg));
    }

    fn ensure_connected(&mut self, now: Instant) {
        if self.asense.connected() {
            self.snap.connection = Connection::Connected;
            return;
        }
        if self.last_connect_try.is_some_and(|t| now.duration_since(t) < RECONNECT_EVERY) {
            return;
        }
        self.last_connect_try = Some(now);
        match self.asense.connect() {
            Ok(()) => {
                self.snap.connection = Connection::Connected;
                self.on_connected();
            }
            Err(AsenseError::Busy) => self.snap.connection = Connection::Busy,
            Err(e) => {
                log::debug!("ASense: {e}");
                self.snap.connection = Connection::Disconnected;
            }
        }
    }

    fn on_connected(&mut self) {
        if let Ok(caps) = self.asense.request("CAPS") {
            if let Some(dev) = parse_caps_device(&caps) {
                self.device = dev;
            }
        }
        self.refresh_platform();
        if !self.login_applied {
            self.login_applied = true;
            let cmds = login_commands(&self.cfg.lock().unwrap().login);
            for c in cmds {
                if let Err(e) = self.asense.request(&c) {
                    self.toast(format!("Padrão do login falhou ({c}): {e}"));
                }
            }
        }
        self.last_diag = None;
        self.refresh_diag(Instant::now());
        self.snap.mode = self.sensors.read_mode().or(self.diag_mode);
        if let Err(e) = self.apply_lighting() {
            self.toast(e);
        }
    }

    fn refresh_platform(&mut self) {
        if let Ok(p) = self.asense.request("PLATFORM GET") {
            self.snap.platform = Some(parse_platform(&p));
        }
    }

    fn refresh_diag(&mut self, now: Instant) {
        if !self.asense.connected() || self.last_diag.is_some_and(|t| now.duration_since(t) < DIAG_EVERY) {
            return;
        }
        self.last_diag = Some(now);
        match self.asense.request("DIAG PASSIVE").and_then(|p| parse_diag(&p)) {
            Ok(d) => {
                self.snap.fan_mode = d.fan_mode;
                self.diag_mode = d.mode;
            }
            Err(e) => log::debug!("DIAG: {e}"),
        }
    }

    fn apply_lighting(&mut self) -> Result<(), String> {
        if !self.asense.connected() {
            return Ok(());
        }
        let cmds = {
            let cfg = self.cfg.lock().unwrap();
            lighting_commands(&self.device, &cfg.keyboard, &cfg.palette, self.snap.mode)
        };
        for c in cmds {
            self.asense.request(&c).map_err(|e| format!("Teclado: {e}"))?;
        }
        Ok(())
    }

    pub fn tick(&mut self, now: Instant) {
        let prev = self.snap.clone();
        self.ensure_connected(now);
        if !self.asense.connected() && self.snap.connection == Connection::Connected {
            self.snap.connection = Connection::Disconnected;
        }
        self.snap.sensors = self.sensors.read();
        self.refresh_diag(now);
        self.snap.mode = self.sensors.read_mode().or(self.diag_mode).or(prev.mode);
        let t = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        self.history.lock().unwrap().push(Sample::from_reading(&self.snap.sensors, t));

        let follow = self.cfg.lock().unwrap().keyboard.follow_mode;
        for ev in diff(&prev, &self.snap) {
            if matches!(ev, Event::ModeChanged { from: Some(_), .. }) && follow {
                if let Err(e) = self.apply_lighting() {
                    self.toast(e);
                }
            }
            (self.emit)(ev);
        }
    }

    pub fn handle(&mut self, req: Request) -> Result<(), String> {
        if let Request::ReapplyLighting = req {
            return self.apply_lighting();
        }
        if !self.asense.connected() {
            return Err(AsenseError::NotConnected.to_string());
        }
        let cmd = match &req {
            Request::SetProfile(m) => format!("PROFILE {}", m.token()),
            Request::SetFan(f) => format!("FAN {}", f.token()),
            Request::SetPlatform(tail) => format!("PLATFORM {tail}"),
            Request::ReapplyLighting => unreachable!(),
        };
        self.asense.request(&cmd).map_err(|e| e.to_string())?;
        if let Request::SetPlatform(_) = req {
            self.refresh_platform();
        }
        self.last_diag = None;
        self.tick(Instant::now());
        Ok(())
    }
}

pub fn run<A: Asense>(mut hub: Hub<A>, rx: Receiver<(Request, Reply)>, shared: Arc<Mutex<Snapshot>>) {
    let mut next = Instant::now();
    loop {
        let now = Instant::now();
        if now >= next {
            hub.tick(now);
            *shared.lock().unwrap() = hub.snapshot().clone();
            next = now + TICK;
        }
        match rx.recv_timeout(next.saturating_duration_since(Instant::now())) {
            Ok((req, reply)) => {
                let r = hub.handle(req);
                *shared.lock().unwrap() = hub.snapshot().clone();
                let _ = reply.send(r);
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asense::AsenseError;
    use crate::config::{Config, FanChoice};
    use crate::state::Connection;
    use std::fs;
    use std::path::Path;

    #[derive(Clone, Default)]
    struct Fake {
        log: Arc<Mutex<Vec<String>>>,
        connected: bool,
        busy: bool,
        reject: Option<String>,
    }

    impl Asense for Fake {
        fn connected(&self) -> bool {
            self.connected
        }
        fn connect(&mut self) -> Result<(), AsenseError> {
            if self.busy {
                return Err(AsenseError::Busy);
            }
            self.connected = true;
            Ok(())
        }
        fn request(&mut self, cmd: &str) -> Result<String, AsenseError> {
            if !self.connected {
                return Err(AsenseError::NotConnected);
            }
            self.log.lock().unwrap().push(cmd.to_string());
            if self.reject.as_deref().is_some_and(|r| cmd.starts_with(r)) {
                return Err(AsenseError::Rejected("nope".into()));
            }
            Ok(match cmd {
                "CAPS" => include_str!("../../fixtures/caps.txt").trim_start_matches("OK ").trim_end().to_string(),
                "PLATFORM GET" => include_str!("../../fixtures/platform_get.txt").trim_start_matches("OK ").trim_end().to_string(),
                "DIAG PASSIVE" => include_str!("../../fixtures/diag_passive.txt").trim_start_matches("OK ").trim_end().to_string(),
                _ => "ok".to_string(),
            })
        }
    }

    struct Rig {
        dir: tempfile::TempDir,
        log: Arc<Mutex<Vec<String>>>,
        events: Arc<Mutex<Vec<Event>>>,
        cfg: Arc<Mutex<Config>>,
    }

    fn set_profile_file(dir: &Path, token: &str) {
        let p = dir.join("sys/firmware/acpi/platform_profile");
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, token).unwrap();
    }

    fn rig(fake: Fake) -> (Rig, Hub<Fake>) {
        let dir = tempfile::tempdir().unwrap();
        set_profile_file(dir.path(), "balanced\n");
        let log = fake.log.clone();
        let events = Arc::new(Mutex::new(Vec::new()));
        let ev2 = events.clone();
        let cfg = Arc::new(Mutex::new(Config::default()));
        let sensors = Sensors::new(dir.path().join("sys"), dir.path().join("proc"), Box::new(|| None));
        let hub = Hub::new(fake, sensors, cfg.clone(), Arc::new(Mutex::new(History::new(10))), Box::new(move |e| ev2.lock().unwrap().push(e)));
        (Rig { dir, log, events, cfg }, hub)
    }

    fn cmds(r: &Rig) -> Vec<String> {
        r.log.lock().unwrap().clone()
    }

    #[test]
    fn first_connect_applies_login_then_lighting() {
        let (r, mut hub) = rig(Fake::default());
        hub.tick(Instant::now());
        let c = cmds(&r);
        assert_eq!(hub.snapshot().connection, Connection::Connected);
        let pos = |s: &str| c.iter().position(|x| x == s).unwrap_or_else(|| panic!("faltou {s} em {c:?}"));
        assert!(pos("PROFILE performance") < pos("FAN AUTO"));
        assert!(pos("FAN AUTO") < pos("LIGHTING APPLY zoned-wmi-keyboard STATIC 100 0 ff8a1f -"), "cor do Equilibrado (modo lido do sysfs)");
        assert_eq!(hub.snapshot().platform.as_ref().unwrap().usb_charging, Some(30));
    }

    #[test]
    fn external_mode_change_emits_event_and_recolors_keyboard() {
        let (r, mut hub) = rig(Fake::default());
        let t0 = Instant::now();
        hub.tick(t0);
        r.log.lock().unwrap().clear();
        r.events.lock().unwrap().clear();
        set_profile_file(r.dir.path(), "performance\n");
        hub.tick(t0 + TICK);
        assert!(r.events.lock().unwrap().contains(&Event::ModeChanged { from: Some(Mode::Balanced), to: Mode::Turbo }));
        assert!(cmds(&r).contains(&"LIGHTING APPLY zoned-wmi-keyboard STATIC 100 0 b026ff -".to_string()));
    }

    #[test]
    fn follow_mode_off_leaves_keyboard_alone_on_mode_change() {
        let (r, mut hub) = rig(Fake::default());
        r.cfg.lock().unwrap().keyboard.follow_mode = false;
        let t0 = Instant::now();
        hub.tick(t0);
        r.log.lock().unwrap().clear();
        set_profile_file(r.dir.path(), "quiet\n");
        hub.tick(t0 + TICK);
        assert!(!cmds(&r).iter().any(|c| c.starts_with("LIGHTING")));
    }

    #[test]
    fn unknown_sysfs_token_falls_back_to_daemon_mode() {
        let (r, mut hub) = rig(Fake::default());
        let t0 = Instant::now();
        hub.tick(t0);
        set_profile_file(r.dir.path(), "custom\n");
        hub.tick(t0 + TICK);
        // fixture do DIAG diz "turbo": sem token válido no sysfs, o hub usa o modo do daemon
        assert_eq!(hub.snapshot().mode, Some(Mode::Turbo));
    }

    #[test]
    fn unknown_mode_everywhere_keeps_last_known() {
        let (r, mut hub) = rig(Fake { busy: true, ..Default::default() });
        let t0 = Instant::now();
        hub.tick(t0);
        set_profile_file(r.dir.path(), "custom\n");
        hub.tick(t0 + TICK);
        assert_eq!(hub.snapshot().mode, Some(Mode::Balanced), "sem sysfs nem daemon, mantém o último modo");
    }

    #[test]
    fn login_defaults_only_once() {
        let (r, mut hub) = rig(Fake::default());
        let t0 = Instant::now();
        hub.tick(t0);
        hub.asense.connected = false; // simula queda
        r.log.lock().unwrap().clear();
        hub.tick(t0 + Duration::from_secs(5));
        assert_eq!(hub.snapshot().connection, Connection::Connected);
        assert!(!cmds(&r).contains(&"PROFILE performance".to_string()));
    }

    #[test]
    fn busy_daemon_sets_busy_and_keeps_sensors() {
        let (r, mut hub) = rig(Fake { busy: true, ..Default::default() });
        hub.tick(Instant::now());
        assert_eq!(hub.snapshot().connection, Connection::Busy);
        assert_eq!(hub.snapshot().mode, Some(Mode::Balanced), "modo vem do sysfs mesmo sem daemon");
        assert!(cmds(&r).is_empty());
    }

    #[test]
    fn rejected_command_returns_message_and_stays_connected() {
        let (_r, mut hub) = rig(Fake { reject: Some("FAN MAXIMUM".into()), ..Default::default() });
        hub.tick(Instant::now());
        let err = hub.handle(Request::SetFan(FanChoice::Maximum)).unwrap_err();
        assert!(err.contains("recusou"), "{err}");
        assert_eq!(hub.snapshot().connection, Connection::Connected);
    }

    #[test]
    fn handle_while_disconnected_errors_but_reapply_is_ok() {
        let (_r, mut hub) = rig(Fake { busy: true, ..Default::default() });
        hub.tick(Instant::now());
        assert!(hub.handle(Request::SetProfile(Mode::Eco)).is_err());
        assert!(hub.handle(Request::ReapplyLighting).is_ok());
    }

    #[test]
    fn set_profile_sends_token() {
        let (r, mut hub) = rig(Fake::default());
        hub.tick(Instant::now());
        hub.handle(Request::SetProfile(Mode::Eco)).unwrap();
        assert!(cmds(&r).contains(&"PROFILE low-power".to_string()));
    }
}
