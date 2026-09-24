# Nitro Control — Plano de implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Construir o `nitro-control`, um painel no estilo NitroSense em Tauri 2 que substitui a interface do ASense no Acer Nitro AN16-51, com temperaturas, RPM real das ventoinhas, 5 modos com cor e animação, ventoinha Auto/Máximo, RGB do teclado acompanhando o modo, opções de plataforma e ícone na barra do GNOME.

**Architecture:** Um processo Tauri 2 roda sempre. Uma thread `hub`, que não depende do Tauri e é testável, tem a única conexão com o socket do ASense, lê sensores do `/sys`/`/proc`/`nvidia-smi` a cada 1 s, calcula diferenças de estado e emite eventos. A camada Tauri só liga esses eventos à janela (HTML/CSS/JS puro, sem bundler) e ao ícone na barra. A interface chama comandos Tauri, que viram requisições para o hub por um canal.

**Tech Stack:** Rust (edition 2021), Tauri 2 (`tray-icon`), `tauri-plugin-single-instance` 2, serde/serde_json/toml, `dirs`, `log`/`env_logger`, `tempfile` (testes). Frontend: HTML + CSS + ES modules, sem npm. Testes de lógica JS com `node --test` (Node 22 já instalado).

**Spec:** `docs/superpowers/specs/2026-09-23-nitro-control-design.md`

## Global Constraints

