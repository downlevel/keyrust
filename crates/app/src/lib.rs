//! Tauri shell of Keyrust.
//!
//! Commands exposed to the webview live here. Secrets must never cross the IPC
//! boundary unless the user explicitly asks to reveal them (see
//! docs/ARCHITECTURE.md, "Principi di confine").

use serde::Serialize;

#[derive(Serialize)]
struct AppInfo {
    version: &'static str,
}

/// Returns non-sensitive build information for the UI.
#[tauri::command]
fn app_info() -> AppInfo {
    AppInfo {
        version: keyrust_core::VERSION,
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[allow(clippy::expect_used)] // Nothing to recover: the app cannot start.
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![app_info])
        .run(tauri::generate_context!())
        .expect("error while running the Keyrust application");
}
