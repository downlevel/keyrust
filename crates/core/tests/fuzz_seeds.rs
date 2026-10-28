//! Generates the seed corpus of the `vault_open_bytes` fuzz target and
//! exercises the round-trip harness locally (feature `fuzzing`):
//!
//! ```sh
//! cargo test -p keyrust-core --features fuzzing --test fuzz_seeds -- --include-ignored
//! ```
//!
//! Seeds use the target's password and minimal key derivation, so each one
//! opens under `Limits::FUZZ`; they are written with keepass-rs directly
//! because `Vault::create` refuses such weak parameters.

#![cfg(feature = "fuzzing")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use arbitrary::{Arbitrary, Unstructured};
use keepass::config::{DatabaseConfig, KdfConfig};
use keepass::db::{History, Value, fields};
use keepass::{Database, DatabaseKey};
use keyrust_core::fuzzing::{self, Model};
use keyrust_core::kdbx::limits::Limits;
use keyrust_core::vault::Vault;
use secrecy::SecretString;

fn seed(kdf: KdfConfig) -> Vec<u8> {
    let mut config = DatabaseConfig::default();
    config.kdf_config = kdf;
    let mut db = Database::with_config(config);
    db.meta.recyclebin_enabled = Some(true);
    let mut root = db.root_mut();
    root.name = "Seed".into();
    let mut group = root.add_group();
    group.name = "Work ✓".into();
    let mut entry = group.add_entry();
    entry.set_unprotected(fields::TITLE, "GitHub");
    entry.set_unprotected(fields::USERNAME, "alice");
    entry.set_protected(fields::PASSWORD, "s3cret");
    entry.set_unprotected(fields::URL, "https://github.com");
    entry.set_unprotected(fields::NOTES, "line one\nline two ünïcode");
    entry.set_protected("Recovery code", "RC-1234");
    entry.add_attachment("readme.txt", Value::unprotected(b"hello".to_vec()));
    let mut version = (*entry).clone();
    version.history = None;
    version.set_protected(fields::PASSWORD, "old");
    let mut history = History::default();
    history.add_entry(version);
    entry.history = Some(history);
    let mut bytes = Vec::new();
    db.save(
        &mut bytes,
        DatabaseKey::new().with_password(fuzzing::PASSWORD),
    )
    .expect("save seed");
    bytes
}

#[test]
#[ignore = "writes the fuzz corpus; run explicitly"]
fn generate_fuzz_seeds() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/corpus/vault_open_bytes");
    std::fs::create_dir_all(&dir).expect("corpus dir");
    let seeds = [
        ("seed-aes-kdf.kdbx", KdfConfig::Aes { rounds: 100 }),
        (
            "seed-argon2id.kdbx",
            KdfConfig::Argon2id {
                iterations: 1,
                memory: 1024 * 1024,
                parallelism: 1,
                version: argon2::Version::Version13,
            },
        ),
    ];
    for (name, kdf) in seeds {
        let bytes = seed(kdf);
        Vault::open_bytes_with_limits(&bytes, SecretString::from(fuzzing::PASSWORD), &Limits::FUZZ)
            .unwrap_or_else(|e| panic!("{name} must open under the fuzz limits: {e}"));
        std::fs::write(dir.join(name), bytes).expect("write seed");
    }
}

/// The committed seeds still open under the fuzz target's exact conditions.
#[test]
fn committed_seeds_open_under_the_fuzz_limits() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/corpus/vault_open_bytes");
    // `seed-*` are ours; others (the KDBX 3.1 fixture) only explore headers.
    let seeds: Vec<_> = std::fs::read_dir(dir)
        .expect("corpus")
        .map(|e| e.expect("entry").path())
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("seed-"))
        })
        .collect();
    assert_eq!(seeds.len(), 2, "{seeds:?}");
    for path in seeds {
        let bytes = std::fs::read(path).expect("seed");
        Vault::open_bytes_with_limits(&bytes, SecretString::from(fuzzing::PASSWORD), &Limits::FUZZ)
            .expect("seed opens");
    }
}

/// A few hundred pseudo-random models through the round-trip harness.
#[test]
fn round_trip_harness_on_random_models() {
    let mut state = 0x9E37_79B9_7F4A_7C15u64;
    for run in 0..300 {
        let mut data = vec![0u8; 4096];
        for b in &mut data {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            *b = state as u8;
        }
        let model = Model::arbitrary(&mut Unstructured::new(&data)).expect("model");
        fuzzing::roundtrip(&model).unwrap_or_else(|e| panic!("run {run}: {e}"));
    }
}