- Máquina: Acer Nitro AN16-51, Ubuntu 26.04, GNOME, kernel 7.0, ASense 0.3.0; `acer_wmi` com `predator_v4=1` (já em `/etc/modprobe.d/acer-wmi-predator.conf`).
- Socket: `/run/asense-control.sock`. Primeiro comando `HELLO 2`. Respostas `OK <payload>` ou `ERR <mensagem>`. Comando com no máximo 192 bytes e sem `\n`. **Uma conexão por vez.**
- Leitura de estado do daemon só por `DIAG PASSIVE` (JSON). O `setpoint` das ventoinhas **não** é a rotação real.
- Modo lido de `/sys/firmware/acpi/platform_profile`. Tabela de tokens: `low-power`→eco, `quiet`→quiet, `balanced`→balanced, `balanced-performance`→performance, `performance`→turbo.
- RPM real: hwmon `acer` `fan1_input` (CPU), `fan2_input` (GPU). Máximo de referência: **7050 RPM**.
- GPU: `nvidia-smi --query-gpu=temperature.gpu,utilization.gpu,clocks.gr,power.draw --format=csv,noheader,nounits`, só se `power/runtime_status` da GPU for `active`.
- Paleta padrão: Eco `#22c55e`/`#86efac`, Silencioso `#38bdf8`/`#a5e3ff`, Equilibrado `#ff8a1f`/`#ffc07a`, Desempenho `#ff2a1a`/`#ff6a2b`, Turbo `#b026ff`/`#ff3df2`.
- Rótulos dos modos: Eco, Silencioso, Equilibrado, Desempenho, Turbo. Texto da interface em português do Brasil.
- Efeitos do teclado: `OFF`, `STATIC`, `BREATHING`, `NEON`; brilho 0..100, velocidade 0..9 (0 no estático).
- Sem arte, logo ou imagem da Acer. Ícone próprio.
- A interface só reflete estado confirmado pelo hardware (sem atualização otimista).
- Animação de troca respeita `prefers-reduced-motion`.
- Config: `~/.config/nitro-control/config.toml`.
- Commits terminam com `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. **Daemon reinicia com o app aberto** (socket some e volta): o app precisa reconectar em ~2 s e reativar os controles. Teste na Task 2 (`reconnects_after_server_restart`).
2. **hwmon renumerado** depois de recarregar o `acer_wmi` (`hwmon7` vira `hwmon8`): os sensores precisam continuar lendo, porque a busca é pelo nome. Teste na Task 3 (`finds_hwmon_after_renumbering`).
3. **GPU entra em repouso no meio da execução:** mostrar "em repouso" e **não** chamar `nvidia-smi`, que acordaria a GPU. Teste na Task 3 (`suspended_gpu_is_not_queried`).
4. **Config editada à mão com valor inválido** (cor `vermelho`, brilho 250): o app corrige para valores válidos e não trava. Teste na Task 4 (`sanitize_fixes_invalid_values`).
5. **`platform_profile` com token desconhecido** (por exemplo `custom`, vindo de outro programa): modo vira "—" sem pânico, e a próxima leitura válida volta ao normal. Teste na Task 3 (`unknown_profile_token_is_none`) e na Task 6 (`unknown_mode_does_not_touch_keyboard`).

---

## Estrutura de arquivos

```
nitro-control/
├── fixtures/                      # respostas reais do daemon (já existem)
│   ├── caps.txt  diag_passive.txt  platform_get.txt
├── src-tauri/
│   ├── Cargo.toml  build.rs  tauri.conf.json
│   ├── capabilities/default.json
│   ├── icons/                     # gerados na Task 1
│   └── src/
│       ├── main.rs                # chama lib::run
│       ├── lib.rs                 # montagem Tauri: plugins, estado, janela, eventos
│       ├── mode.rs                # Mode, FanMode + tabela de tokens
│       ├── asense/mod.rs          # trait Asense, AsenseError, Client (socket)
│       ├── asense/protocol.rs     # parse de OK/ERR, DIAG, CAPS, PLATFORM; platform_tail
│       ├── sensors.rs             # hwmon, /proc, GPU, platform_profile
│       ├── config.rs              # Config, Palette, KeyboardConfig, LoginConfig
│       ├── automation.rs          # comandos de iluminação e do login
│       ├── state.rs               # Snapshot, Event, diff, History
│       ├── hub.rs                 # loop central (sem Tauri)
│       ├── autostart.rs           # ~/.config/autostart/nitro-control.desktop
│       ├── tray.rs                # ícone e menu na barra
│       └── commands.rs            # comandos Tauri
├── ui/
│   ├── index.html
│   ├── css/theme.css  css/anim.css
│   └── js/app.js  api.js  demo.js  logic.js  widgets.js
│       └── tabs/home.js  perf.js  keyboard.js  monitor.js  system.js
├── tests-ui/logic.test.mjs
├── tools/make_icon.py
└── packaging/migrar.sh  packaging/reverter.sh  packaging/nitro-control-autostart.desktop
```

---

### Task 1: Toolchain e esqueleto Tauri

**Files:**
- Create: `src-tauri/Cargo.toml`, `src-tauri/build.rs`, `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`, `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`, `ui/index.html`, `tools/make_icon.py`, `src-tauri/icons/*`

**Interfaces:**
- Produces: crate `nitro_control_lib` com `pub fn run()`; janela `main` carregando `ui/index.html`; `window.__TAURI__` global disponível na interface.

- [ ] **Step 1: Instalar dependências de sistema (usuário roda com pkexec)**

Peça ao usuário para rodar:
```
! pkexec apt-get install -y libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev build-essential libssl-dev
```
Confirme: `pkg-config --modversion webkit2gtk-4.1 ayatana-appindicator3-0.1` imprime duas versões.

- [ ] **Step 2: Instalar o Tauri CLI**

Run: `cargo install tauri-cli --version "^2" --locked`
Expected: `cargo tauri --version` imprime `tauri-cli 2.x`.

- [ ] **Step 3: Criar `src-tauri/Cargo.toml`**

```toml
[package]
name = "nitro-control"
version = "0.1.0"
edition = "2021"
description = "Painel de controle estilo NitroSense para Acer Nitro"

[lib]
name = "nitro_control_lib"
crate-type = ["rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = ["tray-icon", "image-png"] }
tauri-plugin-single-instance = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
dirs = "5"
log = "0.4"
env_logger = "0.11"

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 4: Criar `src-tauri/build.rs`**

```rust
fn main() {
    tauri_build::build()
}
```

- [ ] **Step 5: Criar `src-tauri/tauri.conf.json`**

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Nitro Control",
  "version": "0.1.0",
  "identifier": "br.com.vilefort.nitrocontrol",
  "build": { "frontendDist": "../ui" },
  "app": {
    "withGlobalTauri": true,
    "windows": [
      {
        "label": "main",
        "title": "Nitro Control",
        "width": 1180,
        "height": 720,
        "minWidth": 980,
        "minHeight": 620,
        "visible": false,
        "decorations": false,
        "center": true
      }
    ],
    "security": {
      "csp": "default-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:"
    }
  },
  "bundle": {
    "active": true,
    "targets": ["deb"],
    "category": "Utility",
    "shortDescription": "Painel de controle para Acer Nitro",
    "icon": ["icons/32x32.png", "icons/128x128.png", "icons/128x128@2x.png", "icons/icon.png"],
    "linux": { "deb": { "depends": ["asense"] } }
  }
}
```

- [ ] **Step 6: Criar `src-tauri/capabilities/default.json`**

```json
{
  "identifier": "default",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:window:allow-minimize",
    "core:window:allow-start-dragging"
  ]
}
```

- [ ] **Step 7: Criar `src-tauri/src/main.rs` e um `lib.rs` mínimo**

`src-tauri/src/main.rs`:
```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    nitro_control_lib::run()
}
```

`src-tauri/src/lib.rs` (temporário, substituído na Task 7):
```rust
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if let Some(w) = app.get_webview_window("main") {
                w.show()?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o Nitro Control");
}
```

`ui/index.html` (temporário, substituído na Task 8):
```html
<!doctype html>
<html lang="pt-BR"><head><meta charset="utf-8"><title>Nitro Control</title></head>
<body style="background:#0a0605;color:#fff;font-family:sans-serif">Nitro Control — esqueleto</body></html>
```

- [ ] **Step 8: Gerar o ícone próprio**

`tools/make_icon.py` (desenho original: chevron duplo inclinado, sem referência à Acer):
```python
#!/usr/bin/env python3
"""Gera icons/source.png (1024x1024) com um ícone original do Nitro Control."""
from pathlib import Path
from PIL import Image, ImageDraw, ImageFilter

S = 1024
img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
d = ImageDraw.Draw(img)
d.rounded_rectangle([64, 64, S - 64, S - 64], radius=180, fill=(18, 8, 10, 255))
glow = Image.new("RGBA", (S, S), (0, 0, 0, 0))
g = ImageDraw.Draw(glow)
for dx, col in ((0, (255, 42, 26, 255)), (190, (176, 38, 255, 255))):
    g.polygon([(260 + dx, 300), (420 + dx, 300), (600 + dx, 512), (420 + dx, 724), (260 + dx, 724), (440 + dx, 512)], fill=col)
img.alpha_composite(glow.filter(ImageFilter.GaussianBlur(28)))
img.alpha_composite(glow)
out = Path(__file__).resolve().parent.parent / "src-tauri" / "icons"
out.mkdir(parents=True, exist_ok=True)
img.save(out / "source.png")
print(out / "source.png")
```

Run:
```bash
cd ~/nitro-control && python3 tools/make_icon.py && cd src-tauri && cargo tauri icon icons/source.png
```
Expected: `src-tauri/icons/` contém `32x32.png`, `128x128.png`, `128x128@2x.png` e `icon.png`.

- [ ] **Step 9: Compilar e abrir**

Run: `cd ~/nitro-control/src-tauri && cargo build && timeout 8 ./target/debug/nitro-control; echo exit=$?`
Expected: compila; uma janela sem bordas com "Nitro Control — esqueleto" aparece por ~8 s; `exit=124` (morto pelo timeout, sem pânico).

- [ ] **Step 10: Commit**

```bash
cd ~/nitro-control
printf 'src-tauri/target/\n.superpowers/\n' > .gitignore
git add .gitignore src-tauri ui tools
git commit -m "feat: Tauri 2 skeleton with own icon

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Modos e cliente do ASense

**Files:**
- Create: `src-tauri/src/mode.rs`, `src-tauri/src/asense/mod.rs`, `src-tauri/src/asense/protocol.rs`
- Modify: `src-tauri/src/lib.rs` (adicionar `pub mod mode; pub mod asense;` no topo)

**Interfaces:**
- Produces:
  - `mode::Mode` (`Eco|Quiet|Balanced|Performance|Turbo`, serde `"eco"`…), `Mode::ALL`, `from_token(&str)->Option<Mode>`, `token(self)->&'static str`, `from_id(&str)->Option<Mode>`, `id(self)->&'static str`, `label(self)->&'static str`
  - `mode::FanMode` (`Auto|Maximum|Manual|Unknown`, serde lowercase), `FanMode::from_wire(&str)->FanMode`
  - `asense::AsenseError` (Clone, PartialEq, Display): `Unavailable(String)`, `Busy`, `NotConnected`, `Timeout`, `Io(String)`, `Protocol(String)`, `Rejected(String)`
  - `asense::Asense` trait: `connected(&self)->bool`, `connect(&mut self)->Result<(),AsenseError>`, `request(&mut self, cmd:&str)->Result<String,AsenseError>` (retorna o payload após `OK `)
  - `asense::Client::new(path: PathBuf, timeout: Duration)`, `asense::SOCKET_PATH`
  - `asense::protocol::{Reply, parse_reply, DiagState{mode:Option<Mode>, fan_mode:Option<FanMode>}, parse_diag, parse_caps_device, PlatformState, parse_platform, platform_tail}`

- [ ] **Step 1: Escrever `mode.rs` com testes**

```rust
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
```

- [ ] **Step 2: Escrever os testes de `protocol.rs` (falham antes da implementação)**

Crie `src-tauri/src/asense/protocol.rs` só com o bloco de testes abaixo e as assinaturas com `todo!()`:
```rust
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
```

Run: `cd ~/nitro-control/src-tauri && cargo test asense::protocol`
Expected: FAIL (pânico em `todo!()` ou erro de compilação por funções ausentes).

- [ ] **Step 3: Implementar `protocol.rs`** (acima do bloco de testes)

```rust
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
```

- [ ] **Step 4: Escrever os testes do cliente em `asense/mod.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixListener;
    use std::thread;

    /// Servidor falso: responde HELLO e ecoa comandos conhecidos; fecha depois de `max` comandos.
    fn fake_server(path: &std::path::Path, max: usize) -> thread::JoinHandle<()> {
        let listener = UnixListener::bind(path).unwrap();
        thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut w = stream.try_clone().unwrap();
            let mut r = BufReader::new(stream);
            for _ in 0..max {
                let mut line = String::new();
                if r.read_line(&mut line).unwrap() == 0 {
                    return;
                }
                let reply = match line.trim() {
                    "HELLO 2" => "OK protocol=2 daemon=0.3.0",
                    "PING" => "OK ready",
                    "FAN AUTO" => "OK fan=auto",
                    _ => "ERR unsupported command",
                };
                writeln!(w, "{reply}").unwrap();
            }
        })
    }

    fn client(path: &std::path::Path) -> Client {
        Client::new(path.to_path_buf(), Duration::from_millis(300))
    }

    #[test]
    fn handshake_and_commands() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("s.sock");
        let srv = fake_server(&sock, 10);
        let mut c = client(&sock);
        c.connect().unwrap();
        assert!(c.connected());
        assert_eq!(c.request("PING").unwrap(), "ready");
        assert_eq!(c.request("BOGUS"), Err(AsenseError::Rejected("unsupported command".into())));
        assert!(c.connected(), "ERR mantém a sessão");
        drop(c);
        srv.join().unwrap();
    }

    #[test]
    fn missing_socket_is_unavailable() {
        let dir = tempfile::tempdir().unwrap();
        let mut c = client(&dir.path().join("nada.sock"));
        assert!(matches!(c.connect(), Err(AsenseError::Unavailable(_))));
        assert_eq!(c.request("PING"), Err(AsenseError::NotConnected));
    }

    #[test]
    fn silent_server_means_busy() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("s.sock");
        let listener = UnixListener::bind(&sock).unwrap();
        let _hold = thread::spawn(move || {
            let (_s, _) = listener.accept().unwrap();
            thread::sleep(Duration::from_secs(2));
        });
        let mut c = client(&sock);
        assert_eq!(c.connect(), Err(AsenseError::Busy));
        assert!(!c.connected());
    }

    #[test]
    fn rejects_oversized_or_multiline_commands() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("s.sock");
        let _srv = fake_server(&sock, 10);
        let mut c = client(&sock);
        c.connect().unwrap();
        assert!(matches!(c.request(&"X".repeat(193)), Err(AsenseError::Protocol(_))));
        assert!(matches!(c.request("PING\nFAN MAXIMUM"), Err(AsenseError::Protocol(_))));
    }

    #[test]
    fn reconnects_after_server_restart() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("s.sock");
        let srv = fake_server(&sock, 2); // HELLO + PING, depois fecha
        let mut c = client(&sock);
        c.connect().unwrap();
        assert_eq!(c.request("PING").unwrap(), "ready");
        srv.join().unwrap();
        assert!(c.request("PING").is_err());
        assert!(!c.connected(), "queda derruba a conexão");
        std::fs::remove_file(&sock).unwrap();
        let _srv2 = fake_server(&sock, 10);
        c.connect().unwrap();
        assert_eq!(c.request("FAN AUTO").unwrap(), "fan=auto");
    }
}
```

Run: `cargo test asense::tests`
Expected: FAIL (tipos não existem).

- [ ] **Step 5: Implementar `asense/mod.rs`** (acima dos testes)

```rust
pub mod protocol;

use std::fmt;
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

use protocol::{parse_reply, Reply};

pub const SOCKET_PATH: &str = "/run/asense-control.sock";

#[derive(Debug, Clone, PartialEq)]
pub enum AsenseError {
    Unavailable(String),
    Busy,
    NotConnected,
    Timeout,
    Io(String),
    Protocol(String),
    Rejected(String),
}

impl fmt::Display for AsenseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AsenseError::Unavailable(e) => write!(f, "ASense indisponível: {e}"),
            AsenseError::Busy => write!(f, "A interface do ASense está aberta — feche-a para usar o Nitro Control"),
            AsenseError::NotConnected => write!(f, "ASense desconectado"),
            AsenseError::Timeout => write!(f, "ASense não respondeu a tempo"),
            AsenseError::Io(e) => write!(f, "Erro de comunicação com o ASense: {e}"),
            AsenseError::Protocol(e) => write!(f, "Resposta inválida do ASense: {e}"),
            AsenseError::Rejected(e) => write!(f, "O ASense recusou o comando: {e}"),
        }
    }
}

pub trait Asense: Send {
    fn connected(&self) -> bool;
    fn connect(&mut self) -> Result<(), AsenseError>;
    /// Envia um comando e devolve o payload após `OK `.
    fn request(&mut self, cmd: &str) -> Result<String, AsenseError>;
}

pub struct Client {
    path: PathBuf,
    timeout: Duration,
    conn: Option<BufReader<UnixStream>>,
}

impl Client {
    pub fn new(path: PathBuf, timeout: Duration) -> Self {
        Client { path, timeout, conn: None }
    }
}

impl Asense for Client {
    fn connected(&self) -> bool {
        self.conn.is_some()
    }

    fn connect(&mut self) -> Result<(), AsenseError> {
        self.conn = None;
        let s = UnixStream::connect(&self.path).map_err(|e| AsenseError::Unavailable(e.to_string()))?;
        s.set_read_timeout(Some(self.timeout)).map_err(|e| AsenseError::Io(e.to_string()))?;
        s.set_write_timeout(Some(self.timeout)).map_err(|e| AsenseError::Io(e.to_string()))?;
        self.conn = Some(BufReader::new(s));
        match self.request("HELLO 2") {
            Ok(p) if p.starts_with("protocol=2") => Ok(()),
            Ok(p) => {
                self.conn = None;
                Err(AsenseError::Protocol(format!("handshake inesperado: {p}")))
            }
            Err(AsenseError::Timeout) => {
                self.conn = None;
                Err(AsenseError::Busy)
            }
            Err(e) => {
                self.conn = None;
                Err(e)
            }
        }
    }

    fn request(&mut self, cmd: &str) -> Result<String, AsenseError> {
        if cmd.len() > 192 || cmd.contains('\n') || cmd.contains('\r') {
            return Err(AsenseError::Protocol("comando inválido".into()));
        }
        let conn = self.conn.as_mut().ok_or(AsenseError::NotConnected)?;
        let res: io::Result<String> = (|| {
            conn.get_mut().write_all(format!("{cmd}\n").as_bytes())?;
            let mut line = String::new();
            if conn.read_line(&mut line)? == 0 {
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "socket fechado"));
            }
            Ok(line)
        })();
        match res {
            Ok(line) => match parse_reply(&line) {
                Ok(Reply::Ok(p)) => Ok(p),
                Ok(Reply::Err(m)) => Err(AsenseError::Rejected(m)),
                Err(e) => {
                    self.conn = None;
                    Err(e)
                }
            },
            Err(e) if matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut) => {
                self.conn = None;
                Err(AsenseError::Timeout)
            }
            Err(e) => {
                self.conn = None;
                Err(AsenseError::Io(e.to_string()))
            }
        }
    }
}
```

Adicione no topo de `lib.rs`: `pub mod asense;` e `pub mod mode;`.

- [ ] **Step 6: Rodar os testes**

Run: `cargo test -- mode:: asense::`
Expected: todos PASS (3 de `mode`, 6 de `protocol`, 5 do cliente).

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src
git commit -m "feat: ASense socket client, protocol parsing and mode table

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Sensores

**Files:**
- Create: `src-tauri/src/sensors.rs`
- Modify: `src-tauri/src/lib.rs` (`pub mod sensors;`)

**Interfaces:**
- Consumes: `mode::Mode::from_token`
- Produces:
  - `sensors::GpuReading { temp, usage, clock_mhz, power_w: Option<f32> }` (serde camelCase)
  - `sensors::GpuStatus` (`Active(GpuReading) | Sleeping | Absent`, serde `tag="state"`, lowercase; `Default = Absent`)
  - `sensors::SensorReading { cpu_temp, gpu_temp, sys_temp, ssd_temp: Option<f32>, cpu_fan_rpm, gpu_fan_rpm: Option<u32>, cpu_usage: Option<f32> /*0..100*/, ram_used_gb, ram_total_gb: Option<f32>, gpu: GpuStatus }` (Default, Clone, PartialEq, serde camelCase)
  - `sensors::Sensors::new(sys: PathBuf, proc_root: PathBuf, gpu: Box<dyn FnMut() -> Option<GpuReading> + Send>)`, `Sensors::system()`, `read_mode(&self) -> Option<Mode>`, `read(&mut self) -> SensorReading`
  - `sensors::parse_nvidia_smi(&str) -> Option<GpuReading>`

- [ ] **Step 1: Escrever os testes**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn w(root: &Path, rel: &str, content: &str) {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, content).unwrap();
    }

    /// Árvore falsa parecida com a máquina real.
    fn fake_tree() -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        let sys = d.path().join("sys");
        w(&sys, "class/hwmon/hwmon1/name", "acpitz\n");
        w(&sys, "class/hwmon/hwmon1/temp1_input", "45000\n");
        w(&sys, "class/hwmon/hwmon3/name", "nvme\n");
        w(&sys, "class/hwmon/hwmon3/temp1_input", "35850\n");
        w(&sys, "class/hwmon/hwmon6/name", "coretemp\n");
        w(&sys, "class/hwmon/hwmon6/temp1_input", "57000\n");
        w(&sys, "class/hwmon/hwmon7/name", "acer\n");
        w(&sys, "class/hwmon/hwmon7/fan1_input", "4007\n");
        w(&sys, "class/hwmon/hwmon7/fan2_input", "4045\n");
        w(&sys, "class/hwmon/hwmon7/temp1_input", "57000\n");
        w(&sys, "class/hwmon/hwmon7/temp2_input", "0\n");
        w(&sys, "class/hwmon/hwmon7/temp3_input", "51000\n");
        w(&sys, "firmware/acpi/platform_profile", "performance\n");
        w(&sys, "bus/pci/devices/0000:01:00.0/vendor", "0x10de\n");
        w(&sys, "bus/pci/devices/0000:01:00.0/class", "0x030000\n");
        w(&sys, "bus/pci/devices/0000:01:00.0/power/runtime_status", "active\n");
        w(&sys, "bus/pci/devices/0000:01:00.1/vendor", "0x10de\n");
        w(&sys, "bus/pci/devices/0000:01:00.1/class", "0x040300\n");
        w(&sys, "bus/pci/devices/0000:01:00.1/power/runtime_status", "suspended\n");
        let proc_root = d.path().join("proc");
        w(&proc_root, "stat", "cpu  100 0 100 800 0 0 0 0 0 0\ncpu0 1 1 1 1\n");
        w(&proc_root, "meminfo", "MemTotal:       16777216 kB\nMemFree: 1 kB\nMemAvailable:    8388608 kB\n");
        d
    }

    fn gpu_ok() -> Box<dyn FnMut() -> Option<GpuReading> + Send> {
        Box::new(|| parse_nvidia_smi("43, 12, 1740, 62.5\n"))
    }

    fn sensors(d: &tempfile::TempDir, gpu: Box<dyn FnMut() -> Option<GpuReading> + Send>) -> Sensors {
        Sensors::new(d.path().join("sys"), d.path().join("proc"), gpu)
    }

    #[test]
    fn reads_real_layout() {
        let d = fake_tree();
        let mut s = sensors(&d, gpu_ok());
        let r = s.read();
        assert_eq!(r.cpu_temp, Some(57.0));
        assert_eq!(r.sys_temp, Some(51.0), "acer temp3 tem prioridade sobre acpitz");
        assert_eq!(r.ssd_temp, Some(35.85));
        assert_eq!(r.cpu_fan_rpm, Some(4007));
        assert_eq!(r.gpu_fan_rpm, Some(4045));
        assert_eq!(r.gpu_temp, Some(43.0));
        assert_eq!(r.ram_total_gb, Some(16.0));
        assert_eq!(r.ram_used_gb, Some(8.0));
        assert_eq!(r.cpu_usage, None, "primeira leitura não tem delta");
        match r.gpu {
            GpuStatus::Active(g) => {
                assert_eq!(g.usage, Some(12.0));
                assert_eq!(g.clock_mhz, Some(1740.0));
                assert_eq!(g.power_w, Some(62.5));
            }
            other => panic!("esperava Active, veio {other:?}"),
        }
    }

    #[test]
    fn cpu_usage_from_delta() {
        let d = fake_tree();
        let mut s = sensors(&d, gpu_ok());
        s.read();
        // +100 ocupado, +100 ocioso → 50%
        w(&d.path().join("proc"), "stat", "cpu  150 0 150 900 0 0 0 0 0 0\n");
        assert_eq!(s.read().cpu_usage, Some(50.0));
    }

    #[test]
    fn reads_mode_from_platform_profile() {
        let d = fake_tree();
        let s = sensors(&d, gpu_ok());
        assert_eq!(s.read_mode(), Some(crate::mode::Mode::Turbo));
    }

    #[test]
    fn unknown_profile_token_is_none() {
        let d = fake_tree();
        let s = sensors(&d, gpu_ok());
        w(&d.path().join("sys"), "firmware/acpi/platform_profile", "custom\n");
        assert_eq!(s.read_mode(), None);
        w(&d.path().join("sys"), "firmware/acpi/platform_profile", "quiet\n");
        assert_eq!(s.read_mode(), Some(crate::mode::Mode::Quiet));
    }

    #[test]
    fn suspended_gpu_is_not_queried() {
        let d = fake_tree();
        w(&d.path().join("sys"), "bus/pci/devices/0000:01:00.0/power/runtime_status", "suspended\n");
        let calls = Arc::new(AtomicUsize::new(0));
        let c2 = calls.clone();
        let mut s = sensors(&d, Box::new(move || {
            c2.fetch_add(1, Ordering::SeqCst);
            None
        }));
        let r = s.read();
        assert_eq!(r.gpu, GpuStatus::Sleeping);
        assert_eq!(r.gpu_temp, None, "acer temp2 = 0 com GPU dormindo vira None");
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn no_nvidia_device_is_absent() {
        let d = fake_tree();
        fs::remove_dir_all(d.path().join("sys/bus")).unwrap();
        let mut s = sensors(&d, gpu_ok());
        assert_eq!(s.read().gpu, GpuStatus::Absent);
    }

    #[test]
    fn finds_hwmon_after_renumbering() {
        let d = fake_tree();
        let mut s = sensors(&d, gpu_ok());
        assert_eq!(s.read().cpu_fan_rpm, Some(4007));
        let h = d.path().join("sys/class/hwmon");
        fs::rename(h.join("hwmon7"), h.join("hwmon8")).unwrap();
        assert_eq!(s.read().cpu_fan_rpm, Some(4007));
    }

    #[test]
    fn missing_sensors_are_none() {
        let d = tempfile::tempdir().unwrap();
        let mut s = Sensors::new(d.path().join("sys"), d.path().join("proc"), Box::new(|| None));
        let r = s.read();
        assert_eq!(r, SensorReading::default());
        assert_eq!(s.read_mode(), None);
    }

    #[test]
    fn nvidia_smi_parsing() {
        let g = parse_nvidia_smi("43, 0, 210, 1.28\n").unwrap();
        assert_eq!(g.temp, Some(43.0));
        assert_eq!(g.usage, Some(0.0));
        let na = parse_nvidia_smi("43, [N/A], 210, [N/A]").unwrap();
        assert_eq!(na.usage, None);
        assert_eq!(na.power_w, None);
        assert!(parse_nvidia_smi("").is_none());
        assert!(parse_nvidia_smi("1, 2").is_none());
    }
}
```

Run: `cargo test sensors::`
Expected: FAIL (módulo vazio).

- [ ] **Step 2: Implementar `sensors.rs`** (acima dos testes)

