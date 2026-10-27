# Piano — Milestone 3b (IPC, UI sul vault reale, appunti, dialoghi file) di Keyrust

Riferimenti: `docs/TASKS.md` (Milestone 3, sezione "app"), `docs/ARCHITECTURE.md`, `docs/plans/milestone-3a.md`,
`docs/plans/milestone-2.md`. Base: `main` @ `3fbafab`. Branch: `milestone-3b`.

## Obiettivo
Rendere Keyrust un password manager funzionante: l'app desktop apre, crea, modifica e salva file KDBX reali
attraverso il `Vault` della fase 3a.
- L'interfaccia della Milestone 2 passa dai dati demo a comandi IPC tipizzati.
- La copia negli appunti avviene in Rust, con svuotamento automatico.
- I file si scelgono con i dialoghi nativi.

La demo resta soltanto per l'anteprima web e per i test.

## Contesto e vincoli (confermati)
- **Macchina di sviluppo**: Linux con ~800 MB liberi, senza librerie di sistema Tauri. Il crate `keyrust-app` si
  compila **solo in CI**, che è la prova per i comandi IPC, gli appunti e i dialoghi; in locale si verificano il core
  e il frontend.
  - In locale nessuna nuova compilazione pesante: le dipendenze aggiunte al crate app (`arboard`,
    `tauri-plugin-dialog`, `tauri-plugin-opener`) vengono compilate solo in CI.
- **Utente**: testa su un PC Windows aziendale. Gli installer sono generati sulla PR; la verifica manuale segue la
  checklist di `docs/DEVELOPMENT.md`.
- **Fasi decise dall'utente**: 3a (fatta) core; **3b** questa; 3c auto-lock (inattività, blocco schermo,
  minimizzazione), limiti KDF in apertura e fuzzing.
- **Autorizzato**: commit e push su `milestone-3b`. **Non autorizzato**: push su `main`.

## Criteri di accettazione
1. **Comandi IPC** in `crates/app/src/commands.rs` (sottile strato sopra `keyrust_core::vault::Vault`).
   - Elenco:
     - file e sessione: `recent_files`, `choose_database_file`, `choose_new_database_path`, `unlock`, `create`,
       `lock`, `database_info`;
     - gruppi: `tree`, `create_group`, `rename_group`, `delete_group`;
     - voci: `list_entries`, `get_entry`, `get_entry_for_edit`, `reveal_field`, `copy_field`, `save_entry`,
       `delete_entry`, `restore_entry`;
     - impostazioni del database: `update_database` (nome, cifrario, cestino, Argon2id), `change_master_password`;
     - salvataggio: `save`, `save_overwrite`, `reload`, `check_file`;
     - strumenti: `generate_password`, `estimate_strength`, `benchmark_kdf`, `open_url`;
     - quelli della M1 e M2: `app_info`, `get_settings`, `update_settings`, `exit_app`.
   - `COMMANDS` e `security_config.rs` sono aggiornati.
   - **Stato**: `VaultState(Mutex<Option<Vault>>)`. Un comando che richiede un vault aperto senza averlo restituisce
     l'errore `locked`.
   - **Errori**: arrivano al frontend come `{ kind }` tipizzato (`wrongKey`, `invalidFormat`, `unsupportedVersion`,
     `externalChange`, `notFound`, `invalid`, `locked`, `io`, `notDurable`, `alreadyExists`, `cancelled`), mai con
     percorsi, valori o segreti.
   - **Segreti attraverso l'IPC**: `list_entries`, `get_entry`, `tree` e `database_info` non portano password.
     - La password attraversa l'IPC solo con `reveal_field`, con `get_entry_for_edit` (editor) e in ingresso con
       `unlock`, `create`, `save_entry` e `change_master_password`, dove viene deserializzata direttamente in
       `SecretString`.
     - `copy_field` non restituisce il valore.
   - Lunghi lavori (`unlock`, `create`, `benchmark_kdf`, `save`) girano in un thread di blocco
     (`tauri::async_runtime::spawn_blocking`), così la UI non si blocca durante la KDF.
2. **Appunti** (`crates/app/src/clipboard.rs`, crate `arboard` con `wayland-data-control`).
   - `copy_field` scrive il valore negli appunti chiedendo l'esclusione dalla cronologia dove supportata
     (`exclude_from_history` su Windows, macOS e Linux) e restituisce i secondi di auto-clear presi dalle
     impostazioni.
   - Allo scadere, gli appunti vengono svuotati **solo se** contengono ancora il valore copiato, confrontato tramite
     hash SHA-256 conservato in memoria al posto del valore.
   - Svuotamento anche al lock e all'uscita (stessa condizione).
   - Una nuova copia sostituisce il timer precedente.
   - La logica di decisione è una funzione pura testata in CI, senza display; `arboard` è dietro un piccolo trait,
     così i test usano un appunti finto.
