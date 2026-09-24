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