```rust
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

use crate::mode::Mode;

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuReading {
    pub temp: Option<f32>,
    pub usage: Option<f32>,
    pub clock_mhz: Option<f32>,
    pub power_w: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum GpuStatus {
    Active(GpuReading),
    Sleeping,
    #[default]
    Absent,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SensorReading {
    pub cpu_temp: Option<f32>,
    pub gpu_temp: Option<f32>,
    pub sys_temp: Option<f32>,
    pub ssd_temp: Option<f32>,
    pub cpu_fan_rpm: Option<u32>,
    pub gpu_fan_rpm: Option<u32>,
    pub cpu_usage: Option<f32>,
    pub ram_used_gb: Option<f32>,
    pub ram_total_gb: Option<f32>,
    pub gpu: GpuStatus,
}

pub struct Sensors {
    sys: PathBuf,
    proc_root: PathBuf,
    prev_cpu: Option<(u64, u64)>,
    gpu: Box<dyn FnMut() -> Option<GpuReading> + Send>,
}

fn read_num(p: &Path) -> Option<f64> {
    fs::read_to_string(p).ok()?.trim().parse().ok()
}

fn find_hwmon(sys: &Path, name: &str) -> Option<PathBuf> {
    let mut dirs: Vec<PathBuf> = fs::read_dir(sys.join("class/hwmon")).ok()?.flatten().map(|e| e.path()).collect();
    dirs.sort();
    dirs.into_iter()
        .find(|p| fs::read_to_string(p.join("name")).map(|n| n.trim() == name).unwrap_or(false))
}

/// Temperatura em °C; 0 ou ausente vira None.
fn temp(dir: &Option<PathBuf>, file: &str) -> Option<f32> {
    let v = read_num(&dir.as_ref()?.join(file))?;
    (v > 0.0).then(|| (v / 1000.0) as f32)
}

fn rpm(dir: &Option<PathBuf>, file: &str) -> Option<u32> {
    read_num(&dir.as_ref()?.join(file)).map(|v| v as u32)
}

pub fn parse_nvidia_smi(out: &str) -> Option<GpuReading> {
    let line = out.lines().next()?;
    let f: Vec<Option<f32>> = line.split(',').map(|x| x.trim().parse().ok()).collect();
    if f.len() != 4 {
        return None;
    }
    Some(GpuReading { temp: f[0], usage: f[1], clock_mhz: f[2], power_w: f[3] })
}

fn nvidia_smi() -> Option<GpuReading> {
    let out = Command::new("nvidia-smi")
        .args([
            "--query-gpu=temperature.gpu,utilization.gpu,clocks.gr,power.draw",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    parse_nvidia_smi(&String::from_utf8_lossy(&out.stdout))
}

impl Sensors {
    pub fn new(sys: PathBuf, proc_root: PathBuf, gpu: Box<dyn FnMut() -> Option<GpuReading> + Send>) -> Self {
        Sensors { sys, proc_root, prev_cpu: None, gpu }
    }

    pub fn system() -> Self {
        Self::new("/sys".into(), "/proc".into(), Box::new(nvidia_smi))
    }

    pub fn read_mode(&self) -> Option<Mode> {
        let s = fs::read_to_string(self.sys.join("firmware/acpi/platform_profile")).ok()?;
        Mode::from_token(&s)
    }

    /// Placa de vídeo NVIDIA (classe 0x03xxxx): Some(true) se ativa, Some(false) se dormindo.
    fn gpu_active(&self) -> Option<bool> {
        for e in fs::read_dir(self.sys.join("bus/pci/devices")).ok()?.flatten() {
            let p = e.path();
            let vendor = fs::read_to_string(p.join("vendor")).unwrap_or_default();
            let class = fs::read_to_string(p.join("class")).unwrap_or_default();
            if vendor.trim() == "0x10de" && class.trim().starts_with("0x03") {
                let st = fs::read_to_string(p.join("power/runtime_status")).unwrap_or_default();
                return Some(st.trim() == "active");
            }
        }
        None
    }

    fn cpu_usage(&mut self) -> Option<f32> {
        let stat = fs::read_to_string(self.proc_root.join("stat")).ok()?;
        let nums: Vec<u64> = stat.lines().next()?.split_whitespace().skip(1).filter_map(|x| x.parse().ok()).collect();
        if nums.len() < 5 {
            return None;
        }
        let total: u64 = nums.iter().take(8).sum();
        let idle = nums[3] + nums[4];
        let prev = self.prev_cpu.replace((total, idle))?;
        let dt = total.saturating_sub(prev.0);
        if dt == 0 {
            return None;
        }
        let di = idle.saturating_sub(prev.1);
        Some(((1.0 - di as f64 / dt as f64) * 100.0) as f32)
    }

    fn memory(&self) -> (Option<f32>, Option<f32>) {
        let Ok(m) = fs::read_to_string(self.proc_root.join("meminfo")) else { return (None, None) };
        let field = |name: &str| -> Option<f64> {
            m.lines().find(|l| l.starts_with(name))?.split_whitespace().nth(1)?.parse().ok()
        };
        let gb = |kb: f64| (kb / 1024.0 / 1024.0) as f32;
        match (field("MemTotal:"), field("MemAvailable:")) {
            (Some(t), Some(a)) => (Some(gb(t - a)), Some(gb(t))),
            (Some(t), None) => (None, Some(gb(t))),
            _ => (None, None),
        }
    }

    pub fn read(&mut self) -> SensorReading {
        let acer = find_hwmon(&self.sys, "acer");
        let core = find_hwmon(&self.sys, "coretemp");
        let acpi = find_hwmon(&self.sys, "acpitz");
        let nvme = find_hwmon(&self.sys, "nvme");
        let gpu = match self.gpu_active() {
            Some(true) => (self.gpu)().map(GpuStatus::Active).unwrap_or(GpuStatus::Absent),
            Some(false) => GpuStatus::Sleeping,
            None => GpuStatus::Absent,
        };
        let gpu_temp = match &gpu {
            GpuStatus::Active(g) => g.temp.or_else(|| temp(&acer, "temp2_input")),
            _ => None,
        };
        let (ram_used_gb, ram_total_gb) = self.memory();
        SensorReading {
            cpu_temp: temp(&core, "temp1_input").or_else(|| temp(&acer, "temp1_input")),
            gpu_temp,
            sys_temp: temp(&acer, "temp3_input").or_else(|| temp(&acpi, "temp1_input")),
            ssd_temp: temp(&nvme, "temp1_input"),
            cpu_fan_rpm: rpm(&acer, "fan1_input"),
            gpu_fan_rpm: rpm(&acer, "fan2_input"),
            cpu_usage: self.cpu_usage(),
            ram_used_gb,
            ram_total_gb,
            gpu,
        }
    }
}
```

- [ ] **Step 3: Rodar os testes**

Run: `cargo test sensors::`
Expected: 9 PASS.

- [ ] **Step 4: Conferir contra a máquina real**

Adicione temporariamente em `src-tauri/examples/sensors.rs`:
```rust
fn main() {
    let mut s = nitro_control_lib::sensors::Sensors::system();
    s.read();
    std::thread::sleep(std::time::Duration::from_secs(1));
    println!("{:#?}\nmode={:?}", s.read(), s.read_mode());
}
```
Run: `cargo run --example sensors`
Expected: `cpu_temp`, `sys_temp`, `cpu_fan_rpm` e `gpu_fan_rpm` preenchidos; `gpu` é `Active` ou `Sleeping`; `mode=Some(Turbo)` (ou o modo atual). Apague o exemplo depois (`rm -r src-tauri/examples`).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/sensors.rs src-tauri/src/lib.rs
git commit -m "feat: sensors (hwmon by name, real fan RPM, GPU without waking it)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Configuração e autostart

**Files:**
- Create: `src-tauri/src/config.rs`, `src-tauri/src/autostart.rs`
- Modify: `src-tauri/src/lib.rs` (`pub mod config; pub mod autostart;`)

**Interfaces:**
- Consumes: `mode::Mode`
- Produces:
  - `config::ModeColors { primary: String, secondary: String }`
  - `config::Palette { eco, quiet, balanced, performance, turbo: ModeColors }`, `Palette::get(&self, Mode) -> &ModeColors`
  - `config::Effect` (`Off|Static|Breathing|Neon`, serde lowercase), `Effect::token(self) -> &'static str`
  - `config::FanChoice` (`Auto|Maximum`, serde lowercase), `FanChoice::token(self) -> &'static str` (`"AUTO"`/`"MAXIMUM"`)
  - `config::KeyboardConfig { follow_mode: bool, effect: Effect, brightness: u8, speed: u8, zones: [String; 4] }`
  - `config::LoginConfig { apply: bool, mode: Mode, fan: FanChoice }`
  - `config::Config { palette, keyboard, login }` (serde camelCase, `#[serde(default)]`), `Config::sanitize(&mut self)`
  - `config::{config_path() -> PathBuf, load(&Path) -> (Config, Option<String>), save(&Path, &Config) -> std::io::Result<()>, is_hex_color(&str) -> bool}`
  - `autostart::{enabled(dir: &Path) -> bool, set(dir: &Path, on: bool) -> std::io::Result<()>, autostart_dir() -> PathBuf}`

- [ ] **Step 1: Escrever os testes de `config.rs`**

```rust
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
    fn hex_validation() {
        assert!(is_hex_color("#a1B2c3"));
        assert!(!is_hex_color("a1b2c3"));
        assert!(!is_hex_color("#a1b2c"));
        assert!(!is_hex_color("#a1b2cz"));
    }
}
```

Run: `cargo test config::`
Expected: FAIL.

- [ ] **Step 2: Implementar `config.rs`**

```rust
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
}

impl Default for KeyboardConfig {
    fn default() -> Self {
        KeyboardConfig {
            follow_mode: true,
            effect: Effect::Static,
            brightness: 100,
            speed: 5,
            zones: ["#ff2a1a".into(), "#ff2a1a".into(), "#ff8a1f".into(), "#ff8a1f".into()],
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
```

Observação: o teste `sanitize_fixes_invalid_values` usa `"#GGGGGG"` e `"vermelho"`; `zones` com um valor não string faria o TOML falhar inteiro, e esse caso cai em `broken_toml_gives_defaults_with_warning`.

- [ ] **Step 3: Escrever testes e implementação de `autostart.rs`**

```rust
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
```

- [ ] **Step 4: Rodar os testes**

Run: `cargo test -- config:: autostart::`
Expected: 8 PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src
git commit -m "feat: config file with palette/keyboard/login defaults and autostart

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Estado, diferenças e histórico

**Files:**
- Create: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/lib.rs` (`pub mod state;`)

**Interfaces:**
- Consumes: `mode::{Mode, FanMode}`, `sensors::{SensorReading, GpuStatus}`, `asense::protocol::PlatformState`
- Produces:
  - `state::Connection` (`Connected|Disconnected|Busy`, serde lowercase, Default = Disconnected)
  - `state::Snapshot { connection, mode: Option<Mode>, fan_mode: Option<FanMode>, sensors: SensorReading, platform: Option<PlatformState> }` (Default, Clone, PartialEq, serde camelCase)
  - `state::Event` (`Snapshot(Snapshot) | ModeChanged { from: Option<Mode>, to: Mode } | Toast(String)`)
  - `state::diff(prev: &Snapshot, next: &Snapshot) -> Vec<Event>`
  - `state::Sample { t: u64, cpu_temp, gpu_temp, cpu_usage, gpu_usage, gpu_power, cpu_fan, gpu_fan: Option<f32> }` (serde camelCase), `Sample::from_reading(&SensorReading, t: u64)`
  - `state::History::new(cap: usize)`, `push(Sample)`, `samples(&self) -> Vec<Sample>`

- [ ] **Step 1: Escrever os testes**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensors::GpuReading;

    fn snap(mode: Option<Mode>) -> Snapshot {
        Snapshot { connection: Connection::Connected, mode, ..Default::default() }
    }

    #[test]
    fn unchanged_snapshot_emits_nothing() {
        let a = snap(Some(Mode::Eco));
        assert!(diff(&a, &a.clone()).is_empty());
    }

    #[test]
    fn mode_change_emits_mode_changed_then_snapshot() {
        let a = snap(Some(Mode::Balanced));
        let b = snap(Some(Mode::Turbo));
        let ev = diff(&a, &b);
        assert_eq!(ev[0], Event::ModeChanged { from: Some(Mode::Balanced), to: Mode::Turbo });
        assert!(matches!(ev[1], Event::Snapshot(_)));
    }

    #[test]
    fn first_known_mode_is_a_change() {
        let ev = diff(&snap(None), &snap(Some(Mode::Quiet)));
        assert_eq!(ev[0], Event::ModeChanged { from: None, to: Mode::Quiet });
    }

    #[test]
    fn losing_mode_is_not_a_mode_change() {
        let ev = diff(&snap(Some(Mode::Quiet)), &snap(None));
        assert_eq!(ev.len(), 1);
        assert!(matches!(ev[0], Event::Snapshot(_)));
    }

    #[test]
    fn sensor_change_emits_snapshot_only() {
        let a = snap(Some(Mode::Eco));
        let mut b = a.clone();
        b.sensors.cpu_temp = Some(70.0);
        let ev = diff(&a, &b);
        assert_eq!(ev.len(), 1);
    }

    #[test]
    fn history_is_bounded() {
        let mut h = History::new(3);
        for t in 0..5 {
            h.push(Sample { t, ..Default::default() });
        }
        let s = h.samples();
        assert_eq!(s.iter().map(|x| x.t).collect::<Vec<_>>(), vec![2, 3, 4]);
    }

    #[test]
    fn sample_from_reading() {
        let r = SensorReading {
            cpu_temp: Some(60.0),
            cpu_fan_rpm: Some(4000),
            gpu: GpuStatus::Active(GpuReading { usage: Some(30.0), power_w: Some(50.0), ..Default::default() }),
            ..Default::default()
        };
        let s = Sample::from_reading(&r, 9);
        assert_eq!(s.t, 9);
        assert_eq!(s.cpu_fan, Some(4000.0));
        assert_eq!(s.gpu_usage, Some(30.0));
        assert_eq!(s.gpu_power, Some(50.0));
    }

    #[test]
    fn snapshot_serializes_for_js() {
        let mut s = snap(Some(Mode::Turbo));
        s.sensors.gpu = GpuStatus::Sleeping;
        let j = serde_json::to_value(&s).unwrap();
        assert_eq!(j["mode"], "turbo");
        assert_eq!(j["connection"], "connected");
        assert_eq!(j["sensors"]["gpu"]["state"], "sleeping");
        assert!(j["sensors"].get("cpuFanRpm").is_some());
    }
}
```

Run: `cargo test state::`
Expected: FAIL.

- [ ] **Step 2: Implementar `state.rs`**

