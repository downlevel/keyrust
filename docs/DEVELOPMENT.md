# Development guide

## Repository layout

```
Cargo.toml              Cargo workspace (default member: crates/core)
rust-toolchain.toml     pinned Rust toolchain (rustup installs it automatically)
crates/core/            keyrust-core: vault / KDBX4 layer, no UI dependencies
crates/core/tests/      KeePassXC compatibility tests (spike 0001)
crates/app/             keyrust-app: Tauri 2 crate (tauri.conf.json, capabilities/, icons/)
crates/app/ui/          SvelteKit (SPA) + Svelte 5 + TypeScript + Tailwind + shadcn-svelte
tests/fixtures/         KeePassXC-created test databases (test data only)
scripts/                helper scripts (fixture generation)
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
5. **KeePassXC** from <https://keepassxc.org>. Add its install folder (e.g. `C:\Program Files\KeePassXC`)
   to `PATH`, or set `KEEPASSXC_CLI=C:\Program Files\KeePassXC\keepassxc-cli.exe`.

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
pnpm check && pnpm lint && pnpm format:check && pnpm build

# Desktop app (from crates/app/ui; the script runs the Tauri CLI from crates/app)
pnpm tauri dev
pnpm tauri build
```

Without KeePassXC installed, the compatibility tests fail on purpose (a silent skip would look like
a verification). To skip them knowingly, set `KEYRUST_SKIP_KEEPASSXC=1`
(PowerShell: `$env:KEYRUST_SKIP_KEEPASSXC = "1"`).

Regenerate the KeePassXC fixtures (Linux/macOS/Git Bash): `scripts/gen-fixtures.sh`.

## Milestone 0 verification checklist (Windows)

The development machine used so far has no Tauri system libraries, so the desktop app has **not**
been compiled yet. Please run, on Windows, from the repository root:

- [ ] `cargo test -p keyrust-core` → 7 tests pass
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` → no warnings (first build of the app crate)
- [ ] `cd crates/app/ui; pnpm install; pnpm tauri dev` → a "Keyrust" window opens with the placeholder
      unlock card (disabled password field and button) and `core v0.0.1` at the bottom
- [ ] In the window, open DevTools (right click → Inspect, or Ctrl+Shift+I): the console shows **no
      Content Security Policy violations**
- [ ] `pnpm tauri build` → produces an installer under `target/release/bundle/`; install it, start it,
      and check the console again (the production CSP is stricter than the dev one)
- [ ] Optional: in the KeePassXC GUI create two databases with password `keyrust-test-password`
      (Database → Database Settings → Security → Encryption settings): one with **ChaCha20 + Argon2id**,
      one with **AES-256 + Argon2d**; send them back to be added as fixtures.
