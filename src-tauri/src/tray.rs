use std::sync::{Arc, Mutex};

use tauri::menu::{CheckMenuItem, CheckMenuItemBuilder, MenuBuilder, MenuItem, MenuItemBuilder};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::{AppHandle, Wry};

use crate::config::FanChoice;
use crate::hub::Request;
use crate::mode::{FanMode, Mode};
use crate::state::Snapshot;

#[derive(Clone)]
pub struct Tray {
    icon: TrayIcon<Wry>,
    header: MenuItem<Wry>,
    modes: Vec<(Mode, CheckMenuItem<Wry>)>,
    fans: Vec<(FanChoice, CheckMenuItem<Wry>)>,
    last: Arc<Mutex<String>>,
}

pub fn fmt_temp(v: Option<f32>) -> String {
    v.map(|t| format!("{}°", t.round() as i32)).unwrap_or_else(|| "—".into())
}

pub fn build(app: &AppHandle) -> tauri::Result<Tray> {
    let header = MenuItemBuilder::with_id("header", "CPU — · GPU —").enabled(false).build(app)?;
    let mut modes = Vec::new();
    for m in Mode::ALL {
        modes.push((m, CheckMenuItemBuilder::with_id(format!("mode:{}", m.id()), m.label()).build(app)?));
    }
    let fans = vec![
        (FanChoice::Auto, CheckMenuItemBuilder::with_id("fan:auto", "Ventoinha Auto").build(app)?),
        (FanChoice::Maximum, CheckMenuItemBuilder::with_id("fan:maximum", "Ventoinha Máximo").build(app)?),
    ];
    let open = MenuItemBuilder::with_id("open", "Abrir painel").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Sair").build(app)?;
    let mut mb = MenuBuilder::new(app).item(&header).separator();
    for (_, i) in &modes {
        mb = mb.item(i);
    }
    mb = mb.separator();
    for (_, i) in &fans {
        mb = mb.item(i);
    }
    let menu = mb.separator().item(&open).item(&quit).build()?;
    let mut builder = TrayIconBuilder::with_id("main").menu(&menu).title("—");
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    let icon = builder.on_menu_event(|app, ev| on_menu(app, ev.id().as_ref())).build(app)?;
    Ok(Tray { icon, header, modes, fans, last: Arc::new(Mutex::new(String::new())) })
}

fn on_menu(app: &AppHandle, id: &str) {
    match id {
        "open" => crate::show_window(app),
        "quit" => app.exit(0),
        "fan:auto" => crate::send(app, Request::SetFan(FanChoice::Auto)),
        "fan:maximum" => crate::send(app, Request::SetFan(FanChoice::Maximum)),
        _ => {
            if let Some(m) = id.strip_prefix("mode:").and_then(Mode::from_id) {
                crate::send(app, Request::SetProfile(m));
            }
        }
    }
}

impl Tray {
    pub fn update(&self, s: &Snapshot) {
        let title = format!("{} · {}", fmt_temp(s.sensors.cpu_temp), fmt_temp(s.sensors.gpu_temp));
        let mut last = self.last.lock().unwrap();
        if *last != title {
            let _ = self.icon.set_title(Some(&title));
            let _ = self.header.set_text(format!("CPU {} · GPU {}", fmt_temp(s.sensors.cpu_temp), fmt_temp(s.sensors.gpu_temp)));
            *last = title;
        }
        for (m, item) in &self.modes {
            let _ = item.set_checked(s.mode == Some(*m));
        }
        for (f, item) in &self.fans {
            let on = matches!((f, s.fan_mode), (FanChoice::Auto, Some(FanMode::Auto)) | (FanChoice::Maximum, Some(FanMode::Maximum)));
            let _ = item.set_checked(on);
        }
    }
}