```rust
use std::collections::VecDeque;

use serde::Serialize;

use crate::asense::protocol::PlatformState;
use crate::mode::{FanMode, Mode};
use crate::sensors::{GpuStatus, SensorReading};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Connection {
    Connected,
    #[default]
    Disconnected,
    Busy,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub connection: Connection,
    pub mode: Option<Mode>,
    pub fan_mode: Option<FanMode>,
    pub sensors: SensorReading,
    pub platform: Option<PlatformState>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Snapshot(Snapshot),
    ModeChanged { from: Option<Mode>, to: Mode },
    Toast(String),
}

pub fn diff(prev: &Snapshot, next: &Snapshot) -> Vec<Event> {
    let mut ev = Vec::new();
    if let Some(to) = next.mode {
        if prev.mode != Some(to) {
            ev.push(Event::ModeChanged { from: prev.mode, to });
        }
    }
    if prev != next {
        ev.push(Event::Snapshot(next.clone()));
    }
    ev
}

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sample {
    pub t: u64,
    pub cpu_temp: Option<f32>,
    pub gpu_temp: Option<f32>,
    pub cpu_usage: Option<f32>,
    pub gpu_usage: Option<f32>,
    pub gpu_power: Option<f32>,
    pub cpu_fan: Option<f32>,
    pub gpu_fan: Option<f32>,
}

impl Sample {
    pub fn from_reading(r: &SensorReading, t: u64) -> Sample {
        let (gpu_usage, gpu_power) = match &r.gpu {
            GpuStatus::Active(g) => (g.usage, g.power_w),
            _ => (None, None),
        };
        Sample {
            t,
            cpu_temp: r.cpu_temp,
            gpu_temp: r.gpu_temp,
            cpu_usage: r.cpu_usage,
            gpu_usage,
            gpu_power,
            cpu_fan: r.cpu_fan_rpm.map(|v| v as f32),
            gpu_fan: r.gpu_fan_rpm.map(|v| v as f32),
        }
    }
}

pub struct History {
    cap: usize,
    buf: VecDeque<Sample>,
}

impl History {
    pub fn new(cap: usize) -> Self {
        History { cap, buf: VecDeque::with_capacity(cap) }
    }
    pub fn push(&mut self, s: Sample) {
        if self.buf.len() == self.cap {
            self.buf.pop_front();
        }
        self.buf.push_back(s);
    }
    pub fn samples(&self) -> Vec<Sample> {
        self.buf.iter().cloned().collect()
    }
}
```

- [ ] **Step 3: Rodar os testes**

Run: `cargo test state::`
Expected: 8 PASS.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src
git commit -m "feat: snapshot diffing, events and bounded history

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: Automação e hub central

**Files:**
- Create: `src-tauri/src/automation.rs`, `src-tauri/src/hub.rs`
- Modify: `src-tauri/src/lib.rs` (`pub mod automation; pub mod hub;`)

**Interfaces:**
- Consumes: tudo das Tasks 2–5.
- Produces:
  - `automation::lighting_commands(device: &str, kb: &KeyboardConfig, palette: &Palette, mode: Option<Mode>) -> Vec<String>` (vazio se `follow_mode` e `mode == None`)
  - `automation::login_commands(login: &LoginConfig) -> Vec<String>`
  - `hub::Request` (`SetProfile(Mode) | SetFan(FanChoice) | SetPlatform(String /*tail validado*/) | ReapplyLighting`)
  - `hub::Reply = std::sync::mpsc::Sender<Result<(), String>>`
  - `hub::Hub::new(asense: A, sensors: Sensors, cfg: Arc<Mutex<Config>>, history: Arc<Mutex<History>>, emit: Box<dyn FnMut(Event) + Send>) -> Hub<A>`
  - `Hub::tick(&mut self, now: Instant)`, `Hub::handle(&mut self, req: Request) -> Result<(), String>`, `Hub::snapshot(&self) -> &Snapshot`
  - `hub::run<A: Asense>(hub: Hub<A>, rx: Receiver<(Request, Reply)>, shared: Arc<Mutex<Snapshot>>)`, `hub::TICK`

- [ ] **Step 1: Testes de `automation.rs`**

```rust
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
```

Run: `cargo test automation::` → FAIL.

- [ ] **Step 2: Implementar `automation.rs`**

```rust
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
```

Run: `cargo test automation::` → 6 PASS.

- [ ] **Step 3: Testes do hub** (em `hub.rs`, com um `Asense` falso)

```rust
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
```

Run: `cargo test hub::` → FAIL.

- [ ] **Step 4: Implementar `hub.rs`**

```rust
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
```

Observação sobre `ModeChanged { from: None, .. }`: na primeira leitura a iluminação já é aplicada em `on_connected`, por isso só `from: Some(_)` recolore.

- [ ] **Step 5: Rodar os testes**

Run: `cargo test`
Expected: todos PASS (Tasks 2–6).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src
git commit -m "feat: hub loop with login defaults, mode-follow keyboard and reconnect

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: Integração Tauri (comandos, ícone na barra, janela)

**Files:**
- Create: `src-tauri/src/commands.rs`, `src-tauri/src/tray.rs`
- Modify: `src-tauri/src/lib.rs` (substituir inteiro)

**Interfaces:**
- Consumes: `hub::{Hub, Request, Reply, run}`, `asense::{Client, SOCKET_PATH}`, `sensors::Sensors`, `config`, `autostart`, `state`
- Produces (comandos invocáveis da interface, nomes exatos):
  - `get_snapshot() -> Snapshot`, `get_history() -> Vec<Sample>`, `get_config() -> Config`
  - `save_config(config: Config) -> Result<(), String>`
  - `set_profile(mode: Mode)`, `set_fan(fan: FanChoice)`, `set_platform(key: String, value: String)` → `Result<(), String>`
  - `get_autostart() -> bool`, `set_autostart(enabled: bool) -> Result<(), String>`
  - `hide_window()`
  - Eventos para a interface: `"snapshot"` (Snapshot), `"mode-changed"` (`{from, to}`), `"toast"` (string)
  - `crate::{show_window, hide_window, toggle_window, send}` (usados pelo tray)

- [ ] **Step 1: Escrever `commands.rs`**

```rust
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{AppHandle, State};

use crate::asense::protocol::platform_tail;
use crate::config::{self, Config, FanChoice};
use crate::hub::{Reply, Request};
use crate::mode::Mode;
use crate::state::{History, Sample, Snapshot};
use crate::autostart;

pub struct AppState {
    pub tx: Mutex<Sender<(Request, Reply)>>,
    pub snap: Arc<Mutex<Snapshot>>,
    pub cfg: Arc<Mutex<Config>>,
    pub history: Arc<Mutex<History>>,
    pub cfg_path: PathBuf,
    pub visible: Arc<AtomicBool>,
}

pub fn call(state: &AppState, req: Request) -> Result<(), String> {
    let (rtx, rrx) = mpsc::channel();
    state.tx.lock().unwrap().send((req, rtx)).map_err(|_| "serviço interno parado".to_string())?;
    rrx.recv_timeout(Duration::from_secs(10)).map_err(|_| "sem resposta do serviço interno".to_string())?
}

#[tauri::command]
pub fn get_snapshot(state: State<'_, AppState>) -> Snapshot {
    state.snap.lock().unwrap().clone()
}

#[tauri::command]
pub fn get_history(state: State<'_, AppState>) -> Vec<Sample> {
    state.history.lock().unwrap().samples()
}

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> Config {
    state.cfg.lock().unwrap().clone()
}

#[tauri::command(async)]
pub fn save_config(state: State<'_, AppState>, config: Config) -> Result<(), String> {
    let mut c = config;
    c.sanitize();
    config::save(&state.cfg_path, &c).map_err(|e| format!("não consegui salvar a config: {e}"))?;
    *state.cfg.lock().unwrap() = c;
    call(&state, Request::ReapplyLighting)
}

#[tauri::command(async)]
pub fn set_profile(state: State<'_, AppState>, mode: Mode) -> Result<(), String> {
    call(&state, Request::SetProfile(mode))
}

#[tauri::command(async)]
pub fn set_fan(state: State<'_, AppState>, fan: FanChoice) -> Result<(), String> {
    call(&state, Request::SetFan(fan))
}

#[tauri::command(async)]
pub fn set_platform(state: State<'_, AppState>, key: String, value: String) -> Result<(), String> {
    let tail = platform_tail(&key, &value).ok_or_else(|| format!("opção inválida: {key}={value}"))?;
    call(&state, Request::SetPlatform(tail))
}

#[tauri::command]
pub fn get_autostart() -> bool {
    autostart::enabled(&autostart::autostart_dir())
}

#[tauri::command]
pub fn set_autostart(enabled: bool) -> Result<(), String> {
    autostart::set(&autostart::autostart_dir(), enabled).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hide_window(app: AppHandle) {
    crate::hide_window(&app);
}
```

- [ ] **Step 2: Escrever `tray.rs`**

```rust
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
```

- [ ] **Step 3: Substituir `lib.rs`**

```rust
pub mod asense;
pub mod automation;
pub mod autostart;
pub mod commands;
pub mod config;
pub mod hub;
pub mod mode;
pub mod sensors;
pub mod state;
pub mod tray;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, RunEvent, WindowEvent};

use asense::{Client, SOCKET_PATH};
use commands::AppState;
use hub::{Hub, Request};
use sensors::Sensors;
use state::{Event, History, Snapshot};

fn set_visible(app: &AppHandle, v: bool) {
    if let Some(s) = app.try_state::<AppState>() {
        s.visible.store(v, Ordering::Relaxed);
    }
}

pub fn show_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
        set_visible(app, true);
        if let Some(s) = app.try_state::<AppState>() {
            let _ = app.emit("snapshot", s.snap.lock().unwrap().clone());
        }
    }
}

pub fn hide_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
    set_visible(app, false);
}

pub fn toggle_window(app: &AppHandle) {
    let visible = app.get_webview_window("main").and_then(|w| w.is_visible().ok()).unwrap_or(false);
    if visible {
        hide_window(app)
    } else {
        show_window(app)
    }
}

/// Envia uma requisição ao hub sem esperar a resposta; erros viram toast.
pub fn send(app: &AppHandle, req: Request) {
    let app = app.clone();
    std::thread::spawn(move || {
        if let Some(s) = app.try_state::<AppState>() {
            if let Err(e) = commands::call(&s, req) {
                let _ = app.emit("toast", e);
            }
        }
    });
}

pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let start_hidden = std::env::args().any(|a| a == "--hidden");

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if argv.iter().any(|a| a == "--toggle") {
                toggle_window(app)
            } else {
                show_window(app)
            }
        }))
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::get_history,
            commands::get_config,
            commands::save_config,
            commands::set_profile,
            commands::set_fan,
            commands::set_platform,
            commands::get_autostart,
            commands::set_autostart,
            commands::hide_window,
        ])
        .setup(move |app| {
            let cfg_path = config::config_path();
            let (cfg, warn) = config::load(&cfg_path);
            if let Some(w) = warn {
                log::warn!("{w}");
            }
            let cfg = Arc::new(Mutex::new(cfg));
            let shared = Arc::new(Mutex::new(Snapshot::default()));
            let history = Arc::new(Mutex::new(History::new(300)));
            let visible = Arc::new(AtomicBool::new(!start_hidden));

            let tray = tray::build(app.handle())?;
            let handle = app.handle().clone();
            let vis = visible.clone();
            let emit = Box::new(move |ev: Event| match ev {
                Event::Snapshot(s) => {
                    tray.update(&s);
                    if vis.load(Ordering::Relaxed) {
                        let _ = handle.emit("snapshot", s);
                    }
                }
                Event::ModeChanged { from, to } => {
                    let _ = handle.emit("mode-changed", serde_json::json!({ "from": from, "to": to }));
                }
                Event::Toast(m) => {
                    let _ = handle.emit("toast", m);
                }
            });

            let (tx, rx) = mpsc::channel();
            let hub = Hub::new(
                Client::new(SOCKET_PATH.into(), Duration::from_secs(2)),
                Sensors::system(),
                cfg.clone(),
                history.clone(),
                emit,
            );
            let shared2 = shared.clone();
            std::thread::Builder::new().name("hub".into()).spawn(move || hub::run(hub, rx, shared2))?;

            app.manage(AppState { tx: Mutex::new(tx), snap: shared, cfg, history, cfg_path, visible });
            if !start_hidden {
                show_window(app.handle());
            }
            Ok(())
        })
        .on_window_event(|w, e| {
            if let WindowEvent::CloseRequested { api, .. } = e {
                api.prevent_close();
                hide_window(w.app_handle());
            }
        })
        .build(tauri::generate_context!())
        .expect("erro ao iniciar o Nitro Control");

    app.run(|_app, e| {
        if let RunEvent::ExitRequested { api, code, .. } = e {
            if code.is_none() {
                api.prevent_exit();
            }
        }
    });
}
```

- [ ] **Step 4: Compilar e testar**

Run: `cd ~/nitro-control/src-tauri && cargo build && cargo test`
Expected: compila sem erros; todos os testes PASS. Se o compilador apontar diferença de assinatura na API do Tauri (por exemplo `TrayIconBuilder::title` ou `CheckMenuItem::set_checked`), consulte a documentação do Tauri 2 instalada (`cargo doc -p tauri --open`) e ajuste só a chamada, mantendo o comportamento.

- [ ] **Step 5: Teste manual rápido**

Feche a interface do ASense, se estiver aberta (ela segura a conexão). Run: `RUST_LOG=debug ./target/debug/nitro-control`
Expected: aparece o ícone na barra com `57° · 43°` (ou "—" para a GPU dormindo); o menu lista os 5 modos com o atual marcado; clicar em "Equilibrado" muda o modo (`cat /sys/firmware/acpi/platform_profile` → `balanced`) e o teclado fica laranja; clicar em "Turbo" volta ao roxo. Rodar `./target/debug/nitro-control --toggle` num segundo terminal mostra/esconde a janela. Encerre pelo menu "Sair".

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src
git commit -m "feat: Tauri wiring — commands, tray with temps, hide-on-close, single instance

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 8: Base da interface (tema, abas, API, demo) e aba Início

**Files:**
- Create: `ui/index.html` (substitui), `ui/css/theme.css`, `ui/css/anim.css`, `ui/js/logic.js`, `ui/js/api.js`, `ui/js/demo.js`, `ui/js/widgets.js`, `ui/js/app.js`, `ui/js/tabs/home.js`, `tests-ui/logic.test.mjs`

**Interfaces:**
- Consumes: comandos e eventos da Task 7 (nomes exatos).
- Produces (usados pelas Tasks 9–12):
  - `logic.js`: `MODES` (`[{id,label}]`), `FAN_MAX_RPM = 7050`, `modeLabel(id)`, `fmtTemp(v)`, `fmtRpm(v)`, `fmtPct(v)`, `fanSpinSeconds(rpm)`, `pct(v, max)`, `hexToRgb(h)`, `mix(a, b, t)`, `themeVars({primary, secondary})`, `isHex(s)`, `sparkPath(values, w, h, min, max)`
  - `api.js`: `api` com `getSnapshot, getHistory, getConfig, saveConfig(config), setProfile(mode), setFan(fan), setPlatform(key, value), getAutostart, setAutostart(enabled), hideWindow, minimize, on(event, cb)`; `isDemo`
  - `widgets.js`: `ring(label, big=false) -> HTMLElement`, `setRing(el, value, max, text, top = '')`, `fan(label) -> HTMLElement`, `setFan(el, rpm)`, `chart(title, series:[{key,label,color}]) -> HTMLElement`, `setChart(el, samples, min, max)`, `chips(items:[{id,label}], onPick) -> HTMLElement`, `setChips(el, activeId)`, `panel(title) -> HTMLElement` (retorna o painel; conteúdo vai em `.body`)
  - Cada aba exporta `mount(el, ctx)` e `update(ctx)`. `ctx = { api, store, run, toast, saveConfig }`, `store = { snap, config }`.

