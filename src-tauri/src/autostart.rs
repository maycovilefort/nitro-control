use std::fs;
use std::path::{Path, PathBuf};

const FILE: &str = "nitro-control.desktop";
const ENTRY: &str = "[Desktop Entry]\nType=Application\nName=Nitro Control\nComment=Painel de controle do Acer Nitro\nExec=nitro-control --hidden\nIcon=nitro-control\nTerminal=false\nX-GNOME-Autostart-enabled=true\n";

pub fn autostart_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("autostart")
}

pub fn enabled(dir: &Path) -> bool {
    dir.join(FILE).exists()
}

pub fn set(dir: &Path, on: bool) -> std::io::Result<()> {
    let p = dir.join(FILE);
    if on {
        fs::create_dir_all(dir)?;
        fs::write(p, ENTRY)
    } else if p.exists() {
        fs::remove_file(p)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggles_desktop_file() {
        let d = tempfile::tempdir().unwrap();
        let dir = d.path().join("autostart");
        assert!(!enabled(&dir));
        set(&dir, true).unwrap();
        assert!(enabled(&dir));
        assert!(fs::read_to_string(dir.join(FILE)).unwrap().contains("Exec=nitro-control --hidden"));
        set(&dir, false).unwrap();
        assert!(!enabled(&dir));
        set(&dir, false).unwrap(); // idempotente
    }
}
