# Keyrust

**A modern KeePass client, written in Rust.**

Keyrust is an open source desktop password manager for Linux, Windows and macOS. It reads and
writes the standard **KDBX4** format, so your database stays a plain file you own: open it with
KeePassXC, KeePassDX (Android) or Strongbox (iOS), and sync it with Syncthing, Nextcloud or
Dropbox. No account, no cloud, no telemetry.

> **Status: pre-alpha.** The project is in Milestone 0 (setup). There is no usable vault yet;
> do not store real passwords in it.

## Planned features (v1)

- Create, open and save KDBX4 databases protected by a master password (Argon2id).
- Entries (title, username, password, URL, notes) organised in groups; recycle bin.
- Instant search, configurable password generator.
- Copy to clipboard with automatic clearing; auto-lock on inactivity and screen lock.
- Two-way compatibility with KeePassXC, verified by automated tests.
- Full keyboard navigation.

Out of scope: auto-type, browser extension, a sync server of our own. See [docs/PRD.md](docs/PRD.md).

## Why another KeePass client?

KeePassXC is excellent and battle-tested; Keyrust aims at a calmer, more minimal everyday
experience on top of the same file format, with memory-safe code and visible engineering
quality: a written threat model ([SECURITY.md](SECURITY.md)), compatibility tests against
KeePassXC itself, fuzzing and CI on three operating systems.

## Architecture

```
crates/core   vault, crypto and KDBX4 format (no UI dependencies) — keepass-rs, RustCrypto
crates/app    Tauri 2 desktop shell + SvelteKit / Svelte 5 UI (crates/app/ui)
```

Secrets stay on the Rust side: the webview only receives what it displays, and copying a
password to the clipboard happens in Rust. Details in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

The KDBX4 compatibility of keepass-rs with KeePassXC has been verified in
[spike 0001](docs/spikes/0001-keepass-rs-kdbx4.md).

## Building

See [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) for prerequisites on each OS. In short:

```sh
cargo test -p keyrust-core            # core tests (KeePassXC compatibility needs keepassxc-cli)
cd crates/app/ui && pnpm install
pnpm tauri dev                        # run the desktop app
```

## Documentation

- [Product requirements](docs/PRD.md) · [Architecture](docs/ARCHITECTURE.md) · [Design](docs/DESIGN.md)
- [Roadmap](docs/TASKS.md) · [Development guide](docs/DEVELOPMENT.md) · [Contributing](CONTRIBUTING.md)
- [Security policy and threat model](SECURITY.md)

## License

Copyright © 2026 Mauro Giulivi. Licensed under the [GNU General Public License v3.0 or later](LICENSE).
