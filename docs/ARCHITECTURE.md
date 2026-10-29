# Architettura — Keyrust

## Stack tecnologico
| Ambito | Scelta | Perché |
|---|---|---|
| Linguaggio core | Rust (stable, edition 2021+) | Memory safety, ecosistema crypto RustCrypto maturo, ottimo segnale da portfolio. |
| Formato DB | KDBX4 tramite crate `keepass` (keepass-rs) | Compatibilità con KeePassXC / KeePassDX / Strongbox, quindi mobile e sync arrivano "gratis". |
| Crypto | RustCrypto: `argon2`, `aes-gcm`, `chacha20poly1305` (+ quelle usate da keepass-rs) | Librerie standard, auditate e mantenute; nessuna primitiva scritta a mano. |
| Segreti in memoria | `zeroize` + `secrecy` | Azzeramento garantito al drop, `Debug` redatto per evitare leak nei log. |
| CSPRNG | `getrandom` (via `rand_core::OsRng`) | Entropia dal sistema operativo per generatore e salt. |
| Clipboard | `arboard` | Cross-platform, supporta Wayland (`wayland-data-control`) ed esclusione dalla cronologia dove disponibile. |
| Desktop shell | Tauri 2 | Binari piccoli, backend Rust nativo, modello di permessi (capabilities) per limitare l'IPC. |
| Frontend | Svelte 5 + TypeScript + Vite | Bundle leggero, reattività con runes, poco boilerplate per un'app di questa dimensione. |
| UI / styling | Tailwind CSS + shadcn-svelte (bits-ui) | Componenti headless accessibili, stile minimale pienamente controllabile, token CSS pronti per il dark mode. |
| i18n | Paraglide JS 2 (inlang) | Messaggi compilati e tipizzati, tree-shaking; plugin inlang installati da npm (nessun caricamento da CDN a build time). |
| Backend / DB / Auth | Nessuno | App locale: il "database" è il file `.kdbx` dell'utente; l'autenticazione è la master password. |
| Distribuzione | GitHub Releases (AppImage/deb, MSI/NSIS, dmg) | Tauri bundler; release firmate, build riproducibili come obiettivo. |
| CI/CD | GitHub Actions, matrice Linux/Windows/macOS | Build, test, clippy, fmt su tre OS; job di release su tag. |
| Licenza | GPL-3.0-or-later | Stessa licenza di KeePassXC/KeePassDX; i fork restano open. |

## Componenti
```
┌──────────────────────────── crates/app (Tauri 2) ────────────────────────────┐
│                                                                              │
│  Frontend (Svelte 5, webview)          Backend Tauri (Rust)                  │
│  ┌───────────────────────────┐  IPC   ┌───────────────────────────────────┐  │
│  │ Unlock / Vault / Settings │◀──────▶│ commands.rs  (API tipizzata)      │  │
│  │ store UI (no segreti      │ invoke │ autolock.rs  (Session, locker,    │  │
│  │ persistenti)              │ events │               trigger)            │  │
│  └───────────────────────────┘        │ clipboard.rs (arboard, auto-clear)│  │
│                                       └──────┬──────────┬─────────────────┘  │
└──────────────────────────────────────────────┼──────────┼────────────────────┘
                     crates/os (unsafe FFI) ◀──┘          │
                     blocco schermo, sospensione          │ API Rust
┌──────────────────────────── crates/core ────────────────▼────────────────────┐
│ vault.rs     Vault: open / create / save (atomico) / lock                    │
│ model.rs     viste su gruppi e voci (EntryId, GroupId, EntrySummary…)        │
│ search.rs    ricerca su campi non segreti                                    │
│ generator.rs generatore password (OsRng) + stima robustezza                  │
│ kdf.rs       parametri Argon2id, calibrazione                                │
│ kdbx/header.rs  lettura header KDBX4 senza crittografia (limiti KDF)          │
│ kdbx/limits.rs  tetti di apertura (memoria/iterazioni/lane, round, dimensione)│
│ settings.rs  impostazioni app (JSON, scrittura atomica, nessun segreto)      │
│ error.rs     errori tipizzati (thiserror), senza segreti nei messaggi        │
│      └── keepass-rs (KDBX4 parse/serialize) ── RustCrypto                    │
└──────────────────────────────────────────────────────────────────────────────┘
        (futuro) crates/cli — CLI/TUI che dipende solo da core
        fuzz/              — target cargo-fuzz su core
```

