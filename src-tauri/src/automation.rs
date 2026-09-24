use crate::config::{Effect, KeyboardConfig, LoginConfig, Palette};
use crate::mode::Mode;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct LightingPlan {
    pub commands: Vec<String>,
    /// Linha para `asense_rgb/effect` quando o efeito final é estático.
    pub static_effect: Option<String>,
}

fn hex(c: &str) -> String {
    c.trim_start_matches('#').to_ascii_lowercase()
}

fn rgb(c: &str) -> (u8, u8, u8) {
    let n = u32::from_str_radix(&hex(c), 16).unwrap_or(0);
    ((n >> 16) as u8, (n >> 8) as u8, n as u8)
}

/// Neste modelo o firmware só aceita uma cor global e o daemon não consegue
/// gravar zonas: o estático é feito pintando com respiração (velocidade 0) e
/// depois gravando o efeito estático direto no driver.
pub fn lighting_plan(device: &str, kb: &KeyboardConfig, palette: &Palette, mode: Option<Mode>) -> LightingPlan {
    if kb.effect == Effect::Off {
        return LightingPlan { commands: vec![format!("LIGHTING POWER {device} OFF")], static_effect: None };
    }
    let color = if kb.follow_mode {
        let Some(m) = mode else { return LightingPlan::default() };
        palette.get(m).primary.clone()
    } else {
        kb.zones[0].clone()
    };
    let b = kb.brightness.min(100);
    let (mode_token, speed) = match kb.effect {
        Effect::Neon => ("NEON", kb.speed.min(9)),
        _ => ("BREATHING", 0),
    };
    let static_effect = (kb.effect == Effect::Static).then(|| {
        let (r, g, bl) = rgb(&color);
        format!("0,0,{b},0,{r},{g},{bl}")
    });
    LightingPlan {
        commands: vec![format!("LIGHTING APPLY {device} {mode_token} {b} {speed} {} -", hex(&color))],
        static_effect,
    }
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

    fn plan(kb: &KeyboardConfig, mode: Option<Mode>) -> LightingPlan {
        lighting_plan(DEV, kb, &Palette::default(), mode)
    }

    #[test]
    fn static_paints_with_breathing_then_commits_static_effect() {
        // Neste modelo o daemon não consegue gravar zonas (STATIC falha); a respiração
        // pinta as 4 zonas e o efeito estático é gravado direto no driver.
        let p = plan(&KeyboardConfig::default(), Some(Mode::Turbo));
        // Sem "LIGHTING POWER ON": o APPLY já liga o teclado, e o POWER ON é
        // recusado pelo driver depois que o estático foi gravado direto.
        assert_eq!(p.commands, vec!["LIGHTING APPLY zoned-wmi-keyboard BREATHING 100 0 b026ff -".to_string()]);
        assert_eq!(p.static_effect.as_deref(), Some("0,0,100,0,176,38,255"));
    }

    #[test]
    fn manual_color_uses_first_zone() {
        let kb = KeyboardConfig { follow_mode: false, brightness: 60, ..Default::default() };
        let p = plan(&kb, Some(Mode::Eco));
        assert_eq!(p.commands[0], "LIGHTING APPLY zoned-wmi-keyboard BREATHING 60 0 ff2a1a -");
        assert_eq!(p.static_effect.as_deref(), Some("0,0,60,0,255,42,26"));
    }

    #[test]
    fn breathing_forces_speed_zero_and_has_no_static_step() {
        let kb = KeyboardConfig { effect: Effect::Breathing, speed: 5, ..Default::default() };
        let p = plan(&kb, Some(Mode::Turbo));
        assert_eq!(p.commands[0], "LIGHTING APPLY zoned-wmi-keyboard BREATHING 100 0 b026ff -");
        assert_eq!(p.static_effect, None);
    }

    #[test]
    fn neon_keeps_speed() {
        let kb = KeyboardConfig { effect: Effect::Neon, speed: 5, ..Default::default() };
        let p = plan(&kb, Some(Mode::Turbo));
        assert_eq!(p.commands[0], "LIGHTING APPLY zoned-wmi-keyboard NEON 100 5 b026ff -");
        assert_eq!(p.static_effect, None);
    }

    #[test]
    fn off_turns_power_off() {
        let kb = KeyboardConfig { effect: Effect::Off, ..Default::default() };
        let p = plan(&kb, Some(Mode::Eco));
        assert_eq!(p.commands, vec!["LIGHTING POWER zoned-wmi-keyboard OFF".to_string()]);
        assert_eq!(p.static_effect, None);
    }

    #[test]
    fn unknown_mode_does_not_touch_keyboard() {
        assert_eq!(plan(&KeyboardConfig::default(), None), LightingPlan::default());
    }

    #[test]
    fn login_commands_follow_config() {
        let l = LoginConfig { apply: true, mode: Mode::Turbo, fan: FanChoice::Auto };
        assert_eq!(login_commands(&l), vec!["PROFILE performance".to_string(), "FAN AUTO".to_string()]);
        assert!(login_commands(&LoginConfig { apply: false, ..l }).is_empty());
    }
}