- [ ] **Step 1: Testes da lógica pura (`tests-ui/logic.test.mjs`)**

```js
import test from 'node:test';
import assert from 'node:assert/strict';
import { modeLabel, fmtTemp, fmtRpm, fmtPct, fanSpinSeconds, pct, hexToRgb, mix, themeVars, isHex, sparkPath, FAN_MAX_RPM } from '../ui/js/logic.js';

test('rótulos dos modos', () => {
  assert.equal(modeLabel('turbo'), 'Turbo');
  assert.equal(modeLabel('quiet'), 'Silencioso');
  assert.equal(modeLabel(null), '—');
});

test('formatação', () => {
  assert.equal(fmtTemp(57.4), '57°');
  assert.equal(fmtTemp(null), '—');
  assert.equal(fmtRpm(5674), '5674 RPM');
  assert.equal(fmtRpm(undefined), '—');
  assert.equal(fmtPct(12.6), '13%');
});

test('velocidade da hélice cresce com o RPM', () => {
  assert.equal(fanSpinSeconds(0), 0);
  assert.equal(fanSpinSeconds(null), 0);
  assert.ok(fanSpinSeconds(7000) < fanSpinSeconds(4000));
  assert.ok(fanSpinSeconds(100) <= 3);
  assert.ok(fanSpinSeconds(99999) >= 0.25);
});

test('pct é limitado a 0..100', () => {
  assert.equal(pct(FAN_MAX_RPM * 2, FAN_MAX_RPM), 100);
  assert.equal(pct(-5, 100), 0);
  assert.equal(pct(null, 100), 0);
  assert.equal(pct(50, 100), 50);
});

test('cores', () => {
  assert.deepEqual(hexToRgb('#ff8a1f'), [255, 138, 31]);
  assert.equal(mix('#ffffff', '#000000', 0.5), '#808080');
  assert.ok(isHex('#b026ff'));
  assert.ok(!isHex('b026ff'));
  const v = themeVars({ primary: '#b026ff', secondary: '#ff3df2' });
  assert.equal(v['--r'], '#b026ff');
  assert.equal(v['--r2'], '#ff3df2');
  assert.match(v['--glow'], /^rgba\(176,38,255,/);
});

test('sparkPath ignora buracos', () => {
  assert.equal(sparkPath([], 100, 10, 0, 10), '');
  assert.equal(sparkPath([0, 10], 100, 10, 0, 10), 'M0.0,10.0 L100.0,0.0');
  assert.equal(sparkPath([0, null, 10], 100, 10, 0, 10), 'M0.0,10.0 M100.0,0.0');
});
```

Run: `cd ~/nitro-control && node --test tests-ui/`
Expected: FAIL (módulo não existe).

- [ ] **Step 2: `ui/js/logic.js`**

```js
export const MODES = [
  { id: 'eco', label: 'Eco' },
  { id: 'quiet', label: 'Silencioso' },
  { id: 'balanced', label: 'Equilibrado' },
  { id: 'performance', label: 'Desempenho' },
  { id: 'turbo', label: 'Turbo' },
];
export const FAN_MAX_RPM = 7050;

export const modeLabel = (id) => MODES.find((m) => m.id === id)?.label ?? '—';
export const fmtTemp = (v) => (v == null ? '—' : `${Math.round(v)}°`);
export const fmtRpm = (v) => (v == null ? '—' : `${Math.round(v)} RPM`);
export const fmtPct = (v) => (v == null ? '—' : `${Math.round(v)}%`);

/** Segundos por volta da hélice animada: mais RPM, volta mais curta. */
export function fanSpinSeconds(rpm) {
  if (!rpm || rpm <= 0) return 0;
  return Math.max(0.25, Math.min(3, 2400 / rpm));
}

export function pct(v, max) {
  if (v == null || !max) return 0;
  return Math.max(0, Math.min(100, (v / max) * 100));
}

export function hexToRgb(h) {
  const n = parseInt(h.slice(1), 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

export function mix(a, b, t) {
  const A = hexToRgb(a);
  const B = hexToRgb(b);
  return '#' + A.map((x, i) => Math.round(x + (B[i] - x) * t).toString(16).padStart(2, '0')).join('');
}

export const isHex = (s) => /^#[0-9a-fA-F]{6}$/.test(s ?? '');

export function themeVars({ primary, secondary }) {
  const [r, g, b] = hexToRgb(primary);
  return {
    '--r': primary,
    '--r2': secondary,
    '--glow': `rgba(${r},${g},${b},.55)`,
    '--bg1': mix(primary, '#000000', 0.78),
    '--ln': mix(primary, '#000000', 0.62),
  };
}

/** Caminho SVG de uma série; `null` quebra a linha (novo "M"). */
export function sparkPath(values, w, h, min, max) {
  if (!values.length) return '';
  const step = values.length > 1 ? w / (values.length - 1) : 0;
  const span = max - min || 1;
  let d = '';
  let pen = false;
  values.forEach((v, i) => {
    if (v == null) {
      pen = false;
      return;
    }
    const x = (i * step).toFixed(1);
    const y = (h - ((Math.max(min, Math.min(max, v)) - min) / span) * h).toFixed(1);
    d += `${d ? ' ' : ''}${pen ? 'L' : 'M'}${x},${y}`;
    pen = true;
  });
  return d;
}
```

Run: `node --test tests-ui/` → 6 PASS.

- [ ] **Step 3: `ui/js/api.js` e `ui/js/demo.js`**

`api.js`:
```js
const T = window.__TAURI__;
export const isDemo = !T || new URLSearchParams(location.search).has('demo');

function tauriApi() {
  const inv = (cmd, args) => T.core.invoke(cmd, args);
  return {
    getSnapshot: () => inv('get_snapshot'),
    getHistory: () => inv('get_history'),
    getConfig: () => inv('get_config'),
    saveConfig: (config) => inv('save_config', { config }),
    setProfile: (mode) => inv('set_profile', { mode }),
    setFan: (fan) => inv('set_fan', { fan }),
    setPlatform: (key, value) => inv('set_platform', { key, value }),
    getAutostart: () => inv('get_autostart'),
    setAutostart: (enabled) => inv('set_autostart', { enabled }),
    hideWindow: () => inv('hide_window'),
    minimize: () => T.window.getCurrentWindow().minimize(),
    on: (name, cb) => T.event.listen(name, (e) => cb(e.payload)),
  };
}

export const api = isDemo ? (await import('./demo.js')).demoApi() : tauriApi();
```

`demo.js` (dados falsos para conferir o visual no navegador):
```js
const listeners = {};
const emit = (n, p) => (listeners[n] || []).forEach((cb) => cb(p));
const clone = (o) => JSON.parse(JSON.stringify(o));

export function demoApi() {
  let config = {
    palette: {
      eco: { primary: '#22c55e', secondary: '#86efac' },
      quiet: { primary: '#38bdf8', secondary: '#a5e3ff' },
      balanced: { primary: '#ff8a1f', secondary: '#ffc07a' },
      performance: { primary: '#ff2a1a', secondary: '#ff6a2b' },
      turbo: { primary: '#b026ff', secondary: '#ff3df2' },
    },
    keyboard: { followMode: true, effect: 'static', brightness: 100, speed: 5, zones: ['#ff2a1a', '#ff2a1a', '#ff8a1f', '#ff8a1f'] },
    login: { apply: true, mode: 'turbo', fan: 'auto' },
  };
  const target = { eco: 2600, quiet: 3000, balanced: 3800, performance: 4800, turbo: 5600 };
  const snap = {
    connection: 'connected', mode: 'balanced', fanMode: 'auto',
    sensors: { cpuTemp: 58, gpuTemp: 46, sysTemp: 44, ssdTemp: 36, cpuFanRpm: 3800, gpuFanRpm: 3700, cpuUsage: 18, ramUsedGb: 7.9, ramTotalGb: 14.6,
      gpu: { state: 'active', temp: 46, usage: 22, clockMhz: 1410, powerW: 38 } },
    platform: { batteryLimit: false, batteryCalibration: false, usbCharging: 30, keyboardTimeout: false, bootSound: true, lcdOverride: false },
  };
  const history = [];
  let autostart = true;
  setInterval(() => {
    const s = snap.sensors;
    const goal = snap.fanMode === 'maximum' ? 7040 : target[snap.mode];
    s.cpuFanRpm += (goal - s.cpuFanRpm) * 0.25;
    s.gpuFanRpm += (goal - 80 - s.gpuFanRpm) * 0.25;
    s.cpuTemp = 50 + Math.random() * 15;
    s.gpuTemp = s.gpu.temp = 40 + Math.random() * 12;
    s.cpuUsage = Math.random() * 40;
    s.gpu.usage = Math.random() * 60;
    s.gpu.powerW = 20 + Math.random() * 60;
    history.push({ t: Date.now() / 1000, cpuTemp: s.cpuTemp, gpuTemp: s.gpuTemp, cpuUsage: s.cpuUsage, gpuUsage: s.gpu.usage, gpuPower: s.gpu.powerW, cpuFan: s.cpuFanRpm, gpuFan: s.gpuFanRpm });
    if (history.length > 300) history.shift();
    emit('snapshot', clone(snap));
  }, 1000);
  const ok = () => new Promise((r) => setTimeout(r, 150));
  return {
    getSnapshot: async () => clone(snap),
    getHistory: async () => clone(history),
    getConfig: async () => clone(config),
    saveConfig: async (c) => { config = clone(c); await ok(); },
    setProfile: async (mode) => { await ok(); const from = snap.mode; snap.mode = mode; emit('mode-changed', { from, to: mode }); emit('snapshot', clone(snap)); },
    setFan: async (fan) => { await ok(); snap.fanMode = fan; emit('snapshot', clone(snap)); },
    setPlatform: async (key, value) => {
      await ok();
      const map = { BATTERY_LIMIT: 'batteryLimit', KEYBOARD_TIMEOUT: 'keyboardTimeout', BOOT_SOUND: 'bootSound', LCD_OVERRIDE: 'lcdOverride' };
      if (map[key]) snap.platform[map[key]] = value === 'ON';
      if (key === 'USB_CHARGING') snap.platform.usbCharging = Number(value);
      if (key === 'BATTERY_CALIBRATION') snap.platform.batteryCalibration = value === 'START';
      emit('snapshot', clone(snap));
    },
    getAutostart: async () => autostart,
    setAutostart: async (v) => { autostart = v; },
    hideWindow: async () => emit('toast', 'Demo: a janela seria escondida'),
    minimize: async () => {},
    on: (n, cb) => { (listeners[n] ||= []).push(cb); },
  };
}
```

- [ ] **Step 4: `ui/js/widgets.js`**

```js
import { fanSpinSeconds, fmtRpm, pct, sparkPath, FAN_MAX_RPM } from './logic.js';

const h = (html) => {
  const t = document.createElement('template');
  t.innerHTML = html.trim();
  return t.content.firstElementChild;
};

export function panel(title) {
  return h(`<div class="pn"><div class="ttl">${title}</div><div class="body"></div></div>`);
}

export function ring(label, big = false) {
  return h(`<div class="ring ${big ? 'big' : 'sm'}"><div><span class="u top"></span><span class="val">—</span><span class="u lbl">${label}</span></div></div>`);
}

export function setRing(el, value, max, text, top = '') {
  el.style.setProperty('--p', `${pct(value, max) * 0.75}%`);
  el.querySelector('.val').textContent = text;
  el.querySelector('.top').textContent = top;
}

const BLADE = 'M50 50 C44 30 50 12 62 10 C60 26 58 40 50 50Z';
export function fan(label) {
  const blades = [0, 72, 144, 216, 288].map((a) => `<path d="${BLADE}" transform="rotate(${a} 50 50)"/>`).join('');
  return h(`<div class="fan">
    <svg viewBox="0 0 100 100"><circle cx="50" cy="50" r="46" class="fan-ring"/>
      <g class="rot"><g class="blades">${blades}</g><circle cx="50" cy="50" r="9" class="hub"/></g></svg>
    <div class="ttl">${label}</div><div class="rpm">—</div><div class="bar"><div></div></div></div>`);
}

export function setFan(el, rpm) {
  const s = fanSpinSeconds(rpm);
  const rot = el.querySelector('.rot');
  rot.style.animationDuration = s ? `${s}s` : '0s';
  rot.style.animationPlayState = s ? 'running' : 'paused';
  el.querySelector('.rpm').textContent = fmtRpm(rpm);
  el.querySelector('.bar > div').style.width = `${pct(rpm, FAN_MAX_RPM)}%`;
}

export function chips(items, onPick) {
  const wrap = h('<div class="chips"></div>');
  for (const it of items) {
    const b = h(`<button class="chip" data-id="${it.id}">${it.label}</button>`);
    b.addEventListener('click', () => onPick(it.id));
    wrap.appendChild(b);
  }
  return wrap;
}

export function setChips(el, activeId) {
  el.querySelectorAll('.chip').forEach((b) => b.classList.toggle('on', b.dataset.id === activeId));
}

export function chart(title, series) {
  const legend = series.map((s) => `<span style="color:${s.color}">■ ${s.label}</span>`).join(' ');
  const paths = series.map((s) => `<path data-key="${s.key}" stroke="${s.color}"/>`).join('');
  return h(`<div class="pn chart"><div class="ttl">${title} <span class="legend">${legend}</span></div>
    <svg viewBox="0 0 300 80" preserveAspectRatio="none">${paths}</svg><div class="axis"><span class="max"></span><span class="min"></span></div></div>`);
}

export function setChart(el, samples, min, max, unit = '') {
  el.querySelectorAll('path').forEach((p) => {
    p.setAttribute('d', sparkPath(samples.map((s) => s[p.dataset.key] ?? null), 300, 80, min, max));
  });
  el.querySelector('.max').textContent = `${max}${unit}`;
  el.querySelector('.min').textContent = `${min}${unit}`;
}
```

- [ ] **Step 5: `ui/index.html`**

```html
<!doctype html>
<html lang="pt-BR">
<head>
  <meta charset="utf-8">
  <title>Nitro Control</title>
  <link rel="stylesheet" href="css/theme.css">
  <link rel="stylesheet" href="css/anim.css">
</head>
<body>
  <header class="top" data-tauri-drag-region>
    <div class="logo" data-tauri-drag-region></div>
    <nav id="tabs">
      <button data-tab="home" class="on">Início</button>
      <button data-tab="perf">Desempenho</button>
      <button data-tab="keyboard">Teclado</button>
      <button data-tab="monitor">Monitor</button>
      <button data-tab="system">Sistema</button>
    </nav>
    <div class="sp" data-tauri-drag-region></div>
    <div id="conn" class="conn"></div>
    <button class="win" id="btn-min" title="Minimizar">—</button>
    <button class="win" id="btn-close" title="Fechar (continua na barra)">✕</button>
  </header>
  <main>
    <section id="tab-home" class="tab on"></section>
    <section id="tab-perf" class="tab"></section>
    <section id="tab-keyboard" class="tab"></section>
    <section id="tab-monitor" class="tab"></section>
    <section id="tab-system" class="tab"></section>
  </main>
  <div id="flash" class="flash"></div>
  <div id="banner" class="banner"></div>
  <div id="toasts"></div>
  <script type="module" src="js/app.js"></script>
</body>
</html>
```