### Struttura del repository
```
keyrust/
├── Cargo.toml              # workspace
├── crates/
│   ├── core/               # nessuna dipendenza da Tauri/UI
│   ├── os/                 # blocco schermo e sospensione; unico crate con unsafe (FFI)
│   └── app/                # Tauri 2 (src-tauri) + frontend Svelte (ui/)
├── fuzz/                   # cargo-fuzz
├── tests/fixtures/         # file .kdbx creati con KeePassXC
├── docs/                   # PRD, ARCHITECTURE, DESIGN, TASKS
├── README.md  SECURITY.md  LICENSE  CONTRIBUTING.md
└── .github/workflows/
```

### Principi di confine
- **`core` è puro**: niente Tauri, niente I/O di UI, niente logging di valori. È testabile in isolamento
  e riusabile dalla futura CLI.
- **Il frontend non possiede segreti**: la lista voci riceve solo `EntrySummary` (id, titolo, username, URL,
  gruppo). La password attraversa l'IPC solo su richiesta esplicita ("mostra" o "modifica") e non viene mai
  messa in store persistenti né in `localStorage`. La copia negli appunti avviene **interamente lato Rust**
  (`copy_field(entry_id, field)`), quindi la password non passa nemmeno dal webview.
- **La master password** è passata dal form di sblocco a `unlock` come `SecretString`, poi azzerata;
  il campo input viene svuotato subito dopo l'invio.
- **Plugin chiamati solo dal Rust**: `tauri-plugin-dialog` (dialoghi file) e `tauri-plugin-opener` (URL http/https
  validati) sono registrati ma nessuna capability ne concede i permessi al webview; lo script dell'opener che
  intercetta i link è disattivato. I percorsi arrivano a `unlock`/`create` solo dai dialoghi o dai file recenti
  (allowlist nel backend). Comandi IPC in `crates/app/src/commands.rs`; appunti in `crates/app/src/clipboard.rs`.
- **Tauri capabilities minime**: nessun accesso FS/shell dal frontend; dialoghi file e operazioni su disco
  passano da comandi Rust dedicati. CSP restrittiva, nessuna risorsa remota.

### Frontend (crates/app/ui)
```
src/lib/api/        VaultApi (interfaccia), DemoVault (dati di esempio), AppApi (Tauri o in memoria)
src/lib/state/      AppState: schermate, selezione, editor, overlay; generazioni di sessione e selezione
src/lib/commands/   commands.json (fonte unica per menu, scorciatoie, pannello e palette) + matcher
src/lib/screens/    Unlock/Welcome, Create, Vault
src/lib/components/ app/ (componenti Keyrust), ui/ (shadcn-svelte vendorizzati)
```
- **Nessun routing**: la schermata è stato applicativo (`AppState.screen`), uguale in Tauri e nell'anteprima web.
- **Confine dei segreti anche nel frontend**: lista e dettaglio ricevono `EntrySummary`/`EntryDetail` senza
  password; la password arriva solo da `revealField` (mostra) o `getEntryForEdit` (editor). `copyField` non
  restituisce valori.
- **Risposte tardive**: ogni chiamata asincrona cattura le generazioni di sessione (unlock/lock/create/cambio
  database) e di selezione (cambio voce, apertura/chiusura editor) e applica il risultato solo se sono invariate.
- **Modifiche non salvate**: ogni azione che lascia un editor modificato passa da `requestLeave`
  (Save / Discard / Keep editing); Lock scarta subito.
- **Menu nativo**: `crates/app/src/menu.rs` costruisce il menu da `commands.json` con le etichette di
  `messages/en.json`; un clic emette l'evento `menu` con l'id del comando. Il menu Edit usa le voci
  predefinite, così copia/incolla funziona nei campi di testo.
