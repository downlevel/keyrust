//! Fuzzes opening a whole database file under the tiny `Limits::FUZZ`: it must
//! return an error or a vault, never panic, hang or allocate unboundedly.
//! Random mutations of an encrypted file mostly stop at the integrity checks
//! (header hash, HMAC); `vault_roundtrip` covers what lies beyond.

#![no_main]

use keyrust_core::fuzzing::PASSWORD;
use keyrust_core::kdbx::limits::Limits;
use keyrust_core::vault::Vault;
use libfuzzer_sys::fuzz_target;
use secrecy::SecretString;

fuzz_target!(|data: &[u8]| {
    let _ = Vault::open_bytes_with_limits(data, SecretString::from(PASSWORD), &Limits::FUZZ);
});
