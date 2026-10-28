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
  automatically after a short time, and the vault locks automatically. The lock is decided and done in
  the Rust process (the vault is dropped and zeroized there, even if the webview hangs):
  - after the configured minutes of inactivity (keyboard, pointer and wheel input in the window);
  - when the screen locks (option, on by default): Windows and macOS poll the session state every 2 s,
    Linux listens to the desktop's `ScreenSaver.ActiveChanged` signal on D-Bus;
  - when the computer goes to sleep, always: Windows power notification, Linux logind
    `PrepareForSleep`, and everywhere a detector of wall-clock jumps (the only one on macOS);
  - when the window is minimized (option, off by default).

  An unlock still deriving the key when a lock arrives is discarded, so it cannot undo the lock. Like
  the manual Lock, an automatic lock asks nothing: changes that could not be saved are lost, and the
  Unlock screen says so.
- **Hostile files from a sync provider**: the key-derivation parameters are read before anything is
  authenticated, so a tampered file could ask for any amount of memory or time. Keyrust checks them
  before deriving the key and refuses files beyond Argon2 2 GiB of memory, 1000 iterations or 256 lanes,
  2·10⁹ AES-KDF rounds, or a file size of 512 MiB ("This database asks for more memory or time than
  Keyrust allows"). The same limits bound what Keyrust itself lets you create, and a save that would
  write a file beyond the size limit is refused (the file on disk stays as it was). Reads are bounded
  too, including the checks for external changes.
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
  of the parsing path (whole files and save/open round trips, see [fuzz/README.md](fuzz/README.md)).
  The only `unsafe` code is in `crates/os/src/lib.rs`, a few FFI calls: whether the screen is locked
  (Windows `OpenInputDesktop`, macOS `CGSessionCopyCurrentDictionary`) and the Windows suspend
  notification (`PowerRegisterSuspendResumeNotification`). There it is denied crate-wide and allowed
  function by function, each block with a `SAFETY:` comment (enforced by clippy).

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
- **Auto-lock gaps**: screen-lock detection on Linux depends on the desktop sending the D-Bus signal
  (GNOME, KDE, MATE and Cinnamon do; some minimal window managers do not). On macOS sleep is detected
  from a jump of the clock, so a sleep and wake cycle shorter than 4 seconds, with "require password
  after sleep" turned off, is not noticed. Setting the clock forward by more than 4 seconds locks
  needlessly (the safe side). The idle timer only counts input inside Keyrust's window.
- **Decompression bombs inside an authenticated file**: keepass-rs decompresses the content only after
  verifying its HMAC with your key, so such a file can only be made by someone who has your master
  password, and with it they already have everything. A decompression cap is to be requested upstream.
- **Side channels and a compromised build or update channel.** Releases will be signed; verify signatures.

## Security design rules

- No hand-written cryptographic primitives: only standard, maintained libraries (RustCrypto,
  keepass-rs).
- Argon2id for new databases, with explicit parameters (never library defaults).
- Secrets: `zeroize`/`secrecy`, never logged, never serialised in clear text, never in `Debug` output.
- Minimal Tauri capabilities and a strict Content Security Policy; no remote resources and no
  network access in v1.