- In Milestone 2 `VaultApi` è implementata solo da `DemoVault`; la Milestone 3 aggiunge l'implementazione IPC
  con gli stessi metodi, scelta in `src/lib/api/index.ts`.

## Schema dati
Il file è KDBX4 standard; Keyrust usa il modello di keepass-rs e preserva tutto ciò che non mostra
(campi custom, allegati, cronologia, dati personalizzati), così il round-trip con KeePassXC è lossless.

Tipi esposti da `core` (indicativi):
```rust
pub struct EntrySummary { id: Uuid, group_id: Uuid, title: String, username: String, url: String, modified: DateTime }
pub struct EntryDetail  { summary: EntrySummary, notes: String, has_password: bool, history_len: usize }
pub struct GroupNode    { id: Uuid, name: String, children: Vec<GroupNode>, entry_count: usize, is_recycle_bin: bool }
pub enum   Field        { Username, Password, Url, Notes }
pub struct KdfParams    { memory_kib: u32, iterations: u64, parallelism: u32 }   // Argon2id
pub struct GeneratorOptions { length: u8, lower: bool, upper: bool, digits: bool, symbols: bool, exclude_ambiguous: bool }
```

**Impostazioni app** (`settings.json` nella config dir dell'OS, via Tauri path API), senza segreti:
`recent_files[]`, `clipboard_clear_secs` (20), `idle_lock_mins` (5), `lock_on_minimize` (false),
`lock_on_screen_lock` (true), `generator_defaults`.

**Default KDF per nuovi DB** (allineati a KeePassXC): Argon2id, 64 MiB, parallelismo 2, iterazioni calibrate
per circa 1 s sulla macchina corrente. Cifratura AES-256 o ChaCha20 come previsto da KDBX4.

## Flussi chiave lato backend
- **Salvataggio atomico** (`crates/core/src/file.rs`):
  - lock advisory tra istanze di Keyrust: `.<nome>.keyrust-lock` accanto al database (permessi 0600, lasciato sul
    posto perché rimuoverlo creerebbe una race);
  - confronto dello stampo (lunghezza + SHA-256 dei byte letti o scritti l'ultima volta);
  - serializzazione in un file temporaneo esclusivo nella stessa directory → `fsync`;
  - il file temporaneo nasce con permessi 0600 e prende quelli del file originale (un vault 0600 resta 0600);
  - **secondo confronto** subito prima del `rename` → `fsync` della directory (Unix). Se questo `fsync` fallisce il
    file è già stato sostituito: lo stato del vault segue il file e l'errore è `NotDurable`.
  - Se lo stampo non corrisponde il salvataggio è rifiutato (`ExternalChange`) e l'utente sceglie *Reload*
    (`Vault::reload`, con le credenziali confermate) o *Keep mine* (`save_overwrite`). Niente merge in V1.
  - Rischio residuo: uno scrittore che non usa il lock può scrivere tra l'ultimo confronto e il `rename`.
- **Vault** (`crates/core/src/vault.rs`): avvolge `keepass::Database` e lavora sul suo modello, così tutto ciò che
  Keyrust non mostra resta intatto.
  - La cronologia è gestita da Keyrust: la versione precedente viene aggiunta in fondo (dalla più vecchia alla più
    recente, come KeePass) entro `history_max_items`/`history_max_size`.
  - Le eliminazioni definitive scrivono tombstone in `DeletedObjects`.
  - I file KDBX 3.1 e 4.0 vengono salvati come KDBX 4.1 (keepass-rs scrive solo 4.1).
  - Un database AES-KDF o Argon2d mantiene la sua KDF finché l'utente non sceglie Argon2id.
  - Il vault lavora sul percorso reale del file (i link simbolici vengono risolti all'apertura), così il salvataggio
    aggiorna il file di destinazione e non sostituisce il link.
  - I parametri Argon2id sono validati per intero (`kdf::validate`: limiti di keepass-rs e di Argon2, almeno 8 KiB per
    lane, e gli stessi tetti dell'apertura) prima di toccare il vault.
- **Limiti di apertura** (`crates/core/src/kdbx/limits.rs`): `Vault::open`, `open_bytes` e `reload` leggono il file
  da una sola handle con `take(max + 1)` (512 MiB), poi controllano i parametri KDF dall'header (KDBX 4 e 3.1) prima
  di qualsiasi derivazione: Argon2 ≤ 2 GiB / 1000 iterazioni / 256 lane, AES-KDF ≤ 2·10⁹ round. Oltre →
  `VaultError::TooExpensive` (IPC `tooExpensive`). Anche il controllo di modifica esterna (`file::unchanged`) confronta
  prima la lunghezza e poi calcola SHA-256 in streaming senza leggere oltre. La decompressione resta senza tetto
  (avviene solo dopo l'HMAC con la chiave dell'utente): limite documentato in `SECURITY.md`.
  - Il `Debug` di tutte le viste (`EntrySummary`, `EntryDetail`, `GroupNode`, `DatabaseInfo`, `EntryDraft`, `Vault`)
    mostra solo identificatori e conteggi.
- **Auto-clear clipboard**: dopo la copia parte un timer; alla scadenza svuota gli appunti **solo se** contengono
  ancora il valore copiato (confronto su hash, così non cancella ciò che l'utente ha copiato dopo).
  Hint di esclusione dalla cronologia (KDE/Windows) dove `arboard` lo supporta. Svuotamento anche al lock e all'uscita.
- **Auto-lock** (`crates/app/src/autolock.rs`, decisioni pure in `autolock/policy.rs`), tutto nel processo Rust:
  - `VaultState` contiene una `Session`: il vault e una **generazione** incrementata da ogni lock (manuale o
    automatico). `unlock` e `create` la leggono prima della KDF e installano il vault solo se è invariata;
    altrimenti lo scartano (zeroize) e rispondono `locked`. Un blocco durante uno sblocco lento non viene annullato.
  - Ogni trigger invia un `Reason` (`idle`, `screenLock`, `suspend`, `minimize`) a **un solo thread locker**, che
    applica le impostazioni (`should_lock`), esegue il lock (drop del `Vault`, svuotamento degli appunti) ed emette
    l'evento `locked` con `{ reason }`. I trigger non attendono mai il mutex del vault, che un salvataggio tiene
    durante la KDF.
  - **Inattività**: il frontend chiama `activity` a ogni input, al massimo ogni 15 s; un thread controlla ogni 5 s.
  - **Blocco schermo**: Windows e macOS con polling ogni 2 s di `keyrust_os::screen_locked()` (sul fronte di
    blocco); Linux con `keyrust_os::on_screen_lock` (`ActiveChanged(true)` di un'interfaccia `*.ScreenSaver` sul bus
    di sessione).
  - **Sospensione**: Windows `PowerRegisterSuspendResumeNotification` (callback, senza finestra), Linux logind
    `PrepareForSleep(true)`; ovunque un thread a 1 s che riconosce un salto dell'orologio di parete oltre 4 s
    (unico rilevatore su macOS).
  - **Minimizzazione**: `WindowEvent::Resized` con la finestra minimizzata.
  - Il frontend, all'evento `locked`, azzera lo stato come il Lock (senza richiamare il backend) e mostra la notice
    del motivo sulla schermata di sblocco.
- **`crates/os`** (`keyrust-os`): le poche chiamate al sistema operativo che servono all'auto-lock. È l'unico crate
  con `unsafe` (FFI Windows e macOS), negato a livello di crate e ammesso funzione per funzione con un commento
  `SAFETY:`; la parte Linux usa `zbus` ed è safe.

## Deploy
- **Ambienti**: solo locale (`cargo tauri dev`) + release.
- **Pipeline release** (`.github/workflows/release.yml`, su tag `v*` di un commit di `main`): matrice tre OS →
  `tauri build` (NSIS + MSI, deb + AppImage, dmg universale) → `SHA256SUMS` firmato con **cosign keyless** (identità
  OIDC del workflow, log Rekor) → **bozza** di GitHub Release, pubblicata a mano. Ogni altro evento è un dry run
  senza release. Procedura in [RELEASING.md](RELEASING.md), verifica per gli utenti in `SECURITY.md`.
- **Firma codice OS**: **opzionale**, si attiva solo se esistono i secret dell'OS (Windows: Azure Trusted Signing
  via `signCommand`; macOS: Developer ID + notarizzazione con chiave API). Senza secret gli installer escono non
  firmati (macOS con firma ad hoc) e le note di release lo dichiarano.
- **Build riproducibili** ("best effort"): toolchain, Node e pnpm fissati, lockfile con `--locked` /
  `--frozen-lockfile`, `SOURCE_DATE_EPOCH` (anche come versione di SvelteKit), percorsi rimappati, nessuna cache;
  il job `reproducibility` ricostruisce due volte il binario Linux e riporta l'esito nelle note, senza bloccare.
- **Segreti CI**: solo i secret di firma codice, letti solo su push di tag e solo nello step di build che li usa;
  cosign non ne richiede.

## Testing & osservabilità
- **Unit test `core`**: generatore (distribuzione, vincoli), ricerca, gestione gruppi/voci, parametri KDF, errori.
- **Test di compatibilità KeePassXC** (requisito MVP):
  - *KeePassXC → Keyrust*: fixture `.kdbx` create con KeePassXC (varie combinazioni KDF/cifratura, campi custom,
    allegati, cronologia) committate in `tests/fixtures/`; i test le aprono e verificano il contenuto atteso.
  - *Keyrust → KeePassXC*: in CI (Linux) i test generano un DB e lo verificano con `keepassxc-cli ls/show/export`.
  - *Round-trip lossless*: apri fixture → salva → riapri con `keepassxc-cli` → nessuna differenza nell'export XML.
- **Test di sicurezza**: test che verificano che `Debug`/log non contengano segreti; clippy con lint su `unwrap`
  nei percorsi critici; `cargo audit` / `cargo deny` in CI.
- **Fuzzing**: `cargo-fuzz` (`fuzz/`): `kdbx_header_peek` (header), `vault_open_bytes` (file interi con limiti
  minimi, corpus di semi generati con keepass-rs) e `vault_roundtrip` (contenuto arbitrario salvato e riaperto).
  60 s per target in CI, corsa lunga manuale con `workflow_dispatch` (vedi `fuzz/README.md`).
- **Frontend**: Vitest per store e componenti; Playwright/WebDriver (tauri-driver) per uno smoke e2e su Linux.
- **CI** (`.github/workflows/ci.yml`): job `core` e `app` su Linux/Windows/macOS (fmt, clippy, test con KeePassXC,
  check/lint/test frontend, build Tauri debug non firmata caricata come artifact), `deny`, `fuzz`. Action fissate a SHA.
- **Osservabilità**: `tracing` + `tracing-subscriber` su stderr (filtro `KEYRUST_LOG`, default `info`), nessun file di log.
  Log a livello di eventi, tipi di errore e conteggi; mai valori di campi; percorsi solo a `debug`. Verificato da
  `crates/core/tests/logging_policy.rs`. Nessuna telemetria e nessun error tracking remoto.

## Domande aperte
- **Supporto KDBX4 in scrittura di keepass-rs**: verificare maturità del feature flag di salvataggio KDBX4 e la
  preservazione lossless di campi custom/allegati/cronologia; se insufficiente, contribuire upstream (bonus da portfolio).
- ~~Firma codice Windows/macOS~~: decisa in Milestone 4b. Checksum firmati con cosign keyless sempre; firma codice OS
  opzionale, attivata dai secret (nessun certificato acquistato per v0.1.0).
- ~~Libreria i18n~~: decisa in Milestone 1, Paraglide JS 2.
- ~~Conflitti di sync~~: decisa in Milestone 3, rilevamento e avviso (Reload / Keep mine), niente merge in V1.
- ~~Package manager frontend~~: pnpm.
