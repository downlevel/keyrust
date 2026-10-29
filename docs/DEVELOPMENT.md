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
scripts/                helper scripts (fixture generation, release checks)
fuzz/                   cargo-fuzz targets (separate workspace, nightly)
.github/workflows/      CI (ci.yml) and releases (release.yml)
docs/                   PRD, architecture, design, roadmap, plans, spikes
```

`cargo build` / `cargo test` at the root only build `crates/core`, so they work on any machine
without the Tauri system libraries. Build the app explicitly with `-p keyrust-app` or through
`pnpm tauri`.

## Prerequisites

| Tool | Version | Notes |
|---|---|---|
| Rust (rustup) | pinned by `rust-toolchain.toml` (1.99.1) | rustup installs it on first use |
| Node.js | 24 (CI and releases use the exact version in `crates/app/ui/.node-version`) | |
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

### Accessibility audit and screenshots

The audit (axe, keyboard, focus, reduced motion; see [ACCESSIBILITY.md](ACCESSIBILITY.md)) drives the
demo build in Chromium through Playwright:

```sh
cd crates/app/ui
pnpm exec playwright install chromium   # once; CI adds --with-deps
pnpm test:web                           # builds, serves on 127.0.0.1:4173, runs e2e-web/
pnpm test:web keyboard                  # one file
pnpm screenshots                        # regenerates docs/screenshots/*.png (never in CI)
```

Failures leave a trace in `crates/app/ui/test-results/` (`pnpm exec playwright show-trace …`).
`KEYRUST_SCREENSHOT_SCALE=1.5 pnpm screenshots` makes smaller PNGs if one exceeds 400 KB.

### End-to-end test of the desktop app

`crates/app/e2e` drives the real app (Rust backend, IPC, WebKitGTK) with WebdriverIO and
[tauri-driver](https://v2.tauri.app/develop/tests/webdriver/). It runs in the CI `e2e` job on Linux only:
tauri-driver does not support macOS, and on Windows it needs a matching msedgedriver. It opens copies
of the fixtures from a pre-filled recent-files list in a temporary `XDG_CONFIG_HOME`; the app has no test
hook. On a Linux machine with the Tauri prerequisites:

```sh
sudo apt-get install webkit2gtk-driver xvfb dbus
cargo install tauri-driver --locked --version 2.1.0
(cd crates/app/ui && pnpm install && pnpm tauri build --debug --no-bundle)
pnpm --dir crates/app/e2e install
xvfb-run -a dbus-run-session -- pnpm --dir crates/app/e2e test    # KEYRUST_E2E_DIR chooses the work folder
```

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
| `app` (Linux, Windows, macOS) | frontend check/lint/format/test/build; accessibility audit (Linux); workspace clippy; `keyrust-app` tests (security configuration); debug build of the desktop app |
| `e2e` (Linux) | builds the app, runs the smoke test under Xvfb, then checks the saved file with `keepassxc-cli` |
| `deny` | `cargo deny check` (licences, advisories, sources) |
| `fuzz` | every cargo-fuzz target for 60 s |

The desktop build is the slowest step, so it runs only for pull requests, pushes to `main` and manual
runs; a plain push to a branch stops after the tests. Those runs upload an **unsigned debug build** as
an artifact, kept 7 days:
`keyrust-windows-debug` (NSIS installer), `keyrust-macos-debug` (dmg), `keyrust-linux-debug` (deb).
Download it from the run page on GitHub (*Actions* → run → *Artifacts*). They are for testing only:
Windows SmartScreen and macOS Gatekeeper will warn because they are not signed (on macOS:
right click → Open).

Releases are built by a separate workflow, `.github/workflows/release.yml`: see
[RELEASING.md](RELEASING.md).

## Web preview

The UI runs in a plain browser too: outside Tauri it uses the demo vault and in-memory settings
(`src/lib/api/index.ts`). `cd crates/app/ui && pnpm dev` serves it at http://127.0.0.1:1420; `pnpm build`
writes the static site to `crates/app/ui/build/`, which is what gets published as the private preview
page used to review the design.

## Desktop app verification checklist

The development machine has no Tauri system libraries, so the app is compiled only by CI (installers
on pull requests, pushes to `main` and manual runs). With an installer from a green run
(Windows SmartScreen: *More info* → *Run anyway*; macOS: right click → *Open*):

- [ ] First run shows the Welcome screen (no demo notice). *Create new database* → *Choose…* opens the
      native save dialog; the database is created and opens empty
- [ ] Add a few entries and a group, lock (Ctrl+L), unlock again: everything is there; the file shows up
      under *Recent*
- [ ] The same file opens in KeePassXC with the same entries; an entry edited in KeePassXC while
      Keyrust is open brings up the "changed by another app" banner when you switch back to Keyrust
      (Reload shows KeePassXC's change, Keep mine keeps yours)
- [ ] *Copy password* puts the password in the clipboard, and it is gone after the configured seconds
      (unless you copied something else meanwhile); locking clears it too
- [ ] Closing the window with the editor open, or after a failed save, asks before quitting
- [ ] An entry URL opens in the system browser
- [ ] DevTools (right click → Inspect, or Ctrl+Shift+I): the console shows **no Content Security
      Policy violations**
- [ ] The native menu has File / Edit / Entry / Group / Tools / Help (on macOS also the Keyrust menu);
      *Entry → New Entry* and *Help → Keyboard Shortcuts* act like Ctrl+N and Ctrl+/
- [ ] Copy and paste work inside text fields (Ctrl+C / Ctrl+V, or ⌘C / ⌘V on macOS), and Ctrl+C (⌘C)
      with an entry selected in the list copies its password
- [ ] In DevTools, `window.__TAURI_INTERNALS__.invoke("plugin:dialog|open")` is **rejected** (the webview
      has no plugin permission)
- [ ] Settings → General values survive a restart (they are the real `settings.json`)
- [ ] Auto-lock, with a database open:
  - [ ] Settings → General → lock after **1 minute**, then leave Keyrust alone: it locks and the Unlock
        screen says "Locked after 1 minute of inactivity."; moving the mouse over the window keeps it open
  - [ ] Lock the screen (Windows: Win+L; macOS: Ctrl+⌘+Q; Linux: the desktop's lock) and unlock it:
        Keyrust is locked, "Locked because the screen was locked."
  - [ ] Sleep for a few seconds and wake up, with "require password after sleep" off if possible:
        "Locked when the computer went to sleep."
  - [ ] With *Lock when minimized* on, minimizing locks; with it off, it does not
  - [ ] Copied password: an automatic lock clears the clipboard like the manual one
- [ ] A database beyond the open limits (e.g. made in KeePassXC with Argon2 memory above 2 GiB) does not
      open and shows "This database asks for more memory or time than Keyrust allows."
- [ ] Optional: the same database, synced, opens and saves in KeePassDX (Android) and Strongbox (iOS)
- [ ] Optional, from source: `cargo test -p keyrust-core` (with KeePassXC installed) and
      `cd crates/app/ui; pnpm install; pnpm tauri dev`
- [ ] Optional: in the KeePassXC GUI create two databases with password `keyrust-test-password`
      (Database → Database Settings → Security → Encryption settings): one with **ChaCha20 + Argon2id**,
      one with **AES-256 + Argon2d**; send them back to be added as fixtures.

## Manual accessibility checklist

On Windows, with tools already present (no installation): run `pnpm tauri dev` or an installer from
CI. Record the result in [ACCESSIBILITY.md](ACCESSIBILITY.md#manual-checklist).

- [ ] Start Narrator (Ctrl+Win+Enter). On the Unlock screen it reads the database name and the
      "Master password" field; a wrong password is announced as an error
- [ ] Unlock with the keyboard only. In the vault, Tab goes sidebar → search → list → details; in the
      group tree Narrator reads each group with its count ("Work, 3 entries"), in the list each entry
      as an option with its position
- [ ] Select an entry and copy its password (Ctrl+C): Narrator announces "Password copied. The
      clipboard clears in 20 seconds." once, not every second
- [ ] Type in the search field: the number of results is announced after a pause
- [ ] Open the editor (Enter or Ctrl+E): every field has its label; the password field is described by
      its strength; the show/hide button says whether it is pressed
- [ ] Open Settings (Ctrl+,): the focus moves into it, Tab stays inside, Escape closes it and returns
      the focus where it was
- [ ] Zoom to 200% (Ctrl+plus, four times): nothing is cut off and no content needs horizontal
      scrolling to be read; Ctrl+0 restores
- [ ] Windows high contrast (Settings → Accessibility → Contrast themes), observation only: text,
      focus and selection stay visible and the app stays usable
