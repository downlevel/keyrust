# Contributing to Keyrust

Thanks for your interest! Keyrust is at an early stage; issues and discussions are welcome.

## Ground rules

- **Security first.** Read [SECURITY.md](SECURITY.md). Never write cryptographic primitives by
  hand, never log or serialise secrets, keep secret values in `zeroize`/`secrecy` types.
  Report vulnerabilities privately, not in public issues.
- **Compatibility is a feature.** Any change touching the KDBX layer must keep the KeePassXC
  compatibility tests green (`crates/core/tests/compat_keepassxc.rs`).
- **No real vaults in the repository.** `.gitignore` blocks `*.kdbx` outside `tests/fixtures/`;
  fixtures contain test data only.

## Development setup

See [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).

## Before opening a pull request

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings   # the app crate needs the Tauri prerequisites
cargo test -p keyrust-core          # needs keepassxc-cli (see docs/DEVELOPMENT.md)
cargo deny check
cd crates/app/ui && pnpm check && pnpm lint && pnpm format:check && pnpm build
```

## Conventions

- Rust: edition 2024, `unsafe_code` forbidden, no `unwrap`/`expect` outside tests.
- Frontend: Svelte 5 runes, TypeScript strict, Tailwind + shadcn-svelte components.
- Commits: [Conventional Commits](https://www.conventionalcommits.org/) (`feat:`, `fix:`, `docs:` …).
- UI strings are English and go through the i18n layer (from Milestone 1).

By contributing you agree that your contributions are licensed under GPL-3.0-or-later.
