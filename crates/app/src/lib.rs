//! Tauri shell of Keyrust.
//!
//! Commands exposed to the webview live here. Secrets must never cross the IPC
//! boundary unless the user explicitly asks to reveal them (see
//! docs/ARCHITECTURE.md, "Principi di confine").

use std::path::PathBuf;
use std::sync::Mutex;

use keyrust_core::settings::{self, Settings};
use serde::Serialize;
use tauri::{Manager, State};

/// Names of the IPC commands, kept in sync with `generate_handler!` below and
/// checked by `tests/security_config.rs`.
pub const COMMANDS: &[&str] = &["app_info", "get_settings", "update_settings"];

#[derive(Serialize)]
struct AppInfo {
    version: &'static str,
}

/// Settings held in memory and the file they persist to.
struct SettingsState {
    path: PathBuf,
    current: Mutex<Settings>,
}

/// Returns non-sensitive build information for the UI.
#[tauri::command]
fn app_info() -> AppInfo {
    AppInfo {
        version: keyrust_core::VERSION,
    }
}

#[tauri::command]
fn get_settings(state: State<'_, SettingsState>) -> Result<Settings, String> {
    state
        .current
        .lock()
        .map(|s| s.clone())
        .map_err(|_| "settings unavailable".to_string())
}

/// Validates, persists and returns the effective settings. Errors never
/// include file paths.
#[tauri::command]
fn update_settings(
    state: State<'_, SettingsState>,
    settings: Settings,
) -> Result<Settings, String> {
    let settings = settings.normalized();
    // Hold the lock while saving so concurrent updates cannot leave the file
    // and the in-memory copy disagreeing.
    let mut current = state
        .current
        .lock()
        .map_err(|_| "settings unavailable".to_string())?;
    settings::save(&state.path, &settings)
        .map_err(|e| format!("could not save settings ({:?})", e.kind()))?;
    *current = settings.clone();
    Ok(settings)
}

/// Logs go to stderr only (no log file), filtered by `KEYRUST_LOG`
/// (default `info`). See the logging policy in CONTRIBUTING.md.
fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_env("KEYRUST_LOG")
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_tracing();
    tracing::info!(version = keyrust_core::VERSION, "starting Keyrust");

    #[allow(clippy::expect_used)] // Nothing to recover: the app cannot start.
    tauri::Builder::default()
        .setup(|app| {
            let path = app.path().app_config_dir()?.join(settings::FILE_NAME);
            let current = Mutex::new(settings::load(&path));
            app.manage(SettingsState { path, current });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_info,
            get_settings,
            update_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running the Keyrust application");
}
