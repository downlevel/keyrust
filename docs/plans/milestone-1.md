# Piano — Milestone 1 (Infrastruttura) di Keyrust

Riferimenti: `docs/TASKS.md` (Milestone 1), `docs/ARCHITECTURE.md`, `docs/PRD.md`, `docs/plans/milestone-0.md`,
`docs/spikes/0001-keepass-rs-kdbx4.md`. Base: `main` @ `9c46045`.

## Obiettivo
Rendere ogni modifica verificata automaticamente su Linux, Windows e macOS (inclusa la build dell'app Tauri, che non si
può compilare sulla macchina di sviluppo) e predisporre le fondamenta trasversali: fuzzing, logging sicuro, i18n,
impostazioni persistenti.

## Contesto e vincoli (confermati)
- Macchina di sviluppo Linux con **~2 GB liberi**: nessuna build Tauri locale, niente toolchain nightly locale. La CI è
  la prova per tutto ciò che riguarda il crate app, il fuzzing e Windows/macOS.
- Repo `https://github.com/downlevel/keyrust` (pubblico). **Autorizzato**: push sul branch `milestone-1`.
  **Non autorizzato**: push su `main` (si passa da PR, aperta e mergiata dall'utente). Nessuna release.
- Lo stato della CI si legge dall'API pubblica GitHub (repo pubblico, senza credenziali).
- Decisioni utente: i18n con **Paraglide JS 2**; la CI pubblica le build debug **non firmate** dei tre OS come artifact (7 giorni).
- `keepassxc-cli db-create` (2.7.6) crea **KDBX 3.1 / AES-KDF**; `import` crea KDBX 4.1 / AES-KDF. Fixture Argon2/ChaCha20
  di KeePassXC restano possibili solo dalla GUI (task utente, già documentato).

## Criteri di accettazione
1. **CI** `.github/workflows/ci.yml`, trigger `push` (tutti i branch), `pull_request` verso `main`, `workflow_dispatch`;
   `permissions: contents: read`; `concurrency` che annulla run superati sullo stesso ref; ogni action **fissata a SHA
   di commit** (con il tag in commento). Job:
   - `core` (matrice ubuntu-24.04, windows-latest, macos-latest): installa KeePassXC (apt / `choco install keepassxc` /
     `brew install --cask keepassxc`), imposta `KEEPASSXC_CLI` dove non è nel PATH, esegue
     `cargo clippy -p keyrust-core --all-targets -- -D warnings` e `cargo test -p keyrust-core`
     (senza `KEYRUST_SKIP_KEEPASSXC`: se KeePassXC manca il job fallisce). `cargo fmt --all --check` solo su Linux.
   - `app` (stessa matrice): prerequisiti Tauri su Linux; Node + pnpm (versione da `packageManager`);
     `pnpm install --frozen-lockfile`, `pnpm check`, `pnpm lint`, `pnpm format:check`, `pnpm test`;
     `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test -p keyrust-app`;
     `pnpm tauri build --debug --bundles <nsis|dmg|deb>`; upload artifact `keyrust-<os>-debug` (retention 7 giorni).
   - `deny` (Linux): `cargo deny check` (advisories, bans, licenses, sources) tramite `EmbarkStudios/cargo-deny-action`.
   - `fuzz` (Linux): toolchain nightly, `cargo-fuzz` (versione fissata, `--locked`), esecuzione di ogni target per 60 s con
     corpus iniziale dalle fixture; i crash vengono caricati come artifact e fanno fallire il job.
   - Cache Rust con `Swatinem/rust-cache`, separata per job.
2. **Il primo run CI sul branch `milestone-1` è verde su tutti i job** e produce i tre artifact.
3. **Header KDBX4 (`keyrust_core::kdbx::header`)**: funzione pura `peek(data: &[u8]) -> Result<HeaderSummary, HeaderError>`
   che legge firma, versione (major/minor), cifrario esterno (UUID → AES-256 / ChaCha20 / Twofish / sconosciuto),
   compressione e parametri KDF (AES-KDF rounds; Argon2d/Argon2id memoria, iterazioni, parallelismo, versione) dalla
   VariantDictionary, **senza alcuna operazione crittografica** e senza leggere oltre l'header. Rifiuta input troncati,
   lunghezze incoerenti e versioni ≠ 4 con errori tipizzati; mai panic, mai allocazioni proporzionali a lunghezze
   dichiarate non verificate. Scopo: in Milestone 3 imporre limiti ai parametri KDF prima di aprire file non fidati
   (DoS via Argon2 con memoria enorme) e mostrare le impostazioni di cifratura.
   Test: valori di `peek` sulla fixture e sui DB creati in T3 coincidono con `keepassxc-cli db-info` (cifrario, KDF,
   round/memoria/iterazioni dove `db-info` li mostra) e con la configurazione usata per crearli; casi d'errore su input
   troncati/corrotti; KDBX 3.1 → errore `UnsupportedVersion`.
4. **Fuzzing** `fuzz/` (crate separato, escluso dal workspace): target `kdbx_header_peek` che chiama
   `keyrust_core::kdbx::header::peek` su input arbitrari; corpus iniziale `fuzz/corpus/kdbx_header_peek/` con le fixture.
   Un secondo target su `keepass::Database::parse` è rinviato alla Milestone 3, dopo che `peek` permetterà di limitare il
   KDF (motivazione nel README di `fuzz/`).
5. **Tracing**: `tracing` in `core` e `app`; l'app inizializza `tracing-subscriber` (stderr, filtro da `KEYRUST_LOG`,
   default `info`, nessun file di log). Policy scritta in `CONTRIBUTING.md` e `docs/ARCHITECTURE.md`: i log contengono
   eventi, tipi di errore e conteggi; **mai** valori di campi (password, titoli, username, URL, note), master password o
   chiavi; i percorsi di file solo a livello `debug`. I tipi del core che contengono dati del vault non implementano
   `Display` e il loro `Debug` è redatto (vale per i tipi introdotti da qui in avanti; oggi il core non ne ha).
   Verifica automatica in questa milestone: test con subscriber di cattura sulle operazioni di `settings` e
   `kdbx::header` che controlla che nessun valore delle impostazioni e nessun percorso compaia a livello `info` o
   superiore. Verifica sui segreti veri rinviata alla Milestone 3 (oggi non esistono flussi con segreti).
