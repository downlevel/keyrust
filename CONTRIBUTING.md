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
- UI strings are English and go through Paraglide: add them to `crates/app/ui/messages/en.json` and
  use `m.key()` in markup (a test rejects literal text in route markup).

## Commands and shortcuts

Every user action that has a menu item, a shortcut or a palette entry is declared once in
`crates/app/ui/src/lib/commands/commands.json`. To add one:

1. add the command (`id`, `menu`, `labelKey`, `shortcut`, `scope`, `nativeAccelerator`) and its label to
   `crates/app/ui/messages/en.json`;
2. handle the `id` in `AppState.run` (`crates/app/ui/src/lib/state/app.svelte.ts`).

The native menu (`crates/app/src/menu.rs`), the keyboard handler, the *Keyboard Shortcuts* panel and the
command palette pick it up automatically. Give a command `nativeAccelerator: true` only if its shortcut
never conflicts with text editing; shortcuts with `scope: "outsideText"` are ignored while typing.
Tests in both languages check ids, labels and accelerators.

## Logging policy

Logs use `tracing` and go to stderr only (`KEYRUST_LOG=debug` for more detail).

- At `info` and above: events, error kinds and counts only.
- **Never** log field values (passwords, titles, usernames, URLs, notes), master passwords, keys,
  or file contents, at any level.
- File paths only at `debug`.
- Types holding vault data must not implement `Display`, and their `Debug` must be redacted.

`crates/core/tests/logging_policy.rs` checks this for the core operations; extend it when you add
operations that touch user data.

By contributing you agree that your contributions are licensed under GPL-3.0-or-later.
