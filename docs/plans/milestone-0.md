# Piano — Milestone 0 (Setup) di Keyrust

Riferimenti: `BRIEF.md`, `docs/PRD.md`, `docs/ARCHITECTURE.md`, `docs/DESIGN.md`, `docs/TASKS.md` (Milestone 0).

## Obiettivo
Portare il repository da "solo documenti" a un workspace Rust + Tauri pronto per lo sviluppo, e **verificare
con evidenza riproducibile** che il crate `keepass` (keepass-rs) legga e scriva KDBX4 in modo compatibile e
lossless con KeePassXC, il rischio tecnico principale del progetto.

## Contesto e vincoli d'ambiente (confermati)
- Macchina di sviluppo: Ubuntu 24.04 x86_64, **4.1 GB liberi**. Installati: Rust 1.99.1 (rustup, profilo minimal
  + clippy/rustfmt), pnpm 12.15.0 (corepack), Node 24.13, keepassxc-cli 2.7.6 (apt, no-recommends), git 2.43.
- **Decisione utente**: si sviluppa su questa macchina; i test della GUI Tauri li fa l'utente sulla sua macchina
  **Windows** (dove non può usare Claude/Codex). Qui **non** si installano le librerie webkit2gtk/GTK e **non** si
  compila il crate Tauri (anche per mancanza di spazio).
- Autore commit / copyright: Mauro Giulivi (`maurogiulivi@gmail.com`), configurato solo come git config locale del repo.
- Autorizzati: git init + commit locali. **Non** autorizzati: creazione repo GitHub, push, release.

## Criteri di accettazione (osservabili)
1. Repo git inizializzato con un commit baseline (BRIEF + docs) e uno o più commit dell'implementazione, attribuiti come sopra.
2. Workspace Cargo alla root con membri `crates/core` (package `keyrust-core`) e `crates/app` (package `keyrust-app`,
   binario `keyrust`); `default-members = ["crates/core"]` così `cargo build/test` funzionano su Linux senza librerie GTK.
   `rust-toolchain.toml` fissa la toolchain `1.99.1` con componenti `rustfmt`, `clippy`. `Cargo.lock` committato.
   Licenza `GPL-3.0-or-later` in ogni `Cargo.toml` (via `workspace.package`).
3. File di progetto: `LICENSE` (testo integrale GPL-3.0 da gnu.org), `README.md` (scheletro: pitch, stato "pre-alpha",
   feature pianificate, link a docs, build), `CONTRIBUTING.md`, `SECURITY.md` (bozza: threat model protegge/non protegge,
   policy di segnalazione tramite GitHub private vulnerability reporting, versioni supportate), `.editorconfig`,
   `.gitignore`, `.gitattributes` (fixture `.kdbx` binarie, `eol=lf` per testo), `docs/DEVELOPMENT.md` con le istruzioni Windows.
4. `.gitignore` ignora `target/`, `node_modules/`, `dist/`, `.env*`, `*.kdbx.tmp` e **ogni `*.kdbx` tranne
   `tests/fixtures/**`** (protezione contro il commit accidentale di un vault reale).
5. **Spike keepass-rs** (dettagli sotto) con test automatici che passano qui con `KEYRUST_REQUIRE_KEEPASSXC=1`, e un
   rapporto `docs/spikes/0001-keepass-rs-kdbx4.md` con esito PASS/GAP/UNVERIFIED per ogni capability, la versione testata e una decisione.
6. Scaffold Tauri 2 in `crates/app` (Rust) + frontend in `crates/app/ui` (Svelte 5 + TypeScript + Vite, pnpm),
   Tailwind CSS v4 (`@tailwindcss/vite`), shadcn-svelte inizializzato con base color **zinc** e almeno i componenti
   `button` e `input`, font Inter e JetBrains Mono bundlati via `@fontsource*`, icone `@lucide/svelte`.
   Una schermata placeholder (wordmark "Keyrust", input password disabilitato con label "Master password",
   pulsante "Unlock" disabilitato, testo "Pre-alpha: database support coming in Milestone 3") che invoca un comando
   Tauri `app_info` e mostra la versione restituita da `keyrust_core::VERSION`. Nessun'altra logica.
