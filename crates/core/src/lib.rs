//! Keyrust core: vault, crypto and KDBX4 format layer.
//!
//! This crate must stay free of any UI dependency so it can be reused by the
//! desktop app and a future CLI/TUI. The vault API arrives in Milestone 3.
//!
//! Logging policy (see CONTRIBUTING.md): events, error kinds and counts only.
//! Never log field values, passwords, keys or vault content; file paths only at
//! `debug` level.

pub mod kdbx;
pub mod settings;

/// Version of the core crate, shown by the app.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
