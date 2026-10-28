# Piano — Milestone 3c (auto-lock, limiti KDF, fuzzing del parsing) di Keyrust

Riferimenti: `docs/TASKS.md` (Milestone 3: auto-lock, fuzz `open_from_bytes`), `docs/ARCHITECTURE.md`, `SECURITY.md`,
`fuzz/README.md`, `docs/plans/milestone-3a.md`, `docs/plans/milestone-3b.md`. Base: `main` @ `1f07b58`. Branch:
`milestone-3c`.

## Obiettivo
Chiudere la Milestone 3 con tre protezioni:
- **auto-lock**: il vault si blocca da solo per inattività, al blocco dello schermo (o alla sospensione) e,
  opzionalmente, alla minimizzazione;
- **limiti di risorse in apertura**: un file `.kdbx` ostile non può bloccare il computer chiedendo una derivazione
  della chiave enorme o un file gigantesco;
- **fuzzing dell'intero parsing**: `Vault::open_bytes`, oltre all'header.

## Contesto e vincoli (confermati)
- **Macchina di sviluppo**: Linux, ~1,3 GB liberi, senza librerie Tauri e senza nightly. Il crate app e il fuzzing si
  compilano solo in CI; il core si testa in locale.
- **Decisioni dell'utente**:
  - (a) blocco schermo su Windows e macOS tramite un **mini-crate isolato `crates/os`**: l'`unsafe` è permesso solo lì,
    ogni blocco è commentato e testato; core e app restano `forbid(unsafe_code)`;
  - (b) **limiti generosi** in apertura: memoria Argon2 ≤ 2 GiB, iterazioni Argon2 ≤ 1.000, parallelismo ≤ 256, round
    AES-KDF ≤ 2.000.000.000, file ≤ 512 MiB.
- **Impostazioni già esistenti** (M1): `idle_lock_mins` (1–240, default 5), `lock_on_screen_lock` (default sì),
  `lock_on_minimize` (default no).
- **Autorizzato**: commit e push su `milestone-3c`. **Non autorizzato**: push su `main`.