6. **Impostazioni (`keyrust_core::settings`)**: struttura `Settings` serializzata in JSON con campo `version: 1`,
   `recent_files` (max 10, senza duplicati, più recente prima), `clipboard_clear_secs` (default 20, range 5–300),
   `idle_lock_mins` (default 5, range 1–240), `lock_on_screen_lock` (true), `lock_on_minimize` (false), `generator`
   (lunghezza 20 nel range 8–128, minuscole/maiuscole/cifre/simboli true, `exclude_ambiguous` false, almeno un set attivo).
   Nessun campo segreto per costruzione. `load(path)`: file mancante → default; JSON invalido → default, il file corrotto
   viene rinominato `settings.json.corrupt` e si logga un warning senza contenuto; valori fuori range → riportati nel range;
   campi sconosciuti ignorati. `save(path)`: crea la directory, scrittura atomica (file temporaneo nella stessa directory,
   `fsync`, `rename`), nessun file temporaneo residuo. Test unitari per ognuno di questi casi.
   App: `Mutex<Settings>` nello stato Tauri, file in `app_config_dir()/settings.json`, comandi `get_settings` e
   `update_settings` (valida, salva, restituisce il valore effettivo); errori di I/O restituiti come messaggi senza percorsi.
7. **Sicurezza Tauri verificata da test** (`crates/app/tests/security_config.rs`, gira in CI): la capability `default`
   concede solo `core:default`; nessun plugin registrato in `tauri.conf.json`; la CSP di produzione ha `script-src 'self'`
   senza `unsafe-inline`/`unsafe-eval`, `default-src 'self'`, `object-src 'none'`, nessun host remoto (`http(s)://`
   diverso da `http://ipc.localhost`); `freezePrototype: true`. I comandi IPC registrati sono esattamente
   `app_info`, `get_settings`, `update_settings`.