- [ ] **Step 6: `ui/css/theme.css`**

```css
:root {
  --r: #ff8a1f; --r2: #ffc07a; --glow: rgba(255,138,31,.55); --bg1: #381e07; --ln: #613410;
  --mut: #b5a5a0; --txt: #f3eeec; --panel: rgba(0,0,0,.35);
  --t: .9s;
}
* { box-sizing: border-box; }
html, body { margin: 0; height: 100%; }
body {
  background: radial-gradient(ellipse at 50% 40%, var(--bg1) 0%, #0c0605 60%, #050303 100%);
  color: var(--txt); font-family: "Rajdhani", "Oxanium", "Segoe UI", system-ui, sans-serif;
  overflow: hidden; user-select: none; transition: background var(--t);
}
button { font: inherit; color: inherit; background: none; border: 0; cursor: pointer; }
.top { display: flex; align-items: center; gap: 18px; height: 44px; padding: 0 12px; background: rgba(0,0,0,.55); border-bottom: 1px solid var(--ln); transition: border-color var(--t); }
.logo { width: 18px; height: 18px; border: 2px solid var(--r); transform: skewX(-15deg); box-shadow: 0 0 10px var(--glow); }
#tabs button { color: var(--mut); padding: 10px 2px; font-size: 14px; letter-spacing: .04em; border-bottom: 2px solid transparent; }
#tabs button.on { color: #fff; border-bottom-color: var(--r); font-weight: 700; }
.sp { flex: 1; align-self: stretch; }
.win { width: 34px; height: 30px; color: var(--mut); }
.win:hover { color: #fff; background: rgba(255,255,255,.06); }
.conn { font-size: 12px; color: var(--mut); }
.conn.bad { color: #ffb4a8; }
main { height: calc(100% - 44px); overflow: auto; padding: 14px; }
.tab { display: none; }
.tab.on { display: block; }

.pn {
  background: linear-gradient(180deg, rgba(255,255,255,.04), var(--panel)); border: 1px solid var(--ln); padding: 12px; position: relative;
  clip-path: polygon(0 10px,10px 0,calc(100% - 10px) 0,100% 10px,100% calc(100% - 10px),calc(100% - 10px) 100%,10px 100%,0 calc(100% - 10px));
  transition: border-color var(--t);
}
.pn::before { content: ""; position: absolute; left: 10px; right: 10px; top: 0; height: 2px; background: linear-gradient(90deg, transparent, var(--r), transparent); }
.ttl { font-size: 11px; letter-spacing: .14em; color: var(--mut); text-transform: uppercase; margin-bottom: 8px; }
.u { font-size: 11px; color: var(--mut); }
.kv { display: flex; justify-content: space-between; padding: 5px 0; border-bottom: 1px solid rgba(255,255,255,.06); font-size: 13px; }
.kv b { color: var(--r2); }

.ring { border-radius: 50%; margin: auto; display: flex; align-items: center; justify-content: center;
  background: conic-gradient(from 225deg, var(--r) 0 var(--p, 0%), rgba(255,255,255,.07) var(--p, 0%) 75%, transparent 75%);
  box-shadow: 0 0 18px var(--glow), inset 0 0 12px var(--glow); transition: box-shadow var(--t); }
.ring > div { border-radius: 50%; background: radial-gradient(circle, #1a0d0b, #080404); border: 1px solid var(--ln); display: flex; flex-direction: column; align-items: center; justify-content: center; }
.ring.big { width: 170px; height: 170px; } .ring.big > div { width: 140px; height: 140px; } .ring.big .val { font-size: 30px; }
.ring.sm { width: 92px; height: 92px; } .ring.sm > div { width: 72px; height: 72px; } .ring.sm .val { font-size: 20px; }
.val { font-weight: 700; line-height: 1; }

.chips { display: flex; flex-wrap: wrap; gap: 4px; }
.chip { padding: 6px 12px; border: 1px solid var(--ln); font-size: 13px; clip-path: polygon(6px 0,100% 0,calc(100% - 6px) 100%,0 100%); transition: all .3s; }
.chip:hover { border-color: var(--r); }
.chip.on { background: linear-gradient(90deg, var(--r), var(--r2)); color: #fff; font-weight: 700; border-color: var(--r); }
body.offline .chip, body.offline .needs-daemon { opacity: .4; pointer-events: none; }

.fan { text-align: center; flex: 1; }
.fan svg { width: 110px; height: 110px; filter: drop-shadow(0 0 8px var(--glow)); }
.fan-ring { fill: none; stroke: var(--ln); stroke-width: 3; }
.blades path { fill: var(--r); transition: fill var(--t); }
.hub { fill: #1c0b08; stroke: var(--r2); stroke-width: 2; }
.rpm { font-size: 22px; font-weight: 700; }
.bar { height: 5px; background: rgba(255,255,255,.07); margin-top: 6px; }
.bar > div { height: 5px; background: linear-gradient(90deg, var(--r), var(--r2)); transition: width .8s; }

.kb { display: flex; gap: 4px; }
.kb div { flex: 1; height: 16px; box-shadow: 0 0 10px currentColor; }

.chart svg { width: 100%; height: 110px; }
.chart path { fill: none; stroke-width: 1.6; vector-effect: non-scaling-stroke; }
.chart .legend { float: right; text-transform: none; letter-spacing: 0; }
.chart .axis { display: flex; justify-content: space-between; font-size: 10px; color: var(--mut); }

.home { display: grid; grid-template-columns: 1.05fr 1.3fr .8fr .95fr; gap: 12px; }
.mode-title { text-align: center; }
.mode-title .h { font-size: 26px; letter-spacing: .2em; font-weight: 800; }
.mode-title .h span { color: var(--r); }
.curmode { font-size: 26px; font-weight: 800; color: var(--r2); letter-spacing: .1em; text-shadow: 0 0 14px var(--glow); margin: 4px 0 10px; }
.grid3 { display: grid; grid-template-columns: repeat(3, 1fr); gap: 5px; }
.cell { background: rgba(0,0,0,.35); border: 1px solid var(--ln); padding: 5px; font-size: 10px; color: var(--mut); }
.cell b { display: block; color: #fff; font-size: 15px; }
.stack { display: flex; flex-direction: column; gap: 12px; }

.field { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 8px 0; border-bottom: 1px solid rgba(255,255,255,.06); }
.switch { width: 42px; height: 22px; border-radius: 11px; background: rgba(255,255,255,.12); position: relative; transition: background .2s; }
.switch::after { content: ""; position: absolute; top: 3px; left: 3px; width: 16px; height: 16px; border-radius: 50%; background: #fff; transition: left .2s; }
.switch.on { background: var(--r); } .switch.on::after { left: 23px; }
input[type=range] { accent-color: var(--r); width: 200px; }
input[type=color] { width: 44px; height: 28px; border: 1px solid var(--ln); background: none; padding: 0; }

#toasts { position: fixed; right: 16px; bottom: 16px; display: flex; flex-direction: column; gap: 8px; z-index: 20; }
.toast { background: #1d0c09; border: 1px solid var(--r); padding: 10px 14px; max-width: 380px; font-size: 13px; box-shadow: 0 0 12px var(--glow); }
```

- [ ] **Step 7: `ui/css/anim.css`**

```css
.rot { transform-origin: 50% 50%; animation: spin 1s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }

.flash { position: fixed; inset: 0; pointer-events: none; opacity: 0; z-index: 10;
  background: linear-gradient(100deg, transparent 30%, var(--glow) 48%, rgba(255,255,255,.75) 50%, var(--glow) 52%, transparent 70%); background-size: 250% 100%; }
.banner { position: fixed; left: 0; right: 0; top: 40%; text-align: center; font-size: 56px; font-weight: 900; letter-spacing: .35em;
  color: #fff; text-shadow: 0 0 18px var(--r), 0 0 40px var(--r); opacity: 0; pointer-events: none; z-index: 11; }
body.sw .flash { animation: sweep .9s ease-out; }
body.sw .banner { animation: ban 1.3s ease-out; }
body.sw .pn { animation: pulse 1.1s ease-out; }
body.sw[data-mode="turbo"] .banner { animation-duration: 1.6s; font-size: 68px; text-shadow: 0 0 22px var(--r), 0 0 50px var(--r2), 0 0 90px var(--r); }
body[data-mode="turbo"] .curmode { text-shadow: 0 0 16px var(--r), 0 0 32px var(--r2); }
@keyframes sweep { 0% { opacity: 1; background-position: 120% 0; } 100% { opacity: 0; background-position: -40% 0; } }
@keyframes ban { 0% { opacity: 0; transform: scale(1.6); } 20% { opacity: 1; transform: scale(1); } 70% { opacity: 1; } 100% { opacity: 0; } }
@keyframes pulse { 25% { filter: brightness(1.6); } 100% { filter: none; } }

@media (prefers-reduced-motion: reduce) {
  body.sw .flash, body.sw .banner, body.sw .pn { animation: none; }
  .rot { animation: none; }
}
```

- [ ] **Step 8: `ui/js/app.js`**

```js
import { api, isDemo } from './api.js';
import { themeVars, modeLabel } from './logic.js';
import * as home from './tabs/home.js';
import * as perf from './tabs/perf.js';
import * as keyboard from './tabs/keyboard.js';
import * as monitor from './tabs/monitor.js';
import * as system from './tabs/system.js';

const tabs = { home, perf, keyboard, monitor, system };
const store = { snap: null, config: null };

export function toast(msg) {
  const el = document.createElement('div');
  el.className = 'toast';
  el.textContent = msg;
  document.getElementById('toasts').appendChild(el);
  setTimeout(() => el.remove(), 5000);
}

async function run(fn) {
  try {
    await fn();
  } catch (e) {
    toast(String(e));
  }
}

function applyTheme(mode) {
  const colors = store.config?.palette?.[mode];
  if (!colors) return;
  for (const [k, v] of Object.entries(themeVars(colors))) document.documentElement.style.setProperty(k, v);
  document.body.dataset.mode = mode;
}

function playModeAnimation(mode) {
  if (matchMedia('(prefers-reduced-motion: reduce)').matches) return;
  document.getElementById('banner').textContent = modeLabel(mode).toUpperCase();
  document.body.classList.remove('sw');
  void document.body.offsetWidth;
  document.body.classList.add('sw');
}

const CONN_TEXT = { connected: '', disconnected: 'ASense desconectado', busy: 'Feche a interface do ASense' };

function render() {
  const s = store.snap;
  if (!s) return;
  document.body.classList.toggle('offline', s.connection !== 'connected');
  const conn = document.getElementById('conn');
  conn.textContent = (isDemo ? 'DEMO ' : '') + CONN_TEXT[s.connection];
  conn.classList.toggle('bad', s.connection !== 'connected');
  for (const t of Object.values(tabs)) t.update(ctx);
}

const ctx = {
  api,
  store,
  run,
  toast,
  async saveConfig(cfg) {
    await api.saveConfig(cfg);
    store.config = await api.getConfig();
    applyTheme(store.snap?.mode);
    render();
  },
};

function setupChrome() {
  document.querySelectorAll('#tabs button').forEach((b) =>
    b.addEventListener('click', () => {
      document.querySelectorAll('#tabs button').forEach((x) => x.classList.toggle('on', x === b));
      document.querySelectorAll('.tab').forEach((t) => t.classList.toggle('on', t.id === `tab-${b.dataset.tab}`));
      tabs[b.dataset.tab].shown?.(ctx);
    }),
  );
  document.getElementById('btn-close').addEventListener('click', () => run(api.hideWindow));
  document.getElementById('btn-min').addEventListener('click', () => run(api.minimize));
}

async function main() {
  setupChrome();
  store.config = await api.getConfig();
  for (const [name, t] of Object.entries(tabs)) t.mount(document.getElementById(`tab-${name}`), ctx);
  store.snap = await api.getSnapshot();
  applyTheme(store.snap.mode ?? 'balanced');
  render();
  api.on('snapshot', (s) => {
    store.snap = s;
    render();
  });
  api.on('mode-changed', ({ from, to }) => {
    applyTheme(to);
    if (from) playModeAnimation(to);
  });
  api.on('toast', toast);
}

main().catch((e) => toast(`Falha ao iniciar a interface: ${e}`));
```

- [ ] **Step 9: Aba Início (`ui/js/tabs/home.js`)** e esqueletos das outras abas

