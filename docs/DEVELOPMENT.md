# Development guide

## Repository layout

```
Cargo.toml              Cargo workspace (default member: crates/core)
rust-toolchain.toml     pinned Rust toolchain (rustup installs it automatically)
crates/core/            keyrust-core: vault / KDBX4 layer, no UI dependencies
crates/core/tests/      KeePassXC compatibility and logging-policy tests
crates/app/tests/       Tauri security configuration tests
crates/app/             keyrust-app: Tauri 2 crate (tauri.conf.json, capabilities/, icons/)
crates/app/ui/          SvelteKit (SPA) + Svelte 5 + TypeScript + Tailwind + shadcn-svelte
tests/fixtures/         KeePassXC-created test databases (test data only)
scripts/                helper scripts (fixture generation)
fuzz/                   cargo-fuzz targets (separate workspace, nightly)
.github/workflows/      CI
docs/                   PRD, architecture, design, roadmap, plans, spikes
```

`cargo build` / `cargo test` at the root only build `crates/core`, so they work on any machine
without the Tauri system libraries. Build the app explicitly with `-p keyrust-app` or through
`pnpm tauri`.

## Prerequisites

| Tool | Version | Notes |
|---|---|---|
| Rust (rustup) | pinned by `rust-toolchain.toml` (1.99.1) | rustup installs it on first use |
| Node.js | 22+ | |
| pnpm | 12 (via `corepack enable pnpm`) | version pinned in `crates/app/ui/package.json` |
| KeePassXC | 2.7+ | provides `keepassxc-cli` for the compatibility tests |
| cargo-deny | latest | `cargo install cargo-deny --locked` |

Plus the Tauri system prerequisites for your OS: <https://v2.tauri.app/start/prerequisites/>.

### Windows

1. **Microsoft C++ Build Tools** (Visual Studio Installer → "Desktop development with C++").
2. **WebView2**: preinstalled on Windows 10 (recent updates) and 11.
3. **Rust** via <https://rustup.rs> (default MSVC toolchain).
4. **Node.js** LTS, then in a terminal: `corepack enable pnpm`.
5. **KeePassXC** (not KeePass 2: the tests use KeePassXC's `keepassxc-cli` as an independent
   reference): `winget install --id KeePassXCTeam.KeePassXC -e`. The tests find it in
   `C:\Program Files\KeePassXC\`; for another location set `KEEPASSXC_CLI` to the full path of
   `keepassxc-cli.exe`.

### Linux (Debian/Ubuntu)

```sh
sudo apt install build-essential libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev \
    libayatana-appindicator3-dev libxdo-dev libssl-dev keepassxc
```

(Only `keepassxc` is needed to work on `crates/core`.)

## Everyday commands

```sh
# Core: format, lint, tests (the KeePassXC compatibility tests fail if keepassxc-cli is missing)
cargo fmt --all --check
cargo clippy -p keyrust-core --all-targets -- -D warnings
cargo test -p keyrust-core

# Dependency audit (licences, advisories, sources) for the whole workspace
cargo deny check

# Frontend
cd crates/app/ui
pnpm install
pnpm check && pnpm lint && pnpm format:check && pnpm test && pnpm build

# Desktop app (from crates/app/ui; the script runs the Tauri CLI from crates/app)
pnpm tauri dev
pnpm tauri build
```

Without KeePassXC installed, the compatibility tests fail on purpose (a silent skip would look like
a verification). To skip them knowingly, set `KEYRUST_SKIP_KEEPASSXC=1`
(PowerShell: `$env:KEYRUST_SKIP_KEEPASSXC = "1"`).

Regenerate the KeePassXC fixtures (Linux/macOS/Git Bash): `scripts/gen-fixtures.sh`.

### Logs

The app logs to stderr only. `KEYRUST_LOG=debug pnpm tauri dev` shows more detail (still no
secrets; see the logging policy in [CONTRIBUTING.md](../CONTRIBUTING.md)).

### Translations

UI strings live in `crates/app/ui/messages/en.json` and are compiled by Paraglide into
`src/lib/paraglide/` (generated, not committed) by `pnpm check`, `pnpm test` and the Vite build.

### Fuzzing

See [fuzz/README.md](../fuzz/README.md) (needs a nightly toolchain).

## Continuous integration

`.github/workflows/ci.yml` runs on every push (except documentation-only changes), on pull requests
to `main` and manually (*Actions* → *CI* → *Run workflow*):

| Job | What it does |
|---|---|
| `core` (Linux, Windows, macOS) | installs KeePassXC; fmt (Linux), clippy and tests of `keyrust-core` |
| `app` (Linux, Windows, macOS) | frontend check/lint/format/test/build; workspace clippy; `keyrust-app` tests (security configuration); debug build of the desktop app |
| `deny` | `cargo deny check` (licences, advisories, sources) |
| `fuzz` | every cargo-fuzz target for 60 s |

The desktop build is the slowest step, so it runs only for pull requests, pushes to `main` and manual
runs; a plain push to a branch stops after the tests. Those runs upload an **unsigned debug build** as
an artifact, kept 7 days:
`keyrust-windows-debug` (NSIS installer), `keyrust-macos-debug` (dmg), `keyrust-linux-debug` (deb).
Download it from the run page on GitHub (*Actions* → run → *Artifacts*). They are for testing only:
Windows SmartScreen and macOS Gatekeeper will warn because they are not signed (on macOS:
right click → Open).

## Desktop app verification checklist (Windows)

The development machine has no Tauri system libraries, so the app is compiled only by CI. On Windows:

- [ ] Download `keyrust-windows-debug` from the latest green CI run and run the installer
      (SmartScreen: *More info* → *Run anyway*)
- [ ] A "Keyrust" window opens with the placeholder unlock card (disabled password field and button)
      and `core v0.0.1` at the bottom
- [ ] Open DevTools (right click → Inspect, or Ctrl+Shift+I): the console shows **no Content Security
      Policy violations**
- [ ] Optional, from source: `cargo test -p keyrust-core` (with KeePassXC installed) and
      `cd crates/app/ui; pnpm install; pnpm tauri dev`
- [ ] Optional: in the KeePassXC GUI create two databases with password `keyrust-test-password`
      (Database → Database Settings → Security → Encryption settings): one with **ChaCha20 + Argon2id**,
      one with **AES-256 + Argon2d**; send them back to be added as fixtures.
