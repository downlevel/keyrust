//! Builds a database from arbitrary content (groups, entries, Unicode fields,
//! custom fields, history, attachments), saves it with keepass-rs and reopens
//! it through `Vault`: what was written must come back unchanged.

#![no_main]

use keyrust_core::fuzzing::{Model, roundtrip};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|model: Model| {
    if let Err(e) = roundtrip(&model) {
        panic!("{e}");
    }
});
