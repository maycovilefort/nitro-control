use serde::Serialize;

use super::AsenseError;
use crate::mode::{FanMode, Mode};

#[derive(Debug, Clone, PartialEq)]
pub enum Reply {
    Ok(String),
    Err(String),
}

pub fn parse_reply(line: &str) -> Result<Reply, AsenseError> {
    let line = line.trim_end_matches(['\r', '\n']);
    if line == "OK" {
        return Ok(Reply::Ok(String::new()));
    }
    if let Some(rest) = line.strip_prefix("OK ") {
        return Ok(Reply::Ok(rest.to_string()));
    }
    if let Some(rest) = line.strip_prefix("ERR") {
        return Ok(Reply::Err(rest.trim_start().to_string()));
    }
    Err(AsenseError::Protocol(format!("resposta inesperada: {line}")))
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DiagState {
    pub mode: Option<Mode>,
    pub fan_mode: Option<FanMode>,
}

pub fn parse_diag(payload: &str) -> Result<DiagState, AsenseError> {
    let v: serde_json::Value = serde_json::from_str(payload)
        .map_err(|e| AsenseError::Protocol(format!("DIAG inválido: {e}")))?;
    let mode = v
        .pointer("/profile/current/value/profile")
        .and_then(|x| x.as_str())
        .and_then(Mode::from_id);
    let fan_mode = v
        .pointer("/fans/channels/0/mode/value")
        .and_then(|x| x.as_str())
        .map(FanMode::from_wire);
    Ok(DiagState { mode, fan_mode })
}

/// `caps=1 {json}` → id do primeiro dispositivo de iluminação.
pub fn parse_caps_device(payload: &str) -> Option<String> {
    let json = payload.split_once(' ')?.1;
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    v.pointer("/lighting/0/id")?.as_str().map(str::to_string)
}

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformState {
    pub battery_limit: Option<bool>,
    pub battery_calibration: Option<bool>,
    pub usb_charging: Option<u8>,
    pub keyboard_timeout: Option<bool>,
    pub boot_sound: Option<bool>,
    pub lcd_override: Option<bool>,
}

pub fn parse_platform(payload: &str) -> PlatformState {
    let mut p = PlatformState::default();
    for pair in payload.split_whitespace() {
        let Some((k, v)) = pair.split_once('=') else { continue };
        let b = match v {
            "on" => Some(true),
            "off" => Some(false),
            _ => None,
        };
        match k {
            "battery_limit" => p.battery_limit = b,
            "battery_calibration" => p.battery_calibration = b,
            "keyboard_timeout" => p.keyboard_timeout = b,
            "boot_sound" => p.boot_sound = b,
            "lcd_override" => p.lcd_override = b,
            "usb_charging" => p.usb_charging = v.parse().ok(),
            _ => {}
        }
    }
    p
}

/// Valida uma opção de plataforma vinda da interface e devolve o final do
/// comando (sem o prefixo `PLATFORM `). Qualquer coisa fora da lista é recusada.
pub fn platform_tail(key: &str, value: &str) -> Option<String> {
    let ok = match key {
        "BATTERY_LIMIT" | "KEYBOARD_TIMEOUT" | "BOOT_SOUND" | "LCD_OVERRIDE" => matches!(value, "ON" | "OFF"),
        "BATTERY_CALIBRATION" => matches!(value, "START" | "STOP"),
        "USB_CHARGING" => matches!(value, "0" | "10" | "20" | "30"),
        _ => false,
    };
    ok.then(|| format!("{key} {value}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::{FanMode, Mode};

    const DIAG: &str = include_str!("../../../fixtures/diag_passive.txt");
    const CAPS: &str = include_str!("../../../fixtures/caps.txt");
    const PLATFORM: &str = include_str!("../../../fixtures/platform_get.txt");

    fn payload(line: &str) -> String {
        match parse_reply(line).unwrap() {
            Reply::Ok(p) => p,
            Reply::Err(e) => panic!("ERR {e}"),
        }
    }

    #[test]
    fn parses_ok_and_err() {
        assert_eq!(parse_reply("OK ready\n").unwrap(), Reply::Ok("ready".into()));
        assert_eq!(parse_reply("OK\n").unwrap(), Reply::Ok(String::new()));
        assert_eq!(parse_reply("ERR unsupported command\n").unwrap(), Reply::Err("unsupported command".into()));
        assert!(parse_reply("garbage").is_err());
    }

    #[test]
    fn parses_real_diag_fixture() {
        let d = parse_diag(&payload(DIAG)).unwrap();
        assert_eq!(d.mode, Some(Mode::Turbo));
        assert_eq!(d.fan_mode, Some(FanMode::Auto));
    }

    #[test]
    fn diag_rejects_non_json() {
        assert!(parse_diag("not json").is_err());
    }

    #[test]
    fn caps_gives_keyboard_device() {
        assert_eq!(parse_caps_device(&payload(CAPS)).as_deref(), Some("zoned-wmi-keyboard"));
        assert_eq!(parse_caps_device("caps=1 {}"), None);
    }

    #[test]
    fn parses_platform_fixture() {
        let p = parse_platform(&payload(PLATFORM));
        assert_eq!(p.battery_limit, Some(false));
        assert_eq!(p.boot_sound, Some(true));
        assert_eq!(p.usb_charging, Some(30));
        assert_eq!(p.keyboard_timeout, Some(false));
    }

    #[test]
    fn platform_tail_whitelist() {
        assert_eq!(platform_tail("BATTERY_LIMIT", "ON").as_deref(), Some("BATTERY_LIMIT ON"));
        assert_eq!(platform_tail("USB_CHARGING", "20").as_deref(), Some("USB_CHARGING 20"));
        assert_eq!(platform_tail("BATTERY_CALIBRATION", "START").as_deref(), Some("BATTERY_CALIBRATION START"));
        assert_eq!(platform_tail("USB_CHARGING", "15"), None);
        assert_eq!(platform_tail("REAR_LOGO", "ON"), None);
        assert_eq!(platform_tail("BOOT_SOUND", "ON; rm"), None);
    }
}
