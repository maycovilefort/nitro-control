use crate::config::{Effect, KeyboardConfig, LoginConfig, Palette};
use crate::mode::Mode;

fn hex(c: &str) -> String {
    c.trim_start_matches('#').to_ascii_lowercase()
}

pub fn lighting_commands(device: &str, kb: &KeyboardConfig, palette: &Palette, mode: Option<Mode>) -> Vec<String> {
    if kb.effect == Effect::Off {
        return vec![format!("LIGHTING POWER {device} OFF")];
    }
    let (primary, zones) = if kb.follow_mode {
        let Some(m) = mode else { return Vec::new() };
        (hex(&palette.get(m).primary), "-".to_string())
    } else {
        (hex(&kb.zones[0]), kb.zones.iter().map(|z| hex(z)).collect::<Vec<_>>().join(","))
    };
    let speed = if kb.effect == Effect::Static { 0 } else { kb.speed.min(9) };
    vec![
        format!("LIGHTING POWER {device} ON"),
        format!("LIGHTING APPLY {device} {} {} {speed} {primary} {zones}", kb.effect.token(), kb.brightness.min(100)),
    ]
}

pub fn login_commands(login: &LoginConfig) -> Vec<String> {
    if !login.apply {
        return Vec::new();
    }
    vec![format!("PROFILE {}", login.mode.token()), format!("FAN {}", login.fan.token())]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Effect, FanChoice, KeyboardConfig, LoginConfig, Palette};

    const DEV: &str = "zoned-wmi-keyboard";

    #[test]
    fn follow_mode_uses_mode_primary_for_all_zones() {
        let kb = KeyboardConfig::default();
        let cmds = lighting_commands(DEV, &kb, &Palette::default(), Some(Mode::Turbo));
        assert_eq!(cmds, vec![
            "LIGHTING POWER zoned-wmi-keyboard ON".to_string(),
            "LIGHTING APPLY zoned-wmi-keyboard STATIC 100 0 b026ff -".to_string(),
        ]);
    }

    #[test]
    fn manual_zones_are_listed() {
        let kb = KeyboardConfig { follow_mode: false, effect: Effect::Breathing, speed: 4, brightness: 60, ..Default::default() };
        let cmds = lighting_commands(DEV, &kb, &Palette::default(), Some(Mode::Eco));
        assert_eq!(cmds[1], "LIGHTING APPLY zoned-wmi-keyboard BREATHING 60 4 ff2a1a ff2a1a,ff2a1a,ff8a1f,ff8a1f");
    }

    #[test]
    fn static_forces_speed_zero() {
        let kb = KeyboardConfig { speed: 7, ..Default::default() };
        let cmds = lighting_commands(DEV, &kb, &Palette::default(), Some(Mode::Eco));
        assert_eq!(cmds[1], "LIGHTING APPLY zoned-wmi-keyboard STATIC 100 0 22c55e -");
    }

    #[test]
    fn off_turns_power_off() {
        let kb = KeyboardConfig { effect: Effect::Off, ..Default::default() };
        assert_eq!(lighting_commands(DEV, &kb, &Palette::default(), Some(Mode::Eco)), vec!["LIGHTING POWER zoned-wmi-keyboard OFF".to_string()]);
    }

    #[test]
    fn unknown_mode_does_not_touch_keyboard() {
        assert!(lighting_commands(DEV, &KeyboardConfig::default(), &Palette::default(), None).is_empty());
    }

    #[test]
    fn login_commands_follow_config() {
        let l = LoginConfig { apply: true, mode: Mode::Turbo, fan: FanChoice::Auto };
        assert_eq!(login_commands(&l), vec!["PROFILE performance".to_string(), "FAN AUTO".to_string()]);
        assert!(login_commands(&LoginConfig { apply: false, ..l }).is_empty());
    }
}
