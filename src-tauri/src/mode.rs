use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Eco,
    Quiet,
    Balanced,
    Performance,
    Turbo,
}

/// (modo, token do kernel/ASense, id da interface, rótulo)
const TABLE: [(Mode, &str, &str, &str); 5] = [
    (Mode::Eco, "low-power", "eco", "Eco"),
    (Mode::Quiet, "quiet", "quiet", "Silencioso"),
    (Mode::Balanced, "balanced", "balanced", "Equilibrado"),
    (Mode::Performance, "balanced-performance", "performance", "Desempenho"),
    (Mode::Turbo, "performance", "turbo", "Turbo"),
];

impl Mode {
    pub const ALL: [Mode; 5] = [Mode::Eco, Mode::Quiet, Mode::Balanced, Mode::Performance, Mode::Turbo];

    fn row(self) -> &'static (Mode, &'static str, &'static str, &'static str) {
        TABLE.iter().find(|r| r.0 == self).expect("todo modo está na tabela")
    }
    pub fn from_token(token: &str) -> Option<Mode> {
        TABLE.iter().find(|r| r.1 == token.trim()).map(|r| r.0)
    }
    pub fn token(self) -> &'static str {
        self.row().1
    }
    pub fn from_id(id: &str) -> Option<Mode> {
        TABLE.iter().find(|r| r.2 == id.trim()).map(|r| r.0)
    }
    pub fn id(self) -> &'static str {
        self.row().2
    }
    pub fn label(self) -> &'static str {
        self.row().3
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FanMode {
    Auto,
    Maximum,
    Manual,
    Unknown,
}

impl FanMode {
    pub fn from_wire(s: &str) -> FanMode {
        match s.trim() {
            "auto" => FanMode::Auto,
            "maximum" => FanMode::Maximum,
            "manual" => FanMode::Manual,
            _ => FanMode::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_table_round_trips() {
        for m in Mode::ALL {
            assert_eq!(Mode::from_token(m.token()), Some(m));
            assert_eq!(Mode::from_id(m.id()), Some(m));
        }
        assert_eq!(Mode::from_token("performance"), Some(Mode::Turbo));
        assert_eq!(Mode::from_token("balanced-performance\n"), Some(Mode::Performance));
    }

    #[test]
    fn unknown_tokens_are_none() {
        assert_eq!(Mode::from_token("custom"), None);
        assert_eq!(Mode::from_id(""), None);
    }

    #[test]
    fn fan_mode_from_wire() {
        assert_eq!(FanMode::from_wire("maximum"), FanMode::Maximum);
        assert_eq!(FanMode::from_wire("weird"), FanMode::Unknown);
    }
}
