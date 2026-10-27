//! The logging policy (CONTRIBUTING.md): at `info` and above, logs carry events,
//! error kinds and counts, never values or file paths.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::Write;
use std::sync::{Arc, Mutex};

use keyrust_core::kdbx::header;
use keyrust_core::settings::{self, Settings};
use tracing_subscriber::fmt::MakeWriter;

#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<u8>>>);

impl Write for Capture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for Capture {
    type Writer = Capture;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

/// Runs `f` with a subscriber that records `level` and above, returns the log.
fn logs_at(level: tracing::Level, f: impl FnOnce()) -> String {
    let capture = Capture::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(capture.clone())
        .with_max_level(level)
        .without_time()
        .with_ansi(false)
        .finish();
    tracing::subscriber::with_default(subscriber, f);
    String::from_utf8(capture.0.lock().unwrap().clone()).unwrap()
}

#[test]
fn settings_and_header_operations_log_no_values_or_paths_at_info() {
    let dir = tempfile::tempdir().unwrap();
    let dir_name = dir.path().file_name().unwrap().to_str().unwrap().to_owned();
    let path = dir.path().join("settings.json");
    let recent = "/home/someone/Private Vault 7f3a.kdbx";
    let mut settings = Settings {
        clipboard_clear_secs: 137,
        ..Settings::default()
    };
    settings.push_recent_file(recent.into());

    let run = || {
        settings::save(&path, &settings).unwrap();
        settings::load(&path);
        std::fs::write(&path, b"{ corrupt 7f3a").unwrap();
        settings::load(&path);
        let _ = header::peek(b"not a database");
    };

    let info = logs_at(tracing::Level::INFO, run);
    // The operations do log events at info and above...
    assert!(info.contains("settings saved"), "log: {info}");
    assert!(info.contains("settings file is not valid"), "log: {info}");
    // ...but no values, file contents or paths.
    for forbidden in [
        recent,
        "Private Vault",
        "7f3a",
        "137",
        dir_name.as_str(),
        "settings.json",
    ] {
        assert!(
            !info.contains(forbidden),
            "`{forbidden}` leaked into info logs:\n{info}"
        );
    }
}

#[test]
fn file_paths_appear_only_at_debug() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let debug = logs_at(tracing::Level::DEBUG, || {
        settings::save(&path, &Settings::default()).unwrap();
    });
    assert!(
        debug.contains("settings.json"),
        "debug log should name the file: {debug}"
    );
}

#[test]
fn vault_operations_log_no_values_or_paths_at_info() {
    use keyrust_core::model::{Argon2idParams, Cipher, CreateOptions, EntryDraft};
    use keyrust_core::vault::Vault;
    use secrecy::SecretString;

    let dir = tempfile::tempdir().unwrap();
    let dir_name = dir.path().file_name().unwrap().to_str().unwrap().to_owned();
    let path = dir.path().join("Private Vault 7f3a.kdbx");
    let secret = |s: &str| SecretString::from(s.to_owned());

    let info = logs_at(tracing::Level::INFO, || {
        let mut vault = Vault::create(
            &path,
            CreateOptions {
                name: "Household 9c1e".into(),
                password: secret("master-pw-55aa"),
                cipher: Cipher::ChaCha20,
                kdf: Argon2idParams {
                    memory_mib: 8,
                    iterations: 1,
                    parallelism: 1,
                },
            },
        )
        .unwrap();
        let root = vault.tree().id;
        let id = vault
            .save_entry(&EntryDraft {
                id: None,
                group_id: root,
                title: "Bank 31d4".into(),
                username: "alice-8b2f".into(),
                password: secret("entry-pw-77cc"),
                url: "https://bank-4e6a.example".into(),
                notes: "pin hint 0d9e".into(),
            })
            .unwrap();
        vault.save().unwrap();
        let _ = Vault::open(&path, secret("wrong-pw-1234"));
        let mut vault = Vault::open(&path, secret("master-pw-55aa")).unwrap();
        vault.delete_entry(id).unwrap();
        vault.save().unwrap();
    });

    assert!(info.contains("vault saved"), "log: {info}");
    assert!(
        info.contains("entry moved to the recycle bin"),
        "log: {info}"
    );
    for forbidden in [
        "Private Vault",
        "7f3a",
        dir_name.as_str(),
        "Household",
        "master-pw",
        "wrong-pw",
        "Bank 31d4",
        "alice-8b2f",
        "entry-pw",
        "bank-4e6a",
        "0d9e",
    ] {
        assert!(
            !info.contains(forbidden),
            "`{forbidden}` leaked into info logs:\n{info}"
        );
    }
}
