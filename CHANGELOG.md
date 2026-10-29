# Changelog

All notable changes to Keyrust are documented in this file. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.1.0]

The first release: a minimal KeePass client for Linux, Windows and macOS. Alpha software, not
audited: keep backups of your database.

### Added

- Create, open and save KDBX 4 databases protected by a master password; new databases use
  Argon2id calibrated to about one second.
- KDBX 3.1 and 4.0 files open normally and are converted to KDBX 4.1 on the first save, with a
  notice when they are opened.
- Nested groups, entries with title, username, password, URL and notes, entry history and a
  recycle bin with restore. Custom fields, attachments and history written by other clients are
  preserved.
- Instant search over titles, usernames and URLs.
- Password generator with a strength estimate.
- Clipboard copy from the Rust process, cleared after a delay, on lock and on exit.
- Auto-lock on inactivity, screen lock, sleep and (optionally) minimize.
- Atomic saves with detection of changes made by another app (*Reload* or *Keep mine*).
- Database settings: name, cipher, key-derivation parameters, recycle bin, master password.
- Full keyboard use, a command palette and a shortcuts panel; accessibility checked in CI.

### Distribution

- Installers for Windows (NSIS and MSI), Linux (`.deb` and AppImage) and macOS (universal
  `.dmg`), with SHA-256 checksums signed with Sigstore cosign. See
  [Verifying releases](https://github.com/downlevel/keyrust/blob/main/SECURITY.md#verifying-releases).
- The installers are not code-signed for Windows or macOS: SmartScreen and Gatekeeper warn before
  the first launch (see the [README](https://github.com/downlevel/keyrust/blob/main/README.md#download)).

[Unreleased]: https://github.com/downlevel/keyrust/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/downlevel/keyrust/releases/tag/v0.1.0
