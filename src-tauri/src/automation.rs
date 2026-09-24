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
    let (lr, lg, lb) = led_color(&color, kb);
    let static_effect = (kb.effect == Effect::Static).then(|| format!("0,0,{b},0,{lr},{lg},{lb}"));
    LightingPlan {
        commands: vec![format!("LIGHTING APPLY {device} {mode_token} {b} {speed} {lr:02x}{lg:02x}{lb:02x} -")],
        static_effect,
    }
}

/// Cor que o teclado deve receber para parecer com a cor da tela.
pub fn led_color(color: &str, kb: &KeyboardConfig) -> (u8, u8, u8) {
    let (r, g, b) = rgb(color);
    let ch = |c: u8, gain: u8| -> u8 {
        let mut v = c as f64 / 255.0;
        if kb.gamma {
            v = v.powf(2.2);
        }
        (v * 255.0 * gain.min(100) as f64 / 100.0).round() as u8
    };
    (ch(r, kb.balance[0]), ch(g, kb.balance[1]), ch(b, kb.balance[2]))
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
        let p = plan(&raw(), Some(Mode::Turbo));
        // Sem "LIGHTING POWER ON": o APPLY já liga o teclado, e o POWER ON é
        // recusado pelo driver depois que o estático foi gravado direto.
        assert_eq!(p.commands, vec!["LIGHTING APPLY zoned-wmi-keyboard BREATHING 100 0 b026ff -".to_string()]);
        assert_eq!(p.static_effect.as_deref(), Some("0,0,100,0,176,38,255"));
    }

    #[test]
    fn manual_color_uses_first_zone() {
        let kb = KeyboardConfig { follow_mode: false, brightness: 60, ..raw() };
        let p = plan(&kb, Some(Mode::Eco));
        assert_eq!(p.commands[0], "LIGHTING APPLY zoned-wmi-keyboard BREATHING 60 0 ff2a1a -");
        assert_eq!(p.static_effect.as_deref(), Some("0,0,60,0,255,42,26"));
    }

    #[test]
    fn breathing_forces_speed_zero_and_has_no_static_step() {
        let kb = KeyboardConfig { effect: Effect::Breathing, speed: 5, ..raw() };
        let p = plan(&kb, Some(Mode::Turbo));
        assert_eq!(p.commands[0], "LIGHTING APPLY zoned-wmi-keyboard BREATHING 100 0 b026ff -");
        assert_eq!(p.static_effect, None);
    }

    #[test]
    fn neon_keeps_speed() {
        let kb = KeyboardConfig { effect: Effect::Neon, speed: 5, ..raw() };
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

    fn raw() -> KeyboardConfig {
        KeyboardConfig { gamma: false, ..Default::default() }
    }

    #[test]
    fn gamma_off_and_full_balance_keep_screen_color() {
        assert_eq!(led_color("#22c55e", &raw()), (0x22, 0xc5, 0x5e));
    }

    #[test]
    fn gamma_makes_leds_match_the_screen() {
        let kb = KeyboardConfig::default();
        assert!(kb.gamma, "gamma ligado por padrão");
        assert_eq!(led_color("#22c55e", &kb), (3, 145, 28));
        assert_eq!(led_color("#ff8a1f", &kb), (255, 66, 2));
        assert_eq!(led_color("#000000", &kb), (0, 0, 0));
        assert_eq!(led_color("#ffffff", &kb), (255, 255, 255));
    }

    #[test]
    fn balance_scales_each_channel() {
        let kb = KeyboardConfig { balance: [100, 50, 100], ..Default::default() };
        assert_eq!(led_color("#ff8a1f", &kb), (255, 33, 2));
    }

    #[test]
    fn plan_uses_corrected_color() {
        let p = plan(&KeyboardConfig::default(), Some(Mode::Turbo));
        assert_eq!(p.commands[0], "LIGHTING APPLY zoned-wmi-keyboard BREATHING 100 0 7104ff -");
        assert_eq!(p.static_effect.as_deref(), Some("0,0,100,0,113,4,255"));
    }

    #[test]
    fn login_commands_follow_config() {
        let l = LoginConfig { apply: true, mode: Mode::Turbo, fan: FanChoice::Auto };
        assert_eq!(login_commands(&l), vec!["PROFILE performance".to_string(), "FAN AUTO".to_string()]);
        assert!(login_commands(&LoginConfig { apply: false, ..l }).is_empty());
    }
}
