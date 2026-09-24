pub mod asense;
pub mod automation;
pub mod autostart;
pub mod config;
pub mod hub;
pub mod mode;
pub mod sensors;
pub mod state;

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