7. Sicurezza dello scaffold: `tauri.conf.json` con CSP restrittiva (`default-src 'self'`; niente risorse remote,
   `connect-src` solo IPC Tauri), capability `default` con i soli permessi `core:default` (nessun plugin opener/fs/shell),
   nessun font o CDN remoto. Identificatore `io.github.maurogiulivi.keyrust`.
8. Config qualità: `rustfmt.toml`, lint workspace in `Cargo.toml` (`[workspace.lints]`: `unsafe_code = "forbid"`,
   clippy `unwrap_used`/`expect_used` = warn fuori dai test), `deny.toml` (licenze compatibili con GPL-3.0, advisories,
   sorgenti solo crates.io), ESLint flat config (eslint-plugin-svelte + typescript-eslint) e Prettier
   (prettier-plugin-svelte, prettier-plugin-tailwindcss) nel frontend con script `lint`, `format`, `format:check`, `check`.
9. `docs/TASKS.md`: spuntati i task di Milestone 0 completati e verificati; i task che richiedono verifica su Windows
   restano aperti o sono annotati "(da verificare su Windows)".

## Spike keepass-rs — approccio
Versione: `keepass = { version = "=0.17.0", features = ["save_kdbx4"] }` (MIT). Il README upstream dichiara la
scrittura **KDBX4.1 "experimental"**: è esattamente ciò che lo spike deve misurare.

