//! Tauri shell of Keyrust.
//!
//! Commands exposed to the webview live in `commands`. Secrets must never
//! cross the IPC boundary unless the user explicitly asks to reveal them (see
//! docs/ARCHITECTURE.md, "Principi di confine").
//!
//! The dialog and opener plugins are called from Rust only: no capability
//! grants the webview any of their permissions.

mod autolock;
mod clipboard;
mod commands;
mod error;
mod menu;

use std::sync::Mutex;

use keyrust_core::settings;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, WindowEvent};

use autolock::{AutoLock, Reason};
use clipboard::{Board, Clipboard, SystemBoard};
#[allow(clippy::wildcard_imports)] // generate_handler! below lists bare names
use commands::*;

/// Names of the IPC commands, kept in sync with `generate_handler!` below and
/// checked by `tests/security_config.rs`.
pub const COMMANDS: &[&str] = &[
    "app_info",
    "get_settings",
    "update_settings",
    "exit_app",
    "recent_files",
    "choose_database_file",
    "choose_new_database_path",
    "unlock",
    "create",
    "lock_vault",
    "activity",
    "database_info",
    "tree",
    "list_entries",
    "get_entry",
    "get_entry_for_edit",
    "reveal_field",
    "copy_field",
    "save_entry",
    "delete_entry",
    "restore_entry",
    "create_group",
    "rename_group",
    "delete_group",
    "update_database",
    "change_master_password",
    "save",
    "save_overwrite",
    "reload",
    "check_file",
    "generate_password",
    "estimate_strength",
    "benchmark_kdf",
    "open_url",
];

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

/// Quits the application. The UI calls it once unsaved changes have been
/// handled; the clipboard is cleared if it still holds a copied value.
#[tauri::command]
fn exit_app(app: AppHandle) {
    tracing::info!("quit requested");
    app.state::<Clipboard>().clear_if_unchanged(None);
    app.exit(0);
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
        .plugin(tauri_plugin_dialog::init())
        // No script that routes webview links through the plugin: links are
        // opened by the `open_url` command after validation.
        .plugin(
            tauri_plugin_opener::Builder::new()
                .open_js_links_on_click(false)
                .build(),
        )
        .setup(|app| {
            let path = app.path().app_config_dir()?.join(settings::FILE_NAME);
            let current = Mutex::new(settings::load(&path));
            app.manage(SettingsState { path, current });
            app.manage(VaultState::default());
            app.manage(AllowedPaths::default());
            let board = SystemBoard::new().map(|b| Box::new(b) as Box<dyn Board>);
            if board.is_none() {
                tracing::warn!("no system clipboard available");
            }
            app.manage(Clipboard::new(board));
            app.set_menu(menu::build(app.handle())?)?;
            autolock::start(app.handle());
            Ok(())
        })
        .on_menu_event(|app, event| {
            // Only registry commands are forwarded; predefined items (copy,
            // paste, hide…) are handled natively.
            if let Ok(registry) = menu::registry() {
                let known: Vec<String> = registry.commands.into_iter().map(|c| c.id).collect();
                menu::forward(app, event.id().as_ref(), &known);
            }
        })
        .on_window_event(|window, event| {
            // With a database open, closing the window goes through the UI's
            // Quit command, which handles unsaved changes first.
            match event {
                WindowEvent::CloseRequested { api, .. } => {
                    let open = window.state::<VaultState>().is_open();
                    if open && window.emit(menu::MENU_EVENT, "quit").is_ok() {
                        api.prevent_close();
                    } else {
                        window.state::<Clipboard>().clear_if_unchanged(None);
                    }
                }
                WindowEvent::Resized(_) if window.is_minimized().unwrap_or(false) => {
                    window.state::<AutoLock>().trigger(Reason::Minimize);
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            app_info,
            get_settings,
            update_settings,
            exit_app,
            recent_files,
            choose_database_file,
            choose_new_database_path,
            unlock,
            create,
            lock_vault,
            activity,
            database_info,
            tree,
            list_entries,
            get_entry,
            get_entry_for_edit,
            reveal_field,
            copy_field,
            save_entry,
            delete_entry,
            restore_entry,
            create_group,
            rename_group,
            delete_group,
            update_database,
            change_master_password,
            save,
            save_overwrite,
            reload,
            check_file,
            generate_password,
            estimate_strength,
            benchmark_kdf,
            open_url
        ])
        .run(tauri::generate_context!())
        .expect("error while running the Keyrust application");
}
