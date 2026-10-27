//! Keyrust core: vault, crypto and KDBX4 format layer.
//!
//! This crate must stay free of any UI dependency so it can be reused by the
//! desktop app and a future CLI/TUI. The real vault API arrives in Milestone 3;
//! for now it only pins the `keepass` dependency exercised by the compatibility
//! spike in `tests/compat_keepassxc.rs`.

/// Version of the core crate, shown by the app.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