Lo spike vive nel crate `core` come **integration test** (`crates/core/tests/`), senza aggiungere API pubbliche
definitive a `core` (l'API reale è Milestone 3). In `crates/core/src/lib.rs` resta solo `pub const VERSION`
più eventuali moduli già marcati `#[doc(hidden)]`/test-only se servono helper condivisi; preferibile un modulo
helper in `crates/core/tests/common/mod.rs`.

**Oracolo indipendente: KeePassXC.** Le verifiche non si fidano di keepass-rs per controllare sé stesso:
il contenuto di un file viene letto tramite `keepassxc-cli export --format xml` e confrontato semanticamente
con un parser XML (`roxmltree`, dev-dependency, portabile su Windows).

### Fixture (create da KeePassXC)
- Sorgente: `tests/fixtures/src/rich.xml`, un XML KeePass 2 scritto a mano, con: gruppi annidati (≥ 2 livelli),
  voce con tutti i campi standard, note multilinea e Unicode (es. emoji, accenti, CJK), **campi custom** (uno
  protetto e uno no), **tag**, **allegati** (uno testuale, uno binario non-UTF8), **cronologia** (≥ 2 versioni
  precedenti con valori diversi), **cestino** abilitato con una voce eliminata, `CustomData` popolato su voce e DB,
  **AutoType** popolato su una voce (va preservato anche se fuori scope in UI), almeno un **tombstone `DeletedObjects`**
  con UUID e DeletionTime noti, icona custom (opzionale: se l'import non la supporta è registrata come UNVERIFIED).
- Generazione: `scripts/gen-fixtures.sh` esegue `keepassxc-cli import -p` (password di test `keyrust-test-password`
  passata via stdin, documentata come NON segreta) producendo `tests/fixtures/keepassxc-rich.kdbx`. Il `.kdbx` generato
  è committato, così i test non dipendono dalla generazione. Si registra in un file `tests/fixtures/README.md` la versione
  di KeePassXC usata e i parametri KDF/cipher risultanti (letti con `keepassxc-cli db-info`).
- Limite noto: `keepassxc-cli import` non permette di scegliere KDF/cipher; le combinazioni Argon2id/ChaCha20 nella
  direzione KeePassXC → Keyrust vengono coperte da file creati con la GUI di KeePassXC su Windows (task aperto,
  documentato), mentre qui sono coperte nella direzione Keyrust → KeePassXC.

### Test (in `crates/core/tests/compat_keepassxc.rs`)
- **T0 Validazione fixture (KeePassXC import)**: prima di qualsiasi test di round-trip, `keepassxc-cli export` della
  fixture viene confrontato con **valori attesi espliciti** per ogni capability (gli stessi definiti in `rich.xml`):
  gruppi, campi standard, campi custom e flag protetto, tag, allegati, cronologia, cestino, CustomData (voce e DB),
  AutoType, tombstone DeletedObjects, icona custom. Una capability che l'import di KeePassXC non conserva viene
  registrata nel rapporto come **UNVERIFIED** (non PASS) e la sua copertura nei test successivi è dichiarata assente;
  per CustomData, AutoType e DeletedObjects si cerca una fonte di fixture alternativa (es. file creato con la GUI di
  KeePassXC su Windows) annotata come task aperto. Nessuna capability può risultare PASS in T2 se T0 non ne ha
  verificato la presenza nella fixture.
- **T1 Lettura (KeePassXC → keepass-rs)**: apre la fixture con keepass-rs e verifica tramite asserzioni esplicite
  i valori attesi definiti nel test (non derivati da keepass-rs): albero gruppi, campi standard, campi custom e flag
  protetto, tag, bytes degli allegati, numero e contenuto delle versioni di cronologia, cestino, CustomData, AutoType,
  tombstone DeletedObjects (UUID e DeletionTime), limitatamente alle capability verificate da T0.
- **T2 Round-trip lossless**: apre la fixture con keepass-rs → `save` in un file temporaneo → `keepassxc-cli export`
  sia dell'originale sia del risalvato → confronto semantico dei due XML. Il confronto copre: Meta (nome, descrizione,
  RecycleBinEnabled/UUID, CustomData, icone custom), albero gruppi (UUID, nome, note, icona, Times), voci (UUID, tutte
  le String con valore e flag protetto, Binary risolti per contenuto, Tags, Times, AutoType, CustomData, History ricorsiva),
  DeletedObjects (asserendo che il tombstone noto sia presente e non vuoto, non solo che le due collezioni coincidano).
  Il test asserisce anche che l'export dell'originale contenga ogni capability verificata da T0, così un confronto
  "vuoto = vuoto" non può passare. **Normalizzazioni ammesse, elencate esplicitamente nel codice e nel rapporto**: campi Meta che il
  salvataggio legittimamente aggiorna (es. `Generator`, `HeaderHash`, timestamp di modifica di Meta,
  `SettingsChanged`), e l'ordine degli elementi `Binary` nel pool Meta (risolti per contenuto). Ogni altra differenza
  fa fallire il test con un diff leggibile.
- **T3 Creazione (keepass-rs → KeePassXC)**: crea da zero con keepass-rs un DB KDBX4 con **Argon2id** (parametri
  espliciti piccoli per velocità di test, es. 16 MiB / 2 iterazioni / p=2) e cipher esterno **AES-256** e, in un
  secondo caso, **ChaCha20**; contenuto: gruppo annidato, voce con campi standard, campo custom protetto, allegato,
  una versione in cronologia. Verifica con `keepassxc-cli export` che KeePassXC apra il file e che il contenuto
  corrisponda; verifica con `keepassxc-cli db-info` il KDF/cipher dichiarato.
- **T4 Password errata**: keepass-rs rifiuta la fixture con password sbagliata con un errore (nessun panic), e
  `keepassxc-cli` rifiuta il file creato in T3 con password sbagliata.
- **Gating**: i test che richiedono `keepassxc-cli` lo cercano nel PATH (o in `KEEPASSXC_CLI`). Se assente: se
  `KEYRUST_REQUIRE_KEEPASSXC=1` falliscono, altrimenti stampano un avviso visibile e terminano senza asserire
  (non contano come verifica). T1 e T4-parte-keepass-rs non richiedono la CLI.
- La password passa a `keepassxc-cli` solo via stdin, mai come argomento di riga di comando.

### Esito di un gap
Se un test rivela una perdita di dati o un'incompatibilità reale di keepass-rs, **non** si indebolisce l'asserzione:
il test specifico viene marcato `#[ignore = "GAP-n: <descrizione> — vedi docs/spikes/0001-keepass-rs-kdbx4.md"]`,
il rapporto descrive il gap con il diff osservato, e la decisione indica l'opzione (contributo upstream, workaround in
`core`, alternativa). Il resto dei test deve passare. Un gap non è un fallimento della Milestone 0: lo è un rapporto
non veritiero.