## Criteri di accettazione
1. **Limiti in apertura (core)**.
   - `keyrust_core::kdbx::limits::Limits` con i valori di default decisi dall'utente.
   - `Vault::open`, `Vault::open_bytes` e `Vault::reload` li applicano **prima** di qualsiasi derivazione della chiave.
     - **Lettura limitata**: il file viene aperto una volta; dalla stessa handle si legge `metadata` e poi si legge con
       `take(max + 1)`. Se si supera il limite, anche perché il file è cresciuto nel frattempo, il risultato è
       `TooExpensive`: nessuna allocazione illimitata.
     - Poi la KDF, dall'header.
     - KDBX 4: `kdbx::header::peek`.
     - KDBX 3.1: l'header viene **validato per intero come lo legge keepass-rs** (`kdbx::header::kdbx3_transform_rounds`,
       ispezione M3C-R01/R02): campi 3 e 10 di 4 byte, 6 di 8 byte, id oltre 10 rifiutati, fine header entro 64 KiB,
       e del `TransformRounds` conta l'**ultima** occorrenza (come keepass-rs). Un header malformato dà `InvalidFormat`
       prima di keepass-rs (che altrimenti andrebbe in panic); i round vengono confrontati con i limiti.
   - Superare un limite dà `VaultError::TooExpensive` (nuovo), che in IPC diventa `{ kind: "tooExpensive" }` e in UI
     "This database asks for more memory or time than Keyrust allows."
   - `Vault::open_bytes_with_limits(bytes, password, &Limits)` è pubblica, per il fuzzing e i test.
   - **Controlli di modifica esterna limitati** (`file::unchanged`, usato al salvataggio e da `check_file`):
     - prima confronta la lunghezza da `metadata`;
     - poi calcola SHA-256 in streaming leggendo al massimo la lunghezza attesa più un byte;
     - un file più lungo o più corto risulta "cambiato" senza leggerlo tutto.
   - **Limiti coerenti con ciò che Keyrust crea**: `kdf::validate` riceve gli stessi tetti di apertura (memoria
     ≤ 2048 MiB, iterazioni ≤ 1000, parallelismo ≤ 256).
     - `create`, `set_kdf` e la calibrazione non possono quindi produrre un database che Keyrust poi rifiuta.
     - Il frontend ha gli stessi `max` nei campi, sia nella creazione sia nelle impostazioni del database, e porta i
       valori digitati entro i limiti prima di inviarli (`clampKdf`, ispezione M3C-R05).
     - Un database esistente oltre i limiti, fatto con un altro client, non si apre e mostra il messaggio
       `tooExpensive`: documentato.
   - **Decompressione — limite noto, documentato**:
     - keepass-rs decomprime solo dopo aver verificato l'HMAC dell'header e dei blocchi con la chiave dell'utente:
       un payload "bomba" richiede la master password, e chi la possiede ha già accesso a tutto il vault;
     - i parametri KDF invece si leggono prima dell'autenticazione e un provider di sync ostile può alterarli: per
       questo sono limitati;
     - `SECURITY.md` registra il limite, che viene segnalato upstream a keepass-rs come richiesta di un tetto di
       decompressione.
   - Test:
     - file più lunghi del limite (anche cresciuti tra `metadata` e lettura, tramite l'hook di test esistente) →
       `TooExpensive` senza leggere oltre il limite;
     - `unchanged` su un file più lungo dello stampo → "cambiato";
     - `kdf::validate` rifiuta i parametri oltre i tetti;
     - header costruiti oltre ciascun limite (Argon2 memoria / iterazioni / parallelismo, AES-KDF in KDBX 4 e 3.1) →
       `TooExpensive` in pochi millisecondi (nessuna derivazione eseguita);
     - le fixture e i database creati da Keyrust restano apribili.
2. **Fuzzing del parsing completo**.
   - Nuovo target `fuzz/fuzz_targets/vault_open_bytes.rs`: `Vault::open_bytes_with_limits` con password fissa e limiti
     **minimi** (memoria Argon2 ≤ 1 MiB, iterazioni ≤ 2, AES ≤ 1.000 round, file ≤ 1 MiB). Così ogni esecuzione è
     veloce e il fuzzer esplora il parsing oltre la KDF (blocchi HMAC, decompressione, XML).
   - **Corpus iniziale** `fuzz/corpus/vault_open_bytes/`: piccoli database KDBX 4 (Argon2id 1 MiB / 1 iterazione e
     AES-KDF 100 round, con voci, gruppi, cronologia, allegati), committati.
     - Sono generati **direttamente con keepass-rs** (`DatabaseConfig`), non con `Vault::create`, i cui parametri minimi
       sono più alti, da un test `#[ignore]` del core (`fuzz_seeds`).
     - Lo stesso test verifica che ogni seme si apra con la password e i limiti esatti del target.
   - **Copertura reale**: le mutazioni casuali di un file cifrato si fermano alla verifica HMAC, quindi il target su file
     interi fuzza header, KDF e controlli di integrità.
   - **Target strutturato `vault_roundtrip`** (con `arbitrary`), per il parsing a valle:
     - genera da input arbitrario un modello (gruppi annidati, voci con campi e stringhe Unicode qualsiasi, anche
       lunghe o con caratteri di controllo, campi personalizzati, cronologia, allegati);
     - lo salva con keepass-rs (KDF minima), lo riapre con `Vault::open_bytes_with_limits` e verifica che titoli,
       campi e numero di voci coincidano;
     - così si fuzzano scrittura e lettura di XML e decompressione su contenuti estremi.
   - **XML ostile oltre l'HMAC**: keepass-rs non espone un parser XML pubblico e quel percorso richiede la chiave
     dell'utente. Resta documentato come limite (`fuzz/README.md`) ed è da proporre upstream.
   - **CI**:
     - il job `fuzz` esegue tutti i target (60 s ciascuno);
     - `workflow_dispatch` accetta l'input `fuzz_seconds` per una **corsa lunga manuale**;
     - gli eventuali crash vengono caricati come artifact.
   - `fuzz/README.md` è aggiornato: il rinvio "Database::parse arriva in Milestone 3" è sostituito dalla procedura.
   - Ogni crash trovato durante la milestone viene corretto, oppure, se è in keepass-rs, documentato con una
     mitigazione (limite o controllo preventivo) e segnalato upstream.
3. **Mini-crate `crates/os`** (`keyrust-os`).
   - API:
     - `pub fn screen_locked() -> Option<bool>`, dove `None` significa "non determinabile su questa piattaforma o in
       questa sessione";
     - `pub fn on_suspend(callback) -> Option<Guard>`, solo su Windows (`PowerRegisterSuspendResumeNotification`); il
       `Guard` de-registra al drop.
   - **Windows**: `OpenInputDesktop(0, FALSE, DESKTOP_SWITCHDESKTOP)` fallisce quando la sessione è bloccata o sul
     desktop sicuro (UAC): in entrambi i casi il risultato è "bloccato", scelta prudente. L'handle viene chiuso con
     `CloseDesktop`. Dipendenza: `windows-sys`.
   - **macOS**: `CGSessionCopyCurrentDictionary()` e la chiave `CGSSessionScreenIsLocked`. Dipendenza:
     `core-foundation`, rilasciando il dizionario secondo la create rule.
   - **Linux**: restituisce `None`; lì se ne occupa il D-Bus del crate app.
   - Lint propri: `unsafe_code = "deny"` a livello di crate e `#[allow(unsafe_code)]` solo sulle funzioni FFI, ognuna con
     un commento `// SAFETY:`; gli altri lint del workspace sono replicati.
   - Test: `screen_locked()` non va in panic e, sui runner CI (sessione non bloccata), non restituisce `Some(true)`.
   - `cargo deny` controlla le nuove dipendenze. `SECURITY.md` e il README dicono esattamente dove si trova l'`unsafe`.
4. **Auto-lock, gestito dal Rust** (`crates/app/src/autolock.rs`). Il vault viene chiuso nel processo Rust anche se il
   webview è bloccato o compromesso.
   - **Generazione di sessione nel backend**:
     - `VaultState` ha un contatore incrementato da ogni lock (manuale, inattività, schermo, minimizzazione);
     - `unlock` e `create` lo catturano prima della derivazione della chiave e installano il vault, sotto il mutex,
       solo se è invariato; altrimenti scartano il vault appena aperto, che viene azzerato, e rispondono `locked`;
     - così un blocco schermo durante uno sblocco non viene annullato dallo sblocco che termina dopo;
     - l'installazione riuscita azzera il timer di inattività.
   - Test (logica pura): un'installazione con generazione obsoleta viene scartata.
   - **Inattività**:
     - il frontend chiama il comando `activity` a ogni input (tasti, puntatore, rotella), al massimo una volta ogni
       15 s;
     - un thread controlla ogni 5 s e, con un vault aperto e un'inattività di almeno `idle_lock_mins`, esegue il
       lock (drop del `Vault`, svuotamento degli appunti) ed emette l'evento `locked` con `{ reason: "idle" }`;
     - il locker **ricontrolla la scadenza sotto il mutex della sessione** prima di bloccare, e l'installazione di un
       vault aggiorna l'attività sotto lo stesso mutex: una richiesta `idle` rimasta in coda (salvataggio lento, nuovo
       sblocco) non blocca una sessione in uso (ispezione M3C-R03).
   - **Blocco schermo** (se `lock_on_screen_lock`):
     - Linux: un thread D-Bus (`zbus`, API sincrona) ascolta `org.freedesktop.ScreenSaver.ActiveChanged(true)` sul bus
       di sessione e `org.freedesktop.login1.Manager.PrepareForSleep(true)` sul bus di sistema (anche la sospensione
       blocca). Se un bus non è disponibile la funzione si degrada con un `warn`, senza errori.
     - Windows e macOS: polling di `keyrust_os::screen_locked()` ogni 2 s.
     - Reason `screenLock`.
   - **Sospensione** (sempre, indipendente dal blocco schermo e dall'impostazione `lock_on_screen_lock`):
     - **Linux**: `PrepareForSleep(true)` di logind blocca **prima** della sospensione.
     - **Windows**: `PowerRegisterSuspendResumeNotification` con callback (`DEVICE_NOTIFY_CALLBACK`, nessuna finestra
       né message loop), in `crates/os`. Su `PBT_APMSUSPEND` / `PBT_APMRESUMEAUTOMATIC` il vault si blocca
       **all'entrata in sospensione**, o al più tardi al risveglio, indipendentemente dalla durata.
     - **macOS**: niente notifiche native, che richiederebbero un observer Objective-C su una run loop, cioè molto più
       `unsafe`. Si usa un **rilevatore di salti d'orologio**:
       - un thread dedicato gira ogni **1 s** e annota `SystemTime`;
       - se fra due giri passano più di **4 s**, la macchina è stata sospesa (il thread non gira durante il sonno) e il
         vault si blocca **subito al risveglio**, reason `suspend`.
       - Lo stesso rilevatore resta attivo come rete di sicurezza anche su Linux e Windows.
     - **Rischio residuo documentato (solo macOS)**: un ciclo di sospensione e risveglio sotto i 4 s di tempo reale con
       "richiedi la password al risveglio" disattivato non viene rilevato. Sono possibili solo cicli quasi istantanei,
       che sui Mac attuali non durano meno di diversi secondi tra ingresso e uscita.
     - Un cambio manuale dell'orologio in avanti di più di 4 s produce un lock superfluo: errore dal lato sicuro.
     - Test:
       - funzione pura `suspended(prev, now, threshold)` con salti in avanti, indietro e normali;
       - in CI su Windows: la registrazione della callback di power riesce e la de-registrazione al drop non va in
         panic.
     - Verifica manuale: sospensione breve (pochi secondi) con "richiedi la password al risveglio" disattivato →
       Keyrust bloccato al ritorno, su Windows e macOS.
   - **Minimizzazione** (se `lock_on_minimize`): su `WindowEvent::Resized`, se `window.is_minimized()`, lock con
     reason `minimize`.
   - La logica di decisione è pura e testata: `idle_expired(last, now, minutes)` e `should_lock(trigger, &Settings)`.
   - Nuovo comando `activity` in `COMMANDS` e `security_config.rs`.
5. **Frontend**.
   - Segnala l'attività (throttle 15 s) solo dentro Tauri e solo con un vault aperto.
   - Ascolta `locked`: azzera lo stato come `lock()` senza richiamare il backend e mostra sull'Unlock la notice
     "Locked after {n} minutes of inactivity." / "Locked because the screen was locked." / "Locked when the computer
     went to sleep." / "Locked when the window was minimized.".
   - Il lock automatico **non chiede** nulla, anche con modifiche non salvate: è la stessa regola del Lock manuale. Le
     modifiche non salvate in quel momento (salvataggio fallito) vanno perse, e la notice lo dice.
   - Test (Vitest):
     - l'evento `locked` simulato porta all'Unlock con la notice giusta e azzera la selezione;
     - il throttle di `activity`;
     - nessuna chiamata `activity` nella demo.
6. **Documentazione**:
   - `SECURITY.md` (auto-lock, limiti, `unsafe` confinato in `crates/os`);
   - `ARCHITECTURE.md`;
   - `fuzz/README.md`;
   - `docs/DEVELOPMENT.md`: checklist manuale (bloccare lo schermo su Windows con Win+L → Keyrust bloccato; inattività
     con 1 minuto; KeePassDX/Strongbox sullo stesso file sincronizzato);
   - `docs/TASKS.md`.

## Approccio e decisioni
- **Auto-lock dal lato Rust**, non con un timer nel webview: il segreto vive nel processo Rust ed è lì che deve sparire,
  anche se il frontend è bloccato.
- **Polling su Windows e macOS** invece delle notifiche di sessione: le notifiche richiederebbero una message loop o un
  observer Objective-C, quindi molto più `unsafe`; un controllo ogni 2 s è sufficiente per un lock.
- **Limiti verificati sull'header prima della KDF**: costo costante, nessuna allocazione dipendente dall'input.
- **Fuzzing con limiti minimi**: senza, ogni input mutato potrebbe chiedere secondi di KDF e il fuzzer non andrebbe
  oltre l'header. Il target strutturato copre ciò che le mutazioni dei file cifrati non raggiungono.
- **Minaccia "provider di sync ostile"**: può alterare ciò che precede l'autenticazione (header, KDF, dimensione), ma
  non contenuti che passino l'HMAC senza la master password. I limiti coprono la prima parte; per la seconda il
  limite è documentato.

## Non-goals
- Merge delle modifiche esterne; key file e YubiKey; tema scuro; firma del codice e release (Milestone 4).
- Inattività a livello di sistema (input in altre applicazioni): conta solo l'attività dentro Keyrust.

## Rischi
- **Rilevamento del blocco schermo su desktop Linux non standard** (Wayland senza `org.freedesktop.ScreenSaver`):
  degrado documentato; restano l'inattività e la sospensione (logind).
- **UAC su Windows** fa sembrare la sessione bloccata: Keyrust si blocca, che è il lato sicuro dell'errore.
  Documentato.
- **Crash di keepass-rs trovati dal fuzzing**: correzione dove possibile nel nostro codice (controlli preventivi),
  altrimenti issue upstream e annotazione nel log.
- **Disco** (~1,3 GB): nessuna build locale di app, os o fuzz; `target/` del core ripulito alla fine.

## Verifica
Locale:
```
CARGO_PROFILE_DEV_DEBUG=line-tables-only cargo test -p keyrust-core
CARGO_PROFILE_DEV_DEBUG=line-tables-only cargo test -p keyrust-core --test fuzz_seeds -- --ignored   # rigenera i semi
cargo clippy -p keyrust-core --all-targets -- -D warnings && cargo fmt --all --check && cargo deny check
(cd crates/app/ui && pnpm check && pnpm lint && pnpm format:check && pnpm test && pnpm build)
```
Remota:
- CI su `milestone-3c` tutta verde: crate `os` e app compilati e testati sui 3 OS, job `fuzz` con entrambi i target;
- una corsa lunga manuale (`workflow_dispatch`, `fuzz_seconds` = 1800) senza crash, oppure con i crash corretti.

Manuale (utente): checklist in `docs/DEVELOPMENT.md`.

## Toolchain per provider
- **Builder** (Claude, host): Rust 1.99.1 (solo il core in locale), Node 24, pnpm 12.15; rete verso crates.io, npm e
  api.github.com; push via SSH (account `downlevel`).
- **Reviewer/Inspector** (Codex, sandbox read-only): legge file e diff con comandi di sola lettura; non esegue i comandi
  di prova, build o test (li esegue l'host). Non raggiunge GitHub: lo stato della CI viene fornito dall'host.