`home.js`:
```js
import { MODES, modeLabel, fmtTemp, fmtPct } from '../logic.js';
import { panel, ring, setRing, chips, setChips } from '../widgets.js';

const el = {};
const FX = { off: 'Desligado', static: 'Estático', breathing: 'Respiração', neon: 'Neon' };

export function mount(root, ctx) {
  root.innerHTML = '<div class="home"></div>';
  const g = root.firstElementChild;

  const gpu = panel('GPU');
  el.gpuRing = ring('MHz', true);
  gpu.querySelector('.body').append(el.gpuRing);
  gpu.querySelector('.body').insertAdjacentHTML('beforeend',
    '<div class="kv" style="margin-top:10px"><span>Uso GPU</span><b data-k="gpuUse">—</b></div><div class="kv"><span>Uso CPU</span><b data-k="cpuUse">—</b></div><div class="kv"><span>Energia GPU</span><b data-k="gpuW">—</b></div>');

  const center = document.createElement('div');
  center.className = 'pn mode-title';
  center.innerHTML = '<div class="h">NITRO<span>CONTROL</span></div><div class="u" style="letter-spacing:.2em">MODO DO SISTEMA</div><div class="curmode">—</div>';
  el.cur = center.querySelector('.curmode');
  el.modes = chips(MODES, (id) => ctx.run(() => ctx.api.setProfile(id)));
  el.modes.style.justifyContent = 'center';
  center.append(el.modes);
  center.insertAdjacentHTML('beforeend', '<div class="ttl" style="margin-top:14px">Ventoinha</div>');
  el.fans = chips([{ id: 'auto', label: 'Auto' }, { id: 'maximum', label: 'Máximo' }], (id) => ctx.run(() => ctx.api.setFan(id)));
  el.fans.style.justifyContent = 'center';
  center.append(el.fans);
  center.insertAdjacentHTML('beforeend', '<div class="ttl" style="margin-top:14px">Teclado</div><div class="kb"><div></div><div></div><div></div><div></div></div>');
  el.kb = [...center.querySelectorAll('.kb div')];

  const temps = panel('Temperatura');
  el.tGpu = ring('GPU');
  el.tCpu = ring('CPU');
  el.tSys = ring('Sistema');
  temps.querySelector('.body').append(el.tGpu, el.tCpu, el.tSys);
  temps.querySelector('.body').style.cssText = 'display:flex;flex-direction:column;gap:10px';

  const right = document.createElement('div');
  right.className = 'stack';
  const prof = panel('Perfil ativo');
  prof.querySelector('.body').innerHTML = '<div class="kv"><span>Modo</span><b data-k="mode">—</b></div><div class="kv"><span>Ventoinha</span><b data-k="fan">—</b></div><div class="kv"><span>Efeito</span><b data-k="fx">—</b></div><div class="kv"><span>Bateria</span><b data-k="bat">—</b></div>';
  const mon = panel('Monitor');
  mon.querySelector('.body').innerHTML = '<div class="grid3"><div class="cell">GPU<b data-k="c1">—</b></div><div class="cell">GPU<b data-k="c2">—</b></div><div class="cell">CPU<b data-k="c3">—</b></div><div class="cell">CPU<b data-k="c4">—</b></div><div class="cell">RAM<b data-k="c5">—</b></div><div class="cell">SSD<b data-k="c6">—</b></div></div>';
  right.append(prof, mon);

  g.append(gpu, center, temps, right);
  el.root = root;
}

const put = (k, v) => {
  const n = el.root.querySelector(`[data-k="${k}"]`);
  if (n) n.textContent = v;
};

export function update({ store }) {
  const s = store.snap;
  const cfg = store.config;
  const sn = s.sensors;
  const g = sn.gpu?.state === 'active' ? sn.gpu : null;
  setRing(el.gpuRing, g?.clockMhz, 2500, g ? `${Math.round(g.clockMhz ?? 0)}` : sn.gpu?.state === 'sleeping' ? 'zZ' : '—', g ? 'frequência' : sn.gpu?.state === 'sleeping' ? 'em repouso' : '');
  put('gpuUse', fmtPct(g?.usage));
  put('cpuUse', fmtPct(sn.cpuUsage));
  put('gpuW', g?.powerW != null ? `${Math.round(g.powerW)} W` : '—');
  el.cur.textContent = modeLabel(s.mode);
  setChips(el.modes, s.mode);
  setChips(el.fans, s.fanMode);
  setRing(el.tGpu, sn.gpuTemp, 100, sn.gpu?.state === 'sleeping' ? 'zZ' : fmtTemp(sn.gpuTemp));
  setRing(el.tCpu, sn.cpuTemp, 100, fmtTemp(sn.cpuTemp));
  setRing(el.tSys, sn.sysTemp, 100, fmtTemp(sn.sysTemp));
  const kbColors = cfg.keyboard.effect === 'off'
    ? ['#111', '#111', '#111', '#111']
    : cfg.keyboard.followMode && s.mode ? Array(4).fill(cfg.palette[s.mode].primary) : cfg.keyboard.zones;
  el.kb.forEach((d, i) => { d.style.background = kbColors[i]; d.style.color = kbColors[i]; });
  put('mode', modeLabel(s.mode));
  put('fan', s.fanMode === 'maximum' ? 'Máximo' : s.fanMode === 'auto' ? 'Auto' : '—');
  put('fx', FX[cfg.keyboard.effect]);
  put('bat', s.platform?.batteryLimit ? 'Limite 80%' : 'Carga total');
  put('c1', fmtPct(g?.usage));
  put('c2', fmtTemp(sn.gpuTemp));
  put('c3', fmtPct(sn.cpuUsage));
  put('c4', fmtTemp(sn.cpuTemp));
  put('c5', sn.ramUsedGb != null ? `${sn.ramUsedGb.toFixed(1)}G` : '—');
  put('c6', fmtTemp(sn.ssdTemp));
}
```

Crie `perf.js`, `keyboard.js`, `monitor.js` e `system.js` temporários com:
```js
export function mount(root) { root.innerHTML = '<div class="pn"><div class="ttl">Em construção</div></div>'; }
export function update() {}
```

- [ ] **Step 10: Conferir no modo demonstração**

Run: `cd ~/nitro-control/ui && python3 -m http.server 8765` (em segundo plano) e abra `http://localhost:8765/?demo=1` no navegador (ou use o visual companion/Playwright para tirar um screenshot).
Expected: aba Início parecida com o mockup aprovado; temperaturas mudando a cada segundo; clicar em "Turbo" dispara feixe + "TURBO" grande + tema roxo; "Máximo" marca o chip.

Run: `node --test tests-ui/` → PASS.

- [ ] **Step 11: Commit**

```bash
git add ui tests-ui
git commit -m "feat(ui): theme, tabs shell, demo API and home tab with mode animation

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 9: Aba Desempenho

**Files:**
- Modify: `ui/js/tabs/perf.js` (substituir o esqueleto)
- Modify: `ui/css/theme.css` (adicionar ao final)

**Interfaces:**
- Consumes: `widgets.{panel, fan, setFan, chips, setChips}`, `logic.{MODES}`, `ctx.api.{setProfile, setFan}`

- [ ] **Step 1: Implementar `perf.js`**

```js
import { MODES } from '../logic.js';
import { panel, fan, setFan, chips, setChips } from '../widgets.js';

const el = {};
const DESC = {
  eco: 'Menor consumo, ideal na bateria',
  quiet: 'Ventoinhas baixas, uso leve',
  balanced: 'Equilíbrio entre ruído e desempenho',
  performance: 'Mais potência para jogos',
  turbo: 'Potência máxima da CPU e GPU',
};

export function mount(root, ctx) {
  root.innerHTML = '<div class="perf"></div>';
  const wrap = root.firstElementChild;
  const cards = document.createElement('div');
  cards.className = 'mode-cards';
  el.cards = {};
  for (const m of MODES) {
    const c = document.createElement('button');
    c.className = 'pn mode-card';
    c.dataset.mode = m.id;
    c.innerHTML = `<div class="swatch"></div><div class="name">${m.label}</div><div class="u">${DESC[m.id]}</div>`;
    c.addEventListener('click', () => ctx.run(() => ctx.api.setProfile(m.id)));
    cards.append(c);
    el.cards[m.id] = c;
  }
  const fans = panel('Ventoinhas');
  const row = document.createElement('div');
  row.style.cssText = 'display:flex;gap:16px';
  el.cpu = fan('CPU');
  el.gpu = fan('GPU');
  row.append(el.cpu, el.gpu);
  el.fanChips = chips([{ id: 'auto', label: 'Auto' }, { id: 'maximum', label: 'Máximo' }], (id) => ctx.run(() => ctx.api.setFan(id)));
  el.fanChips.style.cssText = 'justify-content:center;margin-top:12px';
  fans.querySelector('.body').append(row, el.fanChips);
  wrap.append(cards, fans);
}

export function update({ store }) {
  const s = store.snap;
  for (const [id, c] of Object.entries(el.cards)) {
    c.classList.toggle('on', s.mode === id);
    c.querySelector('.swatch').style.background = store.config.palette[id].primary;
  }
  setFan(el.cpu, s.sensors.cpuFanRpm);
  setFan(el.gpu, s.sensors.gpuFanRpm);
  setChips(el.fanChips, s.fanMode);
}
```

- [ ] **Step 2: CSS (adicionar ao final de `theme.css`)**

```css
.perf { display: flex; flex-direction: column; gap: 14px; }
.mode-cards { display: grid; grid-template-columns: repeat(5, 1fr); gap: 10px; }
.mode-card { text-align: left; min-height: 120px; }
.mode-card .swatch { width: 100%; height: 6px; margin-bottom: 10px; box-shadow: 0 0 10px currentColor; }
.mode-card .name { font-size: 20px; font-weight: 800; letter-spacing: .06em; margin-bottom: 6px; }
.mode-card.on { border-color: var(--r); box-shadow: inset 0 0 30px var(--glow); }
body.offline .mode-card { opacity: .4; pointer-events: none; }
```

- [ ] **Step 3: Conferir no demo**

Abra `http://localhost:8765/?demo=1`, aba Desempenho.
Expected: 5 cartões com a faixa na cor de cada modo; o cartão ativo brilha; as hélices giram e mostram RPM (demo ~3800 em Equilibrado); "Máximo" acelera até ~7040 RPM em alguns segundos, e as hélices giram mais rápido.

- [ ] **Step 4: Commit**

```bash
git add ui
git commit -m "feat(ui): performance tab with mode cards and real-RPM fans

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 10: Aba Teclado

**Files:**
- Modify: `ui/js/tabs/keyboard.js`, `ui/css/theme.css` (final)

**Interfaces:**
- Consumes: `ctx.store.config.keyboard/palette`, `ctx.saveConfig(cfg)`, `logic.{MODES, isHex}`, `widgets.{panel, chips, setChips}`

- [ ] **Step 1: Implementar `keyboard.js`**

```js
import { MODES, isHex } from '../logic.js';
import { panel, chips, setChips } from '../widgets.js';

const el = {};
let draft = null;
let saveTimer = null;

const EFFECTS = [
  { id: 'off', label: 'Desligado' },
  { id: 'static', label: 'Estático' },
  { id: 'breathing', label: 'Respiração' },
  { id: 'neon', label: 'Neon' },
];

function scheduleSave(ctx) {
  clearTimeout(saveTimer);
  saveTimer = setTimeout(async () => {
    await ctx.run(() => ctx.saveConfig(structuredClone(draft)));
    saveTimer = null;
  }, 250);
}

export function mount(root, ctx) {
  root.innerHTML = '<div class="kbtab"></div>';
  const wrap = root.firstElementChild;

  const fx = panel('Efeito');
  el.fx = chips(EFFECTS, (id) => { draft.keyboard.effect = id; scheduleSave(ctx); update(ctx); });
  fx.querySelector('.body').append(el.fx);
  fx.querySelector('.body').insertAdjacentHTML('beforeend', `
    <div class="field"><span>Brilho</span><span><input type="range" min="0" max="100" data-k="brightness"> <b data-v="brightness"></b></span></div>
    <div class="field" data-row="speed"><span>Velocidade</span><span><input type="range" min="0" max="9" data-k="speed"> <b data-v="speed"></b></span></div>
    <div class="field"><span>Acompanhar a cor do modo</span><button class="switch" data-k="followMode"></button></div>`);
  fx.querySelectorAll('input[type=range]').forEach((r) =>
    r.addEventListener('input', () => { draft.keyboard[r.dataset.k] = Number(r.value); scheduleSave(ctx); update(ctx); }));
  fx.querySelector('[data-k=followMode]').addEventListener('click', () => {
    draft.keyboard.followMode = !draft.keyboard.followMode; scheduleSave(ctx); update(ctx);
  });

  const zones = panel('Cores por zona (com "Acompanhar" desligado)');
  zones.querySelector('.body').innerHTML = '<div class="zones">' +
    [0, 1, 2, 3].map((i) => `<label class="zone"><input type="color" data-zone="${i}"><span>Zona ${i + 1}</span></label>`).join('') + '</div>';
  zones.querySelectorAll('[data-zone]').forEach((inp) =>
    inp.addEventListener('input', () => { draft.keyboard.zones[Number(inp.dataset.zone)] = inp.value; scheduleSave(ctx); }));
  el.zonesPanel = zones;

  const pal = panel('Paleta dos modos (janela e teclado)');
  pal.querySelector('.body').innerHTML = MODES.map((m) =>
    `<div class="field"><span>${m.label}</span><span><input type="color" data-mode="${m.id}" data-which="primary"> <input type="color" data-mode="${m.id}" data-which="secondary"></span></div>`).join('') +
    '<div style="margin-top:10px"><button class="chip" data-reset>Restaurar paleta padrão</button></div>';
  pal.querySelectorAll('input[data-mode]').forEach((inp) =>
    inp.addEventListener('input', () => {
      if (!isHex(inp.value)) return;
      draft.palette[inp.dataset.mode][inp.dataset.which] = inp.value;
      scheduleSave(ctx);
    }));
  pal.querySelector('[data-reset]').addEventListener('click', () => {
    draft.palette = {
      eco: { primary: '#22c55e', secondary: '#86efac' },
      quiet: { primary: '#38bdf8', secondary: '#a5e3ff' },
      balanced: { primary: '#ff8a1f', secondary: '#ffc07a' },
      performance: { primary: '#ff2a1a', secondary: '#ff6a2b' },
      turbo: { primary: '#b026ff', secondary: '#ff3df2' },
    };
    scheduleSave(ctx);
    update(ctx);
  });

  wrap.append(fx, zones, pal);
  el.root = root;
}

export function update({ store }) {
  if (!draft || !saveTimer) draft = structuredClone(store.config);
  const k = draft.keyboard;
  setChips(el.fx, k.effect);
  for (const key of ['brightness', 'speed']) {
    const r = el.root.querySelector(`input[data-k=${key}]`);
    if (document.activeElement !== r) r.value = k[key];
    el.root.querySelector(`[data-v=${key}]`).textContent = k[key];
  }
  el.root.querySelector('[data-row=speed]').style.opacity = k.effect === 'breathing' || k.effect === 'neon' ? 1 : 0.35;
  el.root.querySelector('[data-k=followMode]').classList.toggle('on', k.followMode);
  el.zonesPanel.style.opacity = k.followMode ? 0.4 : 1;
  el.zonesPanel.style.pointerEvents = k.followMode ? 'none' : 'auto';
  el.root.querySelectorAll('[data-zone]').forEach((inp) => {
    if (document.activeElement !== inp) inp.value = k.zones[Number(inp.dataset.zone)];
  });
  el.root.querySelectorAll('input[data-mode]').forEach((inp) => {
    if (document.activeElement !== inp) inp.value = draft.palette[inp.dataset.mode][inp.dataset.which];
  });
}
```

Observação: `draft` só é recarregado da config salva quando não há salvamento pendente (`saveTimer` nulo), para não apagar o que o usuário está arrastando.

- [ ] **Step 2: CSS (final de `theme.css`)**

```css
.kbtab { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; }
.kbtab > .pn:last-child { grid-column: span 2; }
.zones { display: grid; grid-template-columns: repeat(4, 1fr); gap: 10px; }
.zone { display: flex; flex-direction: column; align-items: center; gap: 6px; font-size: 12px; color: var(--mut); }
.zone input[type=color] { width: 100%; height: 40px; }
```

- [ ] **Step 3: Conferir no demo**

Expected: trocar para "Respiração" ativa a linha de velocidade; desligar "Acompanhar" libera as 4 zonas; a prévia do teclado na aba Início reflete as zonas; mudar a cor do Turbo e trocar para Turbo mostra a janela na cor nova.

- [ ] **Step 4: Commit**

```bash
git add ui
git commit -m "feat(ui): keyboard tab — effect, brightness, speed, follow-mode, zones and palette editor

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 11: Aba Monitor

**Files:**
- Modify: `ui/js/tabs/monitor.js`, `ui/css/theme.css` (final)

**Interfaces:**
- Consumes: `ctx.api.getHistory()` (lista de `Sample` com `t, cpuTemp, gpuTemp, cpuUsage, gpuUsage, gpuPower, cpuFan, gpuFan`), `widgets.{chart, setChart}`, `logic.FAN_MAX_RPM`

- [ ] **Step 1: Implementar `monitor.js`**