## Scaffold app — approccio
- Struttura: `crates/app/{Cargo.toml, build.rs, src/main.rs, src/lib.rs, tauri.conf.json, capabilities/default.json, icons/}`
  e `crates/app/ui/` (package.json, vite, svelte, src/). `tauri.conf.json`: `build.frontendDist = "ui/dist"`,
  `beforeDevCommand`/`beforeBuildCommand` con `cwd: "ui"`. `@tauri-apps/cli` come devDependency del frontend
  (script `pnpm tauri`).
- Generato partendo dal template ufficiale (`create-tauri-app`, template svelte-ts) e poi riorganizzato, non scritto
  a memoria; dipendenze con versioni esatte risolte nei lockfile (`Cargo.lock`, `pnpm-lock.yaml`).
- Icone: icone placeholder generate con `pnpm tauri icon` da un SVG semplice (chiave stilizzata) in `crates/app/icons/source.svg`.
- `keyrust-app` dipende da `keyrust-core` (path) solo per `VERSION`, dimostrando il collegamento nel workspace.

## Non-goals
- Nessuna feature di prodotto (vault, voci, clipboard, auto-lock), nessuna API definitiva in `core`.
- Nessuna CI GitHub Actions (Milestone 1), nessun repo remoto, push o release.
- Nessuna installazione di webkit2gtk/GTK né build Tauri su Linux.
- Nessun design system completo (token indaco, componenti ricorrenti: Milestone 2).

## Rischi residui
- Il crate `keyrust-app` **non viene compilato qui**: errori di compilazione o configurazione Tauri emergono solo su
  Windows. Mitigazione: codice Rust dell'app minimale e derivato dal template ufficiale; `cargo metadata` e
  `cargo deny` coprono comunque la risoluzione delle dipendenze; checklist Windows dettagliata in `docs/DEVELOPMENT.md`.
- Nessun clippy sul crate app qui (vale lo stesso).
- `cargo-deny`: installato da binario di release ufficiale (GitHub `EmbarkStudios/cargo-deny`) nel `~/.cargo/bin`;
  se il download fallisce, `cargo install cargo-deny --locked` (più lento, più disco).
- Spazio disco: monitorare; `target/` di core + test è stimato < 1.5 GB.

## Verifica (comandi di prova, eseguiti dall'host su Linux)
```
cargo fmt --all --check                                            # atteso: nessun output, exit 0
cargo clippy -p keyrust-core --all-targets -- -D warnings          # atteso: exit 0
KEYRUST_REQUIRE_KEEPASSXC=1 cargo test -p keyrust-core             # atteso: tutti passano (eventuali GAP ignorati e documentati)
cargo deny check                                                   # atteso: advisories/bans/licenses/sources ok (workspace intero, incluso app)
cargo metadata --format-version 1 >/dev/null                       # atteso: exit 0 (workspace e app risolvono)
cd crates/app/ui && pnpm install --frozen-lockfile && pnpm check && pnpm lint && pnpm format:check && pnpm build   # atteso: exit 0, ui/dist creato
bash scripts/gen-fixtures.sh && git status --porcelain tests/fixtures   # rigenera: nessuna differenza semantica (T1/T2 ancora verdi)
git ls-files | grep -E '\.kdbx$'                                   # atteso: solo file sotto tests/fixtures/
```
Controlli manuali:
- Leggere il rapporto dello spike e confrontarlo con l'output reale dei test (nessun PASS dichiarato senza test).
- Sanity check su un file reale: un DB creato da T3 riaperto con `keepassxc-cli ls`/`show -s` mostra i valori attesi.
- **Su Windows (utente)**, checklist in `docs/DEVELOPMENT.md`: `cargo test -p keyrust-core` (con KeePassXC installato e
  `KEYRUST_REQUIRE_KEEPASSXC=1`), `cargo clippy --workspace --all-targets -- -D warnings`, `pnpm tauri dev` dalla
  cartella `crates/app/ui` → si apre la finestra con la schermata placeholder e la versione; DevTools console senza
  violazioni CSP; `pnpm tauri build` produce un installer.

## Toolchain per provider
- Builder (Claude, host): usa la toolchain sopra; ha rete per crates.io/npm/gnu.org.
- Inspector (Codex, read-only): legge file e diff; non esegue comandi di prova (li esegue l'host).
