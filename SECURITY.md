# Security policy

> **Draft (Milestone 0).** Keyrust is pre-alpha and has not been audited. Do not use it to
> store real secrets yet.

## Reporting a vulnerability

Please **do not open a public issue**. Report vulnerabilities privately through GitHub's
[private vulnerability reporting](https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing-information-about-vulnerabilities/privately-reporting-a-security-vulnerability)
("Report a vulnerability" in the repository's *Security* tab).

Please include the affected version or commit, steps to reproduce and the impact you expect.
You can expect an acknowledgement within 7 days. Fixes are released as soon as practical, and
reporters are credited unless they prefer otherwise.

## Supported versions

| Version | Supported |
|---|---|
| `main` (pre-release) | ✅ best effort |
| anything else | ❌ |

## Threat model

### Assets
The contents of the user's KDBX database (passwords, usernames, notes, attachments), and the
master password that protects it.

### What Keyrust protects against
These are design goals; most of them are implemented in Milestone 3 (see [docs/TASKS.md](docs/TASKS.md)).

- **Theft or leak of the database file** (lost laptop, compromised cloud sync, backups): the file is
  encrypted per the KDBX4 specification, with the key derived by Argon2id using configurable,
  deliberately expensive parameters. Its security then rests on the strength of the master password.
- **Tampering with the file**: KDBX4 authenticates the header and the payload with HMAC; a modified
  file is rejected rather than silently opened.
- **Casual exposure while unlocked**: passwords are masked by default, the clipboard is cleared
  automatically after a short time, and the vault locks on inactivity and when the screen locks.
- **Secrets lingering in memory or leaking to logs**: secret values are held in `zeroize`/`secrecy`
  types and wiped when dropped; they are never logged, never written to disk in clear text, and the
  webview only receives secrets the user explicitly asks to reveal.
- **Data loss on save**: saves are atomic (temporary file in the same folder, fsync, rename). Before
  replacing the file Keyrust checks, twice, that it still holds exactly the bytes it last read or wrote
  (length and SHA-256); if another program changed it, the save is refused and you choose between
  *Reload* and *Keep mine*. Keyrust instances on the same computer also coordinate through a lock.
- **Resurrected deletions**: permanently deleted entries and groups are recorded as KDBX tombstones,
  so merging in another KeePass client does not bring them back.
- **Supply-chain and memory-safety bugs**, as far as practical: Rust with `unsafe_code` forbidden in
  our crates, no hand-written cryptography, pinned and audited dependencies (`cargo deny`), fuzzing
  of the parsing path.

### What Keyrust does not protect against
- **A compromised computer while the vault is unlocked**: malware running as your user (keyloggers,
  screen capture, memory readers, clipboard monitors) can read what you can read. No password manager
  can defend against this.
- **A weak or reused master password**: the KDF slows guessing down, it cannot make a weak password strong.
- **Physical access to an unlocked session.**
- **Memory forensics after the fact**: wiping is best effort. The operating system may swap memory to
  disk or produce crash dumps, and the webview's JavaScript engine does not guarantee that strings are
  erased.
- **Clipboard managers and history**: copying happens in Rust (the password never passes through the
  webview); Keyrust keeps only a hash of what it copied and clears the clipboard after the configured
  time, on lock and on exit, only if it still holds that value. It asks the platform to exclude copied secrets from clipboard
  history where supported, but cannot control third-party clipboard tools.
- **Sync conflicts and malicious sync providers**: a provider cannot read the file, but it can delete it
  or roll it back to an older version. Keep backups. A program that does not use Keyrust's lock
  (another KeePass client, a sync tool) can still write in the instant between Keyrust's last check and
  its rename; that window cannot be closed without operating-system support. Keyrust does not merge
  concurrent changes.
- **Side channels and a compromised build or update channel.** Releases will be signed; verify signatures.

## Security design rules

- No hand-written cryptographic primitives: only standard, maintained libraries (RustCrypto,
  keepass-rs).
- Argon2id for new databases, with explicit parameters (never library defaults).
- Secrets: `zeroize`/`secrecy`, never logged, never serialised in clear text, never in `Debug` output.
- Minimal Tauri capabilities and a strict Content Security Policy; no remote resources and no
  network access in v1.