3. **Dialoghi file e URL** tramite `tauri-plugin-dialog` e `tauri-plugin-opener`, **chiamati solo dal Rust**.
   - La capability resta `core:default`: nessun permesso di plugin al frontend.
   - `security_config.rs` cambia da "nessun plugin" ad "allowlist esatta `dialog` e `opener`, nessun loro permesso in
     alcuna capability".
   - `open_url` accetta solo `http`/`https` (URL analizzato) e rifiuta il resto con `invalid`.
   - Il dialogo di salvataggio per un nuovo database propone `.kdbx`.
   - Un dialogo annullato restituisce `cancelled`, che il frontend tratta in silenzio.
4. **File recenti**, gestiti solo dal backend.
   - Dopo un unlock o una creazione riusciti il percorso finisce in testa a `Settings.recent_files` (al massimo 10,
     senza duplicati) e le impostazioni vengono salvate.
   - `update_settings` **non accetta** `recent_files` dal frontend: conserva sempre il valore corrente del backend, e
     le due scritture passano dallo stesso `SettingsState` sotto lo stesso mutex. Un'istantanea vecchia del frontend
     non può quindi cancellare un file recente appena registrato.
   - Test: unlock seguito da un aggiornamento delle preferenze con `recent_files` vecchio → il file recente resta.
   - `recent_files` restituisce nome (radice del file), percorso e `missing` (il file non esiste più).
   - Il percorso completo compare in UI, che è il comportamento atteso dall'utente; nei log appare solo a `debug`.
5. **Frontend sul vault reale**.
   - `src/lib/api/tauri.ts` implementa `VaultApi` con `invoke`; `createApis()` sceglie `TauriVault` dentro Tauri e
     `DemoVault` fuori (anteprima web, test).
   - L'avviso "Demo data" compare solo con `DemoVault`.
   - **Tipi aggiornati al core 3a**:
     - `KdfParams` diventa `KdfInfo` (AES-KDF, Argon2d/Argon2id, altro) più `Argon2idParams` modificabili;
     - `DatabaseInfo.version`, `EntryDetail.historyLen`, date opzionali.
   - **Settings → Database**:
     - mostra la KDF reale; per un database AES-KDF o Argon2d propone "Switch to Argon2id" invece di modificare
       parametri che non esistono;
     - "Change master password…" apre un dialog con nuova password, conferma e strength meter, che chiama
       `change_master_password` e poi salva.
   - **Salvataggio**: ogni modifica salva subito, come già nella M2.
     - `externalChange` mostra il banner con Reload / Keep mine, collegati a `reload` / `save_overwrite`; anche
       `check_file`, quando la finestra torna in primo piano, fa comparire il banner.
     - `notDurable` mostra il banner di errore con Retry.
   - **Lock**: chiama `lock` in Rust (drop del `Vault`, svuotamento degli appunti) oltre ad azzerare lo stato del
     frontend.
   - **Modifiche del vault non ancora sul disco**:
     - lo stato tiene `vaultDirty`, aggiornato da ogni comando che modifica il vault (voci, gruppi, impostazioni del
       database, cambio password) e azzerato solo da un salvataggio riuscito, più `saving` (salvataggio in corso);
     - è indipendente dall'editor;
     - `vaultDirty` viene da `Vault::is_dirty` tramite `database_info` / le risposte dei comandi, non da una stima del
       frontend.
   - **Guardia unificata**: Quit, chiusura della finestra, Close Database, New/Open Database e il cambio di file
     recente passano da una guardia che considera sia l'editor modificato sia `vaultDirty`/`saving`.
     - Con modifiche non salvate nel vault mostra un dialog ("Your last changes are not saved") con **Retry save** /
       **Discard changes** / **Cancel**.
     - Con un salvataggio in corso attende che finisca.
     - Lock resta immediato e scarta, per decisione già presa.
   - **Chiusura della finestra**: in Rust `CloseRequested` viene intercettato e inoltrato al frontend come comando
     `quit`, così la guardia vale anche per la X della finestra; `exit_app` svuota gli appunti e chiude.
   - Test di flusso:
     - un salvataggio fallito (`externalChange`, I/O) dopo una modifica di una voce, di un gruppo e delle impostazioni
       del database, seguito da Quit, Close Database o apertura di un altro file, mostra la guardia;
     - Discard procede, Cancel resta, Retry save procede solo se il salvataggio riesce.