8. **i18n (Paraglide JS 2)**: progetto inlang in `crates/app/ui/project.inlang/` (base locale `en`, unica lingua `en`),
   messaggi in `crates/app/ui/messages/en.json`, plugin Vite con output in `src/lib/paraglide/` (generato, ignorato da git),
   strategia locale senza URL (`baseLocale`; predisposta per `preferredLanguage` quando arriverà l'italiano).
   Tutte le stringhe visibili della schermata placeholder passano da `m.*()`. Test Vitest: i messaggi usati esistono e
   restituiscono il testo inglese atteso; un test fallisce se `src/routes/**/*.svelte` contiene testo visibile
   letterale fuori da `m.*()` (controllo semplice sul markup, documentato come euristico).
9. **Fixture**: nuova fixture `tests/fixtures/keepassxc-kdbx31.kdbx` creata da `keepassxc-cli db-create` + `add`
   (KDBX 3.1, AES-KDF) generata da `scripts/gen-fixtures.sh`; test **T5**: keepass-rs la apre e legge la voce (risponde
   in parte alla domanda aperta del PRD sul supporto KDBX 3.1 in lettura). `tests/fixtures/README.md` aggiornato.
10. **Documentazione**: `docs/TASKS.md` (Milestone 1 spuntata con note), `docs/DEVELOPMENT.md` (CI, artifact, fuzzing,
    variabili `KEYRUST_LOG`), `docs/ARCHITECTURE.md` (i18n = Paraglide, header peek, policy tracing), `CONTRIBUTING.md`
    (policy log), README con badge CI.

## Approccio e decisioni
- **Toolchain in CI**: `rustup` è preinstallato sui runner; `rust-toolchain.toml` fa installare `1.99.1` alla prima
  invocazione di cargo. Nightly solo nel job `fuzz` (`cargo +nightly fuzz`).
- **KeePassXC in CI**: le versioni differiscono tra OS (Ubuntu 2.7.6, choco/brew più recenti); i test non dipendono dalla
  versione salvo l'etichetta del cifrario in `db-info` (oggi `AES 256-bit` / `ChaCha20 256-bit`): se una versione diversa
  usa etichette diverse, il test si adegua accettando le varianti documentate, non viene indebolito.
- **Bundle CI**: `--debug` per velocità; Windows `nsis`, macOS `dmg`, Linux `deb`. Non firmati, da non distribuire.
- **Header parser scritto a mano**: è parsing di formato, non crittografia (consentito dalle regole del brief); il parser
  è difensivo (lunghezze controllate prima di ogni slice, limite massimo di dimensione dell'header) e fuzzato.
- **Settings nel core** (non nell'app): logica pura e testabile qui; l'app fa solo da adattatore Tauri.
- **Paraglide**: setup manuale (non `sv add`, che aggiunge hook server inutili in una SPA). Versione esatta nel lockfile.
- Le action GitHub sono fissate a SHA risolti dall'API GitHub al momento della build e annotati col tag.

## Non-goals
- Release, firma del codice, updater, build riproducibili (Milestone 4).
- UI delle impostazioni, traduzione italiana, design tokens (Milestone 2+).
- Apertura vault, clipboard, auto-lock, fuzzing di `Database::parse` (Milestone 3).
- Protezione del branch `main` (azione dell'utente su GitHub).

## Rischi
- `keepassxc-cli` su Windows/macOS potrebbe gestire diversamente la password da stdin o i percorsi: emergerà al primo run;
  le correzioni restano nel perimetro (helper di test), senza indebolire le asserzioni.
- Tempi CI: build Tauri × 3 OS ~10–20 min; accettabile, mitigato dalla cache.
- `brew --cask` / `choco` possono cambiare percorso d'installazione: `KEEPASSXC_CLI` impostato esplicitamente e verificato
  con `--version` nel job prima dei test.
- Il controllo "stringhe non tradotte" è euristico: può avere falsi positivi/negativi; documentato.

## Verifica
Locale (Linux, host):
```
cargo fmt --all --check
cargo clippy -p keyrust-core --all-targets -- -D warnings
cargo test -p keyrust-core                       # T0–T5 + header + settings + tracing policy
cargo deny check
(cd fuzz && cargo check)                         # il target compila (stable); l'esecuzione è in CI
(cd crates/app/ui && pnpm install --frozen-lockfile && pnpm check && pnpm lint && pnpm format:check && pnpm test && pnpm build)
python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/ci.yml'))"   # sintassi del workflow
bash scripts/gen-fixtures.sh && cargo test -p keyrust-core   # fixture rigenerate restano verdi
```
Remota (prova principale per app/Windows/macOS/fuzz):
- push di `milestone-1`; tramite `https://api.github.com/repos/downlevel/keyrust/actions/runs?branch=milestone-1` il run
  è `completed/success`, tutti i job (`core` ×3, `app` ×3, `deny`, `fuzz`) `success`, e
  `.../runs/<id>/artifacts` elenca `keyrust-windows-debug`, `keyrust-macos-debug`, `keyrust-linux-debug`.
- Manuale (utente, Windows): scaricare l'artifact Windows, installarlo, verificare la finestra e la console senza
  violazioni CSP (checklist in `docs/DEVELOPMENT.md`).

## Toolchain per provider
- Builder (Claude, host): Rust 1.99.1, pnpm 12.15, keepassxc-cli 2.7.6, cargo-deny 0.20.2; rete verso crates.io, npm,
  api.github.com; push via SSH (account `downlevel`).
- Reviewer/Inspector (Codex, sandbox read-only): legge file e diff con comandi di sola lettura; non esegue i comandi di
  prova, build o test (li esegue l'host).
