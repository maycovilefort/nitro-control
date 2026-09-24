use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{AppHandle, State};

use crate::asense::protocol::platform_tail;
use crate::autostart;
use crate::config::{self, Config, FanChoice};
use crate::hub::{Reply, Request};
use crate::mode::Mode;
use crate::state::{History, Sample, Snapshot};

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