6. **Test**.
   - **Rust, in CI** (`cargo test -p keyrust-app`):
     - mappatura degli errori senza dettagli;
     - logica degli appunti con un appunti finto: svuota solo se invariato, un timer nuovo sostituisce il vecchio,
       svuotamento al lock;
     - `update_settings` conserva i `recent_files` del backend;
     - validazione di `open_url`;
     - aggiornamento dei file recenti;
     - deserializzazione di `EntryDraft` / `CreateOptions` dal JSON del frontend;
     - `COMMANDS` uguale all'elenco di `generate_handler!`;
     - test di sicurezza della configurazione aggiornati.
   - **Rust, in locale e in CI**: i test del core restano verdi.
   - **Frontend (Vitest)**:
     - `TauriVault` con `invoke` simulato: nomi dei comandi e argomenti, nessuna password nelle risposte di
       lista/dettaglio usate dalla UI, errori `{ kind }` tradotti negli stessi stati della demo;
     - i test di flusso esistenti restano verdi sulla demo;
     - Settings con `KdfInfo` AES-KDF mostra "Switch to Argon2id";
     - dialog di cambio password (conferma obbligatoria).
7. **Documentazione**: `ARCHITECTURE.md` (comandi, appunti, plugin solo dal Rust), `SECURITY.md` (appunti, plugin),
   `DEVELOPMENT.md` (checklist manuale aggiornata: aprire, creare e modificare un database reale, compatibilità con
   KeePassXC, appunti), `TASKS.md`.

## Approccio e decisioni
- **Plugin ufficiali chiamati dal Rust invece di `rfd`/`open` diretti**: gestiscono i thread e le API native sui tre
  sistemi operativi. Il frontend non ottiene alcun permesso, quindi il confine resta "il webview non tocca file né
  shell".
- **Comandi sottili**: tutta la logica di dominio è nel core (già testato contro KeePassXC); il crate app traduce
  tipi ed errori e gestisce stato, appunti e dialoghi.
- **Niente polling degli appunti**: un timer per copia, annullato da quella successiva; al termine controlla l'hash.
- **Demo conservata** per l'anteprima web e i test: lo stesso codice UI gira in entrambi i casi.

## Non-goals
- Auto-lock per inattività, al blocco schermo e alla minimizzazione; limiti KDF per file ostili; fuzzing di
  `open_bytes` (3c).
- Key file, YubiKey, merge, allegati e campi personalizzati in UI, tema scuro.

## Rischi
- **Compilazione del crate app solo in CI**: ogni errore costa un giro di CI. Mitigazioni: comandi sottili e codice
  ripetitivo, tipi del core già testati, `cargo check` delle parti pure dove possibile.
- **Appunti su Linux headless in CI**: i test usano il trait con un appunti finto; l'uso reale si verifica a mano
  (checklist).
- **Plugin e capabilities**: un plugin registrato potrebbe esporre comandi al webview. Mitigazione: nessun permesso
  di plugin nelle capabilities (verificato dal test) e verifica manuale dalla console che le chiamate dirette
  falliscano.
- **Disco** (~800 MB): niente build locale del crate app; frontend senza nuove dipendenze.

## Verifica
Locale:
```
CARGO_PROFILE_DEV_DEBUG=line-tables-only cargo test -p keyrust-core
cargo fmt --all --check
(cd crates/app/ui && pnpm check && pnpm lint && pnpm format:check && pnpm test && pnpm build)
```
Remota (CI su `milestone-3b`): tutti i job verdi, incluso `cargo clippy --workspace -D warnings` e
`cargo test -p keyrust-app` con i nuovi test; sulla PR gli installer dei tre sistemi.

Manuale (utente, appena può installare una build; checklist in `docs/DEVELOPMENT.md`):
- creare un database, aggiungere voci, chiuderlo e riaprirlo;
- aprirlo con KeePassXC;
- copiare una password e verificare che gli appunti si svuotino dopo N secondi;
- una modifica fatta con KeePassXC mentre Keyrust è aperto mostra il banner.

## Toolchain per provider
- **Builder** (Claude, host): Rust 1.99.1 (solo il core in locale), Node 24 e pnpm 12.15 per il frontend; rete verso
  crates.io, npm e api.github.com; push via SSH (account `downlevel`).
- **Reviewer/Inspector** (Codex, sandbox read-only): legge file e diff con comandi di sola lettura; non esegue i comandi
  di prova, build o test (li esegue l'host). Non raggiunge GitHub: lo stato della CI viene fornito dall'host.
