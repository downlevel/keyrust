//! Keyrust core: vault, crypto and KDBX4 format layer.
//!
//! This crate must stay free of any UI dependency so it can be reused by the
//! desktop app and a future CLI/TUI. [`vault::Vault`] is the entry point.
//!
//! Logging policy (see CONTRIBUTING.md): events, error kinds and counts only.
//! Never log field values, passwords, keys or vault content; file paths only at
//! `debug` level.

pub mod error;
mod file;
pub mod generator;
pub mod kdbx;
pub mod kdf;
pub mod model;
pub mod settings;
pub mod strength;
pub mod vault;

/// Version of the core crate, shown by the app.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
