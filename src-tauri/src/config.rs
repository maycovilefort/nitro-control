use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::mode::Mode;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModeColors {
    pub primary: String,
    pub secondary: String,
}

fn mc(p: &str, s: &str) -> ModeColors {
    ModeColors { primary: p.into(), secondary: s.into() }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Palette {
    pub eco: ModeColors,
    pub quiet: ModeColors,
    pub balanced: ModeColors,
    pub performance: ModeColors,
    pub turbo: ModeColors,
}

impl Default for Palette {
    fn default() -> Self {
        Palette {
            eco: mc("#22c55e", "#86efac"),
            quiet: mc("#38bdf8", "#a5e3ff"),
            balanced: mc("#ff8a1f", "#ffc07a"),
            performance: mc("#ff2a1a", "#ff6a2b"),
            turbo: mc("#b026ff", "#ff3df2"),
        }
    }
}

impl Palette {
    pub fn get(&self, m: Mode) -> &ModeColors {
        match m {
            Mode::Eco => &self.eco,
            Mode::Quiet => &self.quiet,
            Mode::Balanced => &self.balanced,
            Mode::Performance => &self.performance,
            Mode::Turbo => &self.turbo,
        }
    }
    fn get_mut(&mut self, m: Mode) -> &mut ModeColors {
        match m {
            Mode::Eco => &mut self.eco,
            Mode::Quiet => &mut self.quiet,
            Mode::Balanced => &mut self.balanced,
            Mode::Performance => &mut self.performance,
            Mode::Turbo => &mut self.turbo,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Effect {
    Off,
    Static,
    Breathing,
    Neon,
}

impl Effect {
    pub fn token(self) -> &'static str {
        match self {
            Effect::Off => "OFF",
            Effect::Static => "STATIC",
            Effect::Breathing => "BREATHING",
            Effect::Neon => "NEON",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FanChoice {
    Auto,
    Maximum,
}

impl FanChoice {
    pub fn token(self) -> &'static str {
        match self {
            FanChoice::Auto => "AUTO",
            FanChoice::Maximum => "MAXIMUM",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct KeyboardConfig {
    pub follow_mode: bool,
    pub effect: Effect,
    pub brightness: u8,
    pub speed: u8,
    pub zones: [String; 4],
    /// Correção de gamma: LEDs respondem de forma linear, a tela não.
    pub gamma: bool,
    /// Ganho de cada canal (R, G, B) em %, para igualar o teclado à tela.
    pub balance: [u8; 3],
}

impl Default for KeyboardConfig {
    fn default() -> Self {
        KeyboardConfig {
            follow_mode: true,
            effect: Effect::Static,
            brightness: 100,
            speed: 5,
            zones: ["#ff2a1a".into(), "#ff2a1a".into(), "#ff8a1f".into(), "#ff8a1f".into()],
            gamma: true,
            balance: [100, 100, 100],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LoginConfig {
    pub apply: bool,
    pub mode: Mode,
    pub fan: FanChoice,
}

impl Default for LoginConfig {
    fn default() -> Self {
        LoginConfig { apply: true, mode: Mode::Turbo, fan: FanChoice::Auto }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Config {
    pub palette: Palette,
    pub keyboard: KeyboardConfig,
    pub login: LoginConfig,
}

pub fn is_hex_color(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
}

impl Config {
    /// Corrige valores fora do intervalo ou cores inválidas usando os padrões.
    pub fn sanitize(&mut self) {
        let kd = KeyboardConfig::default();
        self.keyboard.brightness = self.keyboard.brightness.min(100);
        self.keyboard.speed = self.keyboard.speed.min(9);
        for g in &mut self.keyboard.balance {
            *g = (*g).min(100);
        }
        for (z, d) in self.keyboard.zones.iter_mut().zip(kd.zones.iter()) {
            if !is_hex_color(z) {
                *z = d.clone();
            }
        }
        let pd = Palette::default();
        for m in Mode::ALL {
            let def = pd.get(m).clone();
            let c = self.palette.get_mut(m);
            if !is_hex_color(&c.primary) {
                c.primary = def.primary;
            }
            if !is_hex_color(&c.secondary) {
                c.secondary = def.secondary;
            }
        }
    }
}

pub fn config_path() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("nitro-control").join("config.toml")
}

pub fn load(path: &Path) -> (Config, Option<String>) {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (Config::default(), None),
        Err(e) => return (Config::default(), Some(format!("não consegui ler {}: {e}", path.display()))),
    };
    match toml::from_str::<Config>(&text) {
        Ok(mut c) => {
            c.sanitize();
            (c, None)
        }
        Err(e) => (Config::default(), Some(format!("config inválida em {}: {e}", path.display()))),
    }
}

pub fn save(path: &Path, cfg: &Config) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let text = toml::to_string_pretty(cfg).map_err(|e| std::io::Error::other(e.to_string()))?;
    fs::write(path, text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_spec_palette() {
        let c = Config::default();
        assert_eq!(c.palette.get(Mode::Turbo).primary, "#b026ff");
        assert_eq!(c.palette.get(Mode::Balanced).primary, "#ff8a1f");
        assert_eq!(c.palette.get(Mode::Performance).primary, "#ff2a1a");
        assert!(c.keyboard.follow_mode);
        assert_eq!(c.login.mode, Mode::Turbo);
        assert_eq!(c.login.fan, FanChoice::Auto);
    }

    #[test]
    fn missing_file_gives_defaults_without_warning() {
        let d = tempfile::tempdir().unwrap();
        let (c, warn) = load(&d.path().join("config.toml"));
        assert_eq!(c, Config::default());
        assert!(warn.is_none());
    }

    #[test]
    fn save_and_load_round_trip() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("sub/config.toml");
        let mut c = Config::default();
        c.keyboard.effect = Effect::Neon;
        c.palette.eco.primary = "#00ff00".into();
        save(&p, &c).unwrap();
        let (back, warn) = load(&p);
        assert!(warn.is_none());
        assert_eq!(back, c);
    }

    #[test]
    fn broken_toml_gives_defaults_with_warning() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("config.toml");
        std::fs::write(&p, "isto não é toml = = =").unwrap();
        let (c, warn) = load(&p);
        assert_eq!(c, Config::default());
        assert!(warn.is_some());
    }

    #[test]
    fn partial_file_keeps_other_defaults() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("config.toml");
        std::fs::write(&p, "[keyboard]\nbrightness = 40\n").unwrap();
        let (c, _) = load(&p);
        assert_eq!(c.keyboard.brightness, 40);
        assert!(c.keyboard.follow_mode);
        assert_eq!(c.palette, Palette::default());
    }

    #[test]
    fn sanitize_fixes_invalid_values() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("config.toml");
        std::fs::write(&p, "[keyboard]\nbrightness = 250\nspeed = 40\nzones = [\"vermelho\", \"#00ff00\", \"#GGGGGG\", \"#0000ff\"]\n[palette.turbo]\nprimary = \"roxo\"\nsecondary = \"#ff3df2\"\n").unwrap();
        let (c, _) = load(&p);
        assert_eq!(c.keyboard.brightness, 100);
        assert_eq!(c.keyboard.speed, 9);
        assert_eq!(c.keyboard.zones[0], KeyboardConfig::default().zones[0]);
        assert_eq!(c.keyboard.zones[1], "#00ff00");
        assert_eq!(c.keyboard.zones[2], KeyboardConfig::default().zones[2]);
        assert_eq!(c.palette.turbo.primary, "#b026ff");
    }

    #[test]
    fn sanitize_clamps_balance() {
        let mut c = Config::default();
        c.keyboard.balance = [250, 80, 100];
        c.sanitize();
        assert_eq!(c.keyboard.balance, [100, 80, 100]);
    }

    #[test]
    fn hex_validation() {
        assert!(is_hex_color("#a1B2c3"));
        assert!(!is_hex_color("a1b2c3"));
        assert!(!is_hex_color("#a1b2c"));
        assert!(!is_hex_color("#a1b2cz"));
    }
}