```js
import { chart, setChart } from '../widgets.js';
import { FAN_MAX_RPM } from '../logic.js';

const el = {};
let samples = [];
let lastT = 0;

export function mount(root) {
  root.innerHTML = '<div class="montab"></div>';
  const w = root.firstElementChild;
  el.temp = chart('Temperatura (5 min)', [{ key: 'cpuTemp', label: 'CPU', color: 'var(--r)' }, { key: 'gpuTemp', label: 'GPU', color: '#7dd3fc' }]);
  el.use = chart('Uso', [{ key: 'cpuUsage', label: 'CPU', color: 'var(--r)' }, { key: 'gpuUsage', label: 'GPU', color: '#7dd3fc' }]);
  el.power = chart('Energia da GPU', [{ key: 'gpuPower', label: 'W', color: 'var(--r2)' }]);
  el.fans = chart('Ventoinhas', [{ key: 'cpuFan', label: 'CPU', color: 'var(--r)' }, { key: 'gpuFan', label: 'GPU', color: '#7dd3fc' }]);
  w.append(el.temp, el.use, el.power, el.fans);
}

function draw() {
  const maxW = Math.max(100, ...samples.map((s) => s.gpuPower ?? 0));
  setChart(el.temp, samples, 20, 100, '°');
  setChart(el.use, samples, 0, 100, '%');
  setChart(el.power, samples, 0, Math.ceil(maxW / 20) * 20, ' W');
  setChart(el.fans, samples, 0, FAN_MAX_RPM, '');
}

export async function shown({ api }) {
  samples = await api.getHistory();
  lastT = samples.at(-1)?.t ?? 0;
  draw();
}

export function update({ store }) {
  const s = store.snap.sensors;
  const now = Math.floor(Date.now() / 1000);
  if (now === lastT) return;
  lastT = now;
  const g = s.gpu?.state === 'active' ? s.gpu : {};
  samples.push({ t: now, cpuTemp: s.cpuTemp, gpuTemp: s.gpuTemp, cpuUsage: s.cpuUsage, gpuUsage: g.usage ?? null, gpuPower: g.powerW ?? null, cpuFan: s.cpuFanRpm, gpuFan: s.gpuFanRpm });
  if (samples.length > 300) samples.shift();
  if (document.getElementById('tab-monitor').classList.contains('on')) draw();
}
```

Observação: `setChart` usa `stroke` com `var(--r)`, que funciona como atributo de apresentação SVG no WebKitGTK. Se a linha não aparecer, troque para `style="stroke:var(--r)"` em `widgets.chart`.

- [ ] **Step 2: CSS**

```css
.montab { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; }
```

- [ ] **Step 3: Conferir no demo**

Expected: ao abrir a aba, os gráficos começam do histórico e avançam a cada segundo; "Máximo" na aba Desempenho faz a curva das ventoinhas subir.

- [ ] **Step 4: Commit**

```bash
git add ui
git commit -m "feat(ui): monitor tab with 5-minute charts

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 12: Aba Sistema

**Files:**
- Modify: `ui/js/tabs/system.js`, `ui/css/theme.css` (final)

**Interfaces:**
- Consumes: `store.snap.platform` (`batteryLimit, batteryCalibration, usbCharging, keyboardTimeout, bootSound, lcdOverride`), `api.setPlatform(key, value)`, `api.getAutostart/setAutostart`, `store.config.login`, `ctx.saveConfig`, `widgets.{panel, chips, setChips}`, `logic.MODES`

- [ ] **Step 1: Implementar `system.js`**

```js
import { MODES } from '../logic.js';
import { panel, chips, setChips } from '../widgets.js';

const el = {};
const TOGGLES = [
  { key: 'BATTERY_LIMIT', field: 'batteryLimit', label: 'Limitar carga da bateria em 80%' },
  { key: 'KEYBOARD_TIMEOUT', field: 'keyboardTimeout', label: 'Apagar a luz do teclado após inatividade' },
  { key: 'BOOT_SOUND', field: 'bootSound', label: 'Som ao ligar' },
  { key: 'LCD_OVERRIDE', field: 'lcdOverride', label: 'LCD override' },
];

export function mount(root, ctx) {
  root.innerHTML = '<div class="systab"></div>';
  const w = root.firstElementChild;

  const plat = panel('Bateria e hardware');
  const body = plat.querySelector('.body');
  for (const t of TOGGLES) {
    body.insertAdjacentHTML('beforeend', `<div class="field"><span>${t.label}</span><button class="switch needs-daemon" data-p="${t.key}"></button></div>`);
  }
  body.querySelectorAll('[data-p]').forEach((b) => b.addEventListener('click', () => {
    const on = !b.classList.contains('on');
    ctx.run(() => ctx.api.setPlatform(b.dataset.p, on ? 'ON' : 'OFF'));
  }));
  body.insertAdjacentHTML('beforeend', '<div class="field"><span>Carregar USB com o notebook desligado</span><span data-usb></span></div>');
  el.usb = chips([{ id: '0', label: 'Não' }, { id: '10', label: '10%' }, { id: '20', label: '20%' }, { id: '30', label: '30%' }],
    (id) => ctx.run(() => ctx.api.setPlatform('USB_CHARGING', id)));
  body.querySelector('[data-usb]').append(el.usb);
  body.insertAdjacentHTML('beforeend', '<div class="field"><span>Calibração da bateria</span><button class="chip needs-daemon" data-cal></button></div>');
  el.cal = body.querySelector('[data-cal]');
  el.cal.addEventListener('click', () => {
    const running = ctx.store.snap.platform?.batteryCalibration;
    ctx.run(() => ctx.api.setPlatform('BATTERY_CALIBRATION', running ? 'STOP' : 'START'));
  });

  const login = panel('Ao iniciar a sessão');
  login.classList.add('login');
  const lb = login.querySelector('.body');
  lb.innerHTML = '<div class="field"><span>Aplicar padrões no login</span><button class="switch" data-login-apply></button></div><div class="ttl" style="margin-top:10px">Modo inicial</div>';
  el.loginMode = chips(MODES, (id) => saveLogin(ctx, { mode: id }));
  lb.append(el.loginMode);
  lb.insertAdjacentHTML('beforeend', '<div class="ttl" style="margin-top:10px">Ventoinha inicial</div>');
  el.loginFan = chips([{ id: 'auto', label: 'Auto' }, { id: 'maximum', label: 'Máximo' }], (id) => saveLogin(ctx, { fan: id }));
  lb.append(el.loginFan);
  lb.insertAdjacentHTML('beforeend', '<div class="field" style="margin-top:10px"><span>Iniciar o Nitro Control com o sistema</span><button class="switch" data-autostart></button></div>');
  lb.querySelector('[data-login-apply]').addEventListener('click', () => saveLogin(ctx, { apply: !ctx.store.config.login.apply }));
  el.auto = lb.querySelector('[data-autostart]');
  el.auto.addEventListener('click', () => ctx.run(async () => {
    await ctx.api.setAutostart(!el.auto.classList.contains('on'));
    el.auto.classList.toggle('on', await ctx.api.getAutostart());
  }));
  ctx.api.getAutostart().then((v) => el.auto.classList.toggle('on', v));

  w.append(plat, login);
  el.root = root;
}

function saveLogin(ctx, patch) {
  const cfg = structuredClone(ctx.store.config);
  Object.assign(cfg.login, patch);
  ctx.run(() => ctx.saveConfig(cfg));
}

export function update({ store }) {
  const p = store.snap.platform ?? {};
  el.root.querySelectorAll('[data-p]').forEach((b) => {
    const t = TOGGLES.find((x) => x.key === b.dataset.p);
    b.classList.toggle('on', !!p[t.field]);
  });
  setChips(el.usb, p.usbCharging != null ? String(p.usbCharging) : null);
  el.cal.textContent = p.batteryCalibration ? 'Parar calibração' : 'Iniciar calibração';
  const l = store.config.login;
  el.root.querySelector('[data-login-apply]').classList.toggle('on', l.apply);
  setChips(el.loginMode, l.mode);
  setChips(el.loginFan, l.fan);
}
```

Adicione ao final de `theme.css` (as preferências de login não dependem do daemon, então continuam ativas offline):
```css
.systab { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; }
body.offline .systab .login .chip { opacity: 1; pointer-events: auto; }
```

- [ ] **Step 2: Conferir no demo**

Expected: os interruptores refletem o `platform` do demo e mudam ao clicar; USB 30% marcado; as preferências de login ficam salvas na config do demo.

- [ ] **Step 3: Rodar todos os testes e commit**

Run: `node --test tests-ui/ && (cd src-tauri && cargo test)`
Expected: tudo PASS.

```bash
git add ui
git commit -m "feat(ui): system tab — battery/USB/boot sound/keyboard timeout, login defaults, autostart

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 13: Pacote .deb e migração

**Files:**
- Create: `packaging/migrar.sh`, `packaging/reverter.sh`

**Interfaces:**
- Consumes: binário `nitro-control` com `--hidden` e `--toggle` (Task 7); autostart em `~/.config/autostart/nitro-control.desktop` (Task 4).

- [ ] **Step 1: Gerar o pacote**

Run: `cd ~/nitro-control/src-tauri && cargo tauri build`
Expected: `src-tauri/target/release/bundle/deb/Nitro Control_0.1.0_amd64.deb` existe. Confira o conteúdo: `dpkg -c src-tauri/target/release/bundle/deb/*.deb | grep -E 'bin/|applications'` mostra `usr/bin/nitro-control` e um `.desktop`.

- [ ] **Step 2: Escrever `packaging/migrar.sh`**

```bash
#!/bin/bash
# Migra do ASense GUI para o Nitro Control. Guarda tudo em ~/nitro-control/packaging/backup-<data>.
set -euo pipefail
BK="$HOME/nitro-control/packaging/backup-$(date +%Y%m%d-%H%M%S)"
AUTO="$HOME/.config/autostart"
KEY=/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/asense/
mkdir -p "$BK"

for f in asense_turbo_fans.desktop rgb_config_acer_gkbbl_0.desktop; do
  if [ -f "$AUTO/$f" ]; then mv "$AUTO/$f" "$BK/"; echo "autostart removido: $f"; fi
done

dconf read "${KEY}command" > "$BK/keybinding-command.txt" || true
dconf write "${KEY}command" "'/usr/bin/nitro-control --toggle'"
dconf write "${KEY}name" "'Nitro Control'"
echo "tecla NitroSense (XF86Launch1) -> nitro-control --toggle"

pkill -x asense 2>/dev/null || true

mkdir -p "$AUTO"
cat > "$AUTO/nitro-control.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=Nitro Control
Comment=Painel de controle do Acer Nitro
Exec=nitro-control --hidden
Icon=nitro-control
Terminal=false
X-GNOME-Autostart-enabled=true
EOF
echo "autostart do Nitro Control instalado"
echo "backup em $BK (reverter: packaging/reverter.sh $BK)"
```

- [ ] **Step 3: Escrever `packaging/reverter.sh`**

```bash
#!/bin/bash
# Desfaz o migrar.sh a partir de um diretório de backup.
set -euo pipefail
BK="${1:?uso: reverter.sh <dir-de-backup>}"
AUTO="$HOME/.config/autostart"
KEY=/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/asense/

for f in asense_turbo_fans.desktop rgb_config_acer_gkbbl_0.desktop; do
  [ -f "$BK/$f" ] && cp "$BK/$f" "$AUTO/" && echo "restaurado: $f"
done
rm -f "$AUTO/nitro-control.desktop"
CMD=$(cat "$BK/keybinding-command.txt" 2>/dev/null || echo "'/usr/bin/asense --toggle'")
dconf write "${KEY}command" "${CMD:-'/usr/bin/asense --toggle'}"
dconf write "${KEY}name" "'ASense'"
pkill -x nitro-control 2>/dev/null || true
echo "revertido; a tecla NitroSense volta a abrir o ASense"
```

Run: `chmod +x packaging/*.sh && bash -n packaging/migrar.sh && bash -n packaging/reverter.sh && echo ok`
Expected: `ok`.

- [ ] **Step 4: Instalar e migrar (com o usuário)**

Peça ao usuário:
```
! pkexec apt-get install -y "$HOME/nitro-control/src-tauri/target/release/bundle/deb/Nitro Control_0.1.0_amd64.deb"
```
Depois rode `bash ~/nitro-control/packaging/migrar.sh` e `nitro-control --hidden &` para iniciar sem reiniciar a sessão.

Expected: ícone na barra com as temperaturas; a tecla NitroSense abre e fecha o painel.

- [ ] **Step 5: Commit**

```bash
git add packaging
git commit -m "feat: .deb packaging and reversible migration from the ASense GUI

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 14: Verificação no hardware real

**Files:** nenhum (só verificação; ajustes voltam para a task dona do código).

- [ ] **Step 1: Modos**

Para cada modo, clique no painel e confira:
```bash
cat /sys/firmware/acpi/platform_profile; asense probe --summary | grep Profile
```
Expected: Eco→`low-power`, Silencioso→`quiet`, Equilibrado→`balanced`, Desempenho→`balanced-performance`, Turbo→`performance`. A janela troca de cor com a animação e o teclado acompanha (verde, azul, laranja, vermelho, roxo).

- [ ] **Step 2: Mudança externa**

Com o painel aberto, mude pelo menu de energia do GNOME (canto superior direito) ou rode:
```bash
powerprofilesctl set power-saver
```
Expected: em até ~1 s o painel troca para o modo correspondente, com cor e animação, e o teclado muda de cor.

- [ ] **Step 3: Ventoinhas**

Clique em Máximo e depois em Auto.
Expected: o RPM sobe até ~7000 em ~4 s e depois desce; a hélice acelera; o chip e o menu da barra marcam o modo certo.

- [ ] **Step 4: Teclado**

Teste Respiração e Neon com velocidade 5, depois Desligado e Estático.
Expected: cada efeito é aplicado. Se o daemon recusar Respiração/Neon com velocidade diferente de 0 (o `asense_rgb` exige `speed=0` para os modos 0 e 1 do firmware), aparece um aviso com a mensagem do daemon; nesse caso, ajuste `lighting_commands` na Task 6 para mandar `speed=0` em `BREATHING`, com teste novo, e refaça este passo.

- [ ] **Step 5: Reconexão e conflito**

```bash
pkexec systemctl restart asense.service
```
Expected: o painel mostra "ASense desconectado" por até ~2 s e volta sozinho.

Depois abra `/usr/bin/asense` (a interface antiga) e reinicie o Nitro Control.
Expected: o painel avisa "Feche a interface do ASense" e as temperaturas continuam. Fechar o ASense reconecta em ~2 s.

- [ ] **Step 6: Barra e janela**

Expected: o rótulo na barra mostra `CPU° · GPU°` (ou `—` com a GPU dormindo); fechar a janela no ✕ mantém o ícone; a tecla NitroSense alterna a janela; "Sair" no menu encerra.

- [ ] **Step 7: Memória em repouso**

```bash
ps -o rss,cmd -C nitro-control
```
Expected: com a janela escondida, algo na faixa de 60–150 MB somando os processos. Registre o valor no relatório final.

- [ ] **Step 8: Commit final**

```bash
git commit --allow-empty -m "chore: hardware verification passed on Nitro AN16-51

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```
