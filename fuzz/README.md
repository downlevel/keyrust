# Fuzzing

[cargo-fuzz](https://rust-fuzz.github.io/book/cargo-fuzz.html) targets for code that parses
untrusted input. Requires a nightly toolchain:

```sh
rustup toolchain install nightly
cargo install cargo-fuzz --locked
cd fuzz
cargo +nightly fuzz run kdbx_header_peek -- -max_total_time=60
```

The seed corpus in `corpus/kdbx_header_peek/` holds copies of the KeePassXC test fixtures
(test data only). Crashes are written to `artifacts/` (ignored by git). CI runs every target
for 60 seconds on each push.

| Target | Code under test |
|---|---|
| `kdbx_header_peek` | `keyrust_core::kdbx::header::peek`: KDBX4 outer header, no crypto |

**Not fuzzed yet: `keepass::Database::parse`.** It runs the key derivation function with the
parameters found in the file itself, so a fuzzer immediately hits multi-gigabyte Argon2 memory
settings (a denial-of-service vector, not a crash) and runs a handful of iterations per second.
Milestone 3 adds KDF limits checked with `peek` before opening; a target for the full parse will
apply the same limits.
