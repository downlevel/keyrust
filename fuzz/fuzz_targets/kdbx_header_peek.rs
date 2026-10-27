//! Fuzzes the KDBX4 outer header reader with arbitrary bytes: it must return
//! an error or a summary, never panic, loop or allocate unboundedly.

#![no_main]

use keyrust_core::kdbx::header::{MAX_HEADER_LEN, peek};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let first = peek(data);
    // Only the first MAX_HEADER_LEN bytes may influence the result.
    if data.len() > MAX_HEADER_LEN {
        let capped = peek(&data[..MAX_HEADER_LEN]);
        if let Ok(summary) = &first {
            assert_eq!(capped.as_ref(), Ok(summary));
        }
    }
});
