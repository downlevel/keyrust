# Piano — Milestone 4a (accessibilità, e2e smoke, documentazione definitiva) di Keyrust

Riferimenti: `docs/TASKS.md` (Milestone 4: audit accessibilità, e2e smoke, `SECURITY.md` definitivo, README curato),
`docs/PRD.md` (requisiti di accessibilità, "Domande aperte"), `docs/DESIGN.md` § Accessibilità, `docs/ARCHITECTURE.md`,
`SECURITY.md`, `README.md`, `.github/workflows/ci.yml`. Base: `main` @ `a747fef`. Branch: `milestone-4a`.

La Milestone 4 è divisa in due: **4a** (questo piano) chiude qualità e documentazione; **4b** (pipeline di release,
firma degli artefatti, firma codice Windows/macOS, build riproducibili, `v0.1.0`) è un piano separato.

## Obiettivo
1. L'interfaccia rispetta il requisito di accessibilità del PRD (tastiera completa, focus visibile, contrasto WCAG AA,
   ARIA sui componenti, `prefers-reduced-motion`), con una **verifica automatica in CI** che impedisce le regressioni.
2. Un **test e2e smoke** guida l'app Tauri vera (backend Rust + WebKitGTK) su Linux in CI, su un database reale.
3. `SECURITY.md` e `README.md` diventano **definitivi** per la v0.1.0 (non più "Draft"/"pre-alpha Milestone 2").
4. Le due **domande aperte del PRD** sono chiuse; per KDBX 3.1/4.0 l'utente viene avvisato della conversione.

## Contesto e vincoli (confermati)
- **Macchina di sviluppo** (memoria `keyrust-workflow`): Linux, disco piccolo (38 GB, ~5 GB liberi ora), niente
  WebKitGTK → il crate app non si compila né si avvia qui; nessuna build Tauri locale. Il frontend (Vite, Vitest) e
  Chromium via Playwright invece girano qui: Playwright **1.48.2** e il browser `chromium-1140` sono già nella cache
  (`~/.cache/ms-playwright`). L'e2e Tauri si verifica **solo in CI**.
- **Macchina di test dell'utente**: PC Windows aziendale, nessuna installazione non firmata, nessuna modifica alla
  sicurezza. Strumenti già presenti: Assistente vocale (Narrator), `pnpm tauri dev`.
- **Decisioni dell'utente (2026-10-28)**:
  - (a) e2e: il database si apre da un **file recente pre-caricato** in un `XDG_CONFIG_HOME` temporaneo; **nessun
    hook di test** nel codice di produzione; la creazione di un database resta fuori dall'e2e;
  - (b) audit automatico: **axe-core + Playwright sulla build demo** (Chromium), CI fallisce su violazioni
    `serious`/`critical`; in più controlli manuali da tastiera e con Narrator;
  - (c) screenshot del README: **Playwright sulla demo**, PNG committati;
  - (d) KDBX 3.1/4.0: **avviso non bloccante** all'apertura ("salvando, il database diventa KDBX 4.1").
- **Fatti dal codice**:
  - percorsi aperti solo da dialog o file recenti: `AllowedPaths` è popolato da `choose_*` e da `recent_files`
    (`crates/app/src/commands.rs:54,159-170,251`); le impostazioni sono in `app_config_dir()/settings.json`
    (`crates/app/src/lib.rs:115`), che su Linux segue `XDG_CONFIG_HOME` → l'e2e non richiede hook;
  - `DatabaseInfo.version: (u16, u16)` esiste già (`crates/core/src/model.rs:145`), ma `Vault::write` porta la
    versione a 4.1 **prima** di serializzare e sostituire il file (`crates/core/src/vault.rs:297-301`) e non la
    ripristina se il salvataggio fallisce → serve una piccola correzione nel core (§ 4.2, revisione M4A-01);
  - il webview **riceve** password in chiaro in tre casi: modifica di una voce (`get_entry_for_edit` → `DraftView`,
    `crates/app/src/commands.rs:404`), rivelazione esplicita (`reveal_field`) e generatore; liste e dettaglio non le
    contengono e la copia negli appunti avviene in Rust (revisione M4A-02);
  - ARIA già presente: `tree`/`treeitem` con roving tabindex (`SidebarTree.svelte`), `listbox`/`option`
    (`VaultScreen.svelte`, `EntryListItem.svelte`), `aria-live`/`role=status`/`role=alert` in Toast, Banner, Unlock,
    Create, Settings; `:focus-visible` globale e una regola `prefers-reduced-motion` in `app.css` → il lavoro è un
    **audit con correzioni mirate**, non una riscrittura;
  - la demo (`DemoVault`) si attiva quando `isTauri()` è falso (`crates/app/ui/src/lib/api/index.ts:15`), quindi anche
    in `vite preview` della build statica; password demo `secret`;
  - fixture: `tests/fixtures/keepassxc-rich.kdbx` (KDBX 4) e `keepassxc-kdbx31.kdbx`, password
    `keyrust-test-password` (`tests/fixtures/README.md`).
- **Autorizzato**: commit e push su `milestone-4a`. **Non autorizzato**: push su `main` (PR e merge li fa l'utente).
  L'implementazione parte solo dopo l'approvazione del piano e la conferma dell'utente.

## Criteri di accettazione

### 1. Audit di accessibilità
1. **Harness Playwright** in `crates/app/ui/e2e-web/` (nome distinto dall'e2e Tauri):
   - dipendenze dev esatte: `@playwright/test@1.48.2` (stesso browser già in cache) e `@axe-core/playwright` della
     serie compatibile (versione esatta nel lockfile);
   - `playwright.config.ts` con `webServer` = `pnpm build && pnpm preview --port 4173 --strictPort` (build demo
     statica, CSP di produzione non applicata: è il webview a imporla, non Vite), un solo progetto Chromium,
     viewport 1100×720 (la finestra Tauri di default);
   - script `pnpm test:web` (`playwright test -c e2e-web`); escluso da `pnpm test` (Vitest) e dal lint dei sorgenti
     solo se necessario, mai dal typecheck.
2. **Scansione axe** (`e2e-web/a11y.spec.ts`) con tag `wcag2a`, `wcag2aa`, `wcag21a`, `wcag21aa`, su ogni stato:
   Unlock (vuoto, con file recenti, password errata), Create (passi del wizard), Vault (lista, voce selezionata,
   ricerca senza risultati, cestino, gruppo vuoto), Entry editor (nuova voce e modifica), popover del Generator,
   Settings (ogni scheda), pannello Shortcuts, Command palette, ConfirmDialog, Banner (es. modifica esterna simulata
   dalla demo se disponibile, altrimenti saltato con motivazione nel test), Toast.
   - Il test **fallisce** su qualunque violazione d'impatto `serious` o `critical`; quelle `moderate`/`minor` vengono
     stampate e annotate e vanno corrette o motivate in `docs/ACCESSIBILITY.md`.
   - Nessuna regola axe disattivata globalmente; un'eccezione puntuale è ammessa solo con commento nel test e voce in
     `docs/ACCESSIBILITY.md`.
3. **Tastiera e ordine di tab** (`e2e-web/keyboard.spec.ts`), asserzioni sull'elemento attivo:
   - Vault: Tab percorre nell'ordine ricerca → albero dei gruppi (**una** tappa) → lista voci (**una** tappa) →
     azioni del dettaglio; Shift+Tab percorre lo stesso ordine al contrario;
   - albero: frecce Su/Giù cambiano gruppo, Home/End vanno al primo/ultimo; se l'albero ha nodi espandibili,
     Destra/Sinistra espandono/comprimono (APG Tree View), altrimenti nessun `aria-expanded` è esposto;
   - lista: frecce Su/Giù, Home/End, Invio apre l'editor;
   - dialog (Settings, Confirm, Palette): il focus entra nel dialog, resta intrappolato, Esc chiude e il focus torna
     all'elemento che l'ha aperto;
   - ogni azione del menu "Keyboard Shortcuts" raggiungibile anche senza mouse: le scorciatoie JS documentate in
     `commands.json` vengono invocate una per una e producono l'effetto atteso (quelle solo native del menu sono
     escluse ed elencate).
4. **Focus visibile** (revisione M4A-03): per ogni elemento interattivo raggiunto col Tab nelle schermate sopra:
   - l'indicatore è **reso davvero**: sull'elemento o su un antenato dichiarato nel test (es. il wrapper
     `focus-within` di SecretInput, mappa esplicita elemento → portatore dell'indicatore) c'è un `outline` con
     `outline-width` > 0, `outline-style` ≠ `none` e colore con alfa 1, **oppure** un `box-shadow` con spread > 0 e
     colore con alfa 1;
   - l'indicatore **compare o cambia con il focus da tastiera**: gli stili calcolati del portatore prima del focus e
     dopo il Tab sono diversi (un'ombra decorativa preesistente e immutata non conta);
   - il **colore effettivo** dell'indicatore (quello calcolato, non il token di base) ha contrasto ≥ 3:1 con lo sfondo
     adiacente calcolato del portatore (primo antenato con sfondo opaco). Il colore calcolato viene convertito in sRGB
     nel test con la stessa funzione del punto 5 (esportata da un modulo condiviso, `src/lib/a11y/contrast.ts`);
   - un portatore che non soddisfa una delle tre condizioni fa fallire il test con il selettore dell'elemento.
5. **Contrasto dei token** (`src/tests/contrast.test.ts`, Vitest, nessun browser):
   - elenco esplicito delle coppie testo/sfondo usate dai componenti (foreground, muted-foreground su background,
     surface, sidebar, muted/accent, selection; destructive/warning/success text sui rispettivi sfondi soft; avatar;
     toast) con la soglia corretta (4,5:1 testo normale, 3:1 testo grande ≥ 18,66 px bold o ≥ 24 px, 3:1 componenti
     non testuali: bordi degli input, anello di focus, segmenti dello StrengthMeter rispetto allo sfondo);
   - i valori sono letti dal CSS di Tailwind v4 (`tailwindcss/theme.css`, colori `oklch`) e convertiti in sRGB nel
     test (conversione OKLCH → OKLab → sRGB lineare → luminanza relativa WCAG): niente valori copiati a mano;
   - ogni coppia che non passa viene corretta cambiando il **token** in `app.css` (non la singola classe), poi
     ricontrollata da axe. Caso noto da verificare: `muted-foreground` (zinc-500) su `muted`/`accent` (zinc-100).
6. **Reduced motion**: con `reducedMotion: 'reduce'` emulato, aprire Settings, Palette, Toast e popover non produce
   animazioni/transizioni > 0,01 s (durate calcolate da `getComputedStyle` sugli elementi animati) e nessuno
   `scroll-behavior: smooth`; senza emulazione le animazioni restano.
7. **ARIA**: correzioni emerse dall'audit; in particolare verificare e, se manca, aggiungere:
   - albero: `aria-level`, `aria-setsize`/`aria-posinset` se il raggruppamento visivo non è DOM-annidato, nome
     accessibile di ogni `treeitem` comprensivo del conteggio ("Work, 12 entries") senza duplicazioni lette;
   - lista: `aria-activedescendant` **oppure** roving tabindex (uno solo dei due, coerente), `aria-selected` su ogni
     `option`, nome della lista che riflette il gruppo o la ricerca corrente;
   - annunci `aria-live`: esito di copia ("Password copied, clears in 20 s"), salvataggio, lock automatico e risultati
     della ricerca (conteggio, `polite`, con debounce per non annunciare a ogni tasto); errori di Unlock/Create con
     `role="alert"` e collegati al campo via `aria-describedby`;
   - StrengthMeter: la robustezza è anche testo (già nel DESIGN) ed esposta come `aria-valuetext` o testo associato;
   - SecretInput: il pulsante mostra/nascondi ha `aria-pressed` e un nome che non cambia lingua/forma a metà.
8. **`docs/ACCESSIBILITY.md`** (nuovo, in inglese): cosa viene verificato automaticamente e come, la checklist
   manuale, le eccezioni motivate, i limiti noti (es. la CSP/WebKitGTK e WebView2 non sono il Chromium del test;
   annunci di Narrator/Orca dipendono dal motore).
9. **Checklist manuale** in `docs/DEVELOPMENT.md` per l'utente su Windows, senza installazioni: con `pnpm tauri dev`,
   Narrator (Ctrl+Win+Invio) su Unlock → Vault → copia password → editor → Settings; zoom al 200% (Ctrl+più) senza
   perdita di contenuto; contrasto elevato di Windows (solo osservazione, nessun requisito V1 oltre "resta usabile").

### 2. Test e2e smoke (Tauri, Linux, CI)
1. **Pacchetto** `crates/app/e2e/` con `package.json` proprio (pnpm, `packageManager` uguale alla UI), lockfile
   committato, dipendenze esatte: WebdriverIO (`@wdio/cli`, `@wdio/local-runner`, `@wdio/mocha-framework`,
   `@wdio/spec-reporter`, `@wdio/globals`) e TypeScript. Il config segue la guida ufficiale Tauri 2 "WebdriverIO":
   capability `tauri:options.application` = `target/debug/keyrust`, `tauri-driver` avviato in `beforeSession` e
   chiuso in `afterSession`.
2. **Preparazione dati, senza hook** (`e2e/setup.ts`, in `onPrepare`):
   - crea una cartella temporanea; vi **copia** `keepassxc-rich.kdbx` e `keepassxc-kdbx31.kdbx` (le fixture
     originali non vengono mai modificate);
   - scrive `$XDG_CONFIG_HOME/io.github.downlevel.keyrust/settings.json` con i due percorsi in `recent_files`
     (stesso schema di `keyrust_core::settings::Settings`, campi mancanti lasciati ai default via `serde(default)`),
     e `idle_lock_mins` al default;
   - esporta `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_CACHE_HOME` temporanei per l'app (nessuno stato del runner
     toccato); il percorso esatto della config dir è verificato all'avvio dal test (se il file recente non compare,
     il test fallisce con un messaggio che stampa la directory attesa).
3. **Scenario** (`e2e/specs/smoke.e2e.ts`), un solo avvio dell'app:
   1. la schermata Unlock elenca i due file recenti;
   2. scelta di `keepassxc-rich.kdbx`, password errata → messaggio d'errore (`role=alert`), il campo resta
      focalizzato;
   3. password corretta → Vault visibile, la lista contiene le voci attese della fixture (titoli dal
      `tests/fixtures/README.md`), nessuna password in chiaro nel DOM prima di modifica o rivelazione (revisione
      M4A-04): uno script nel webview percorre l'intero documento (anche i nodi nascosti e i portali) e cerca le
      password note delle voci della fixture in `textContent` di ogni nodo, in **ogni valore di attributo** e nel
      valore corrente (`.value`) di ogni `input`/`textarea`; il controllo si ripete dopo la selezione di una voce
      (dettaglio aperto, password mascherata);
   4. ricerca per una parte del titolo → la lista si riduce alla voce attesa; cancellare la ricerca la ripristina;
   5. modifica del titolo di una voce e salvataggio tramite un percorso gestito dal **frontend** (pulsante Save
      dell'editor/Command palette, non l'acceleratore nativo del menu GTK, che WebDriver non può premere) → indicatore
      "unsaved" sparito, stato "Saved";
   6. Lock → torna la schermata Unlock;
   7. apertura di `keepassxc-kdbx31.kdbx` → banner informativo di conversione (vedi § 4) visibile; nessun
      salvataggio.
   - Dopo la sessione, un passo della CI esegue `keepassxc-cli show` sulla **copia** salvata e verifica il nuovo
     titolo: la prova che il file scritto dall'app vera resta leggibile da KeePassXC.
4. **CI**: nuovo job `e2e` (ubuntu-24.04), su ogni evento di CI (non solo PR) perché serve a verificare il branch:
   - prerequisiti Tauri come nel job `app` + `webkit2gtk-driver`, `xvfb`, `keepassxc`;
   - `cargo install tauri-driver --locked --version <esatta>` con cache;
   - build `pnpm tauri build --debug --no-bundle` (nessun installer, nessuna firma);
   - `xvfb-run -a pnpm --dir crates/app/e2e test`;
   - in caso di errore carica come artefatto log dell'app (`KEYRUST_LOG=info`, stderr) e screenshot WebDriver
     (retention 7 giorni). I log non contengono valori di campo (policy M1), ma la password delle fixture è pubblica.
   - `timeout-minutes` dichiarato; il job `e2e` non sostituisce il job `app`.
5. **Rischi dichiarati**: WebKitWebDriver può richiedere ritardi per il rendering; si usano solo attese su
   condizioni (`waitUntil`/`waitForDisplayed`), mai `pause` fisse. La KDF della fixture deve stare sotto i limiti
   M3c (lo è: creata da `keepassxc-cli` con i default).

### 3. Screenshot per il README
1. `e2e-web/screenshots.spec.ts`, eseguito **solo** con `pnpm screenshots` (non in CI, non in `test:web`): demo,
   viewport 1100×720, `deviceScaleFactor: 2`, font bundlati già caricati (`document.fonts.ready`), animazioni
   disattivate.
2. PNG in `docs/screenshots/`: `unlock.png`, `vault.png`, `editor-generator.png`, `settings.png`; ottimizzati
   (nessun file > 400 KB; se serve `oxipng`/`pngquant` locale, altrimenti scala 1,5).
3. I dati mostrati sono quelli della demo (nessun dato reale, nessun nome di persona reale oltre a quelli già nella
   demo); nessuna password in chiaro negli screenshot.

### 4. Domande aperte del PRD
1. **Conflitti di sync** → decisione registrata nel PRD (§ Domande aperte diventa "Decisioni"): in V1 Keyrust rileva
   la modifica esterna (lunghezza + SHA-256, prima e dopo) e chiede *Reload* / *Keep mine*; nessun merge (già fuori
   scope). Nessun cambiamento al codice.
2. **KDBX 3.1 e 4.0** → decisione: si aprono in lettura e scrittura; al primo salvataggio diventano KDBX 4.1. Avviso:
   - frontend: dopo l'unlock, se `DatabaseInfo.version` < (4, 1), banner **informativo** (tono neutro, non errore)
     nel Vault: "This database uses KDBX {version}. Saving converts it to KDBX 4.1, which very old KeePass clients
     cannot open." con azione "Dismiss";
   - il banner sparisce dopo il primo salvataggio riuscito (la versione diventa 4.1: rileggere `DatabaseInfo` dopo
     il save o aggiornarla localmente) o con Dismiss; ricompare al prossimo unlock se il file non è stato salvato;
   - non blocca nulla, non chiede conferme, non cambia il salvataggio;
   - priorità rispetto agli altri banner (modifica esterna, salvataggio fallito, lock): quelli esistenti hanno
     precedenza; l'avviso di versione non ne nasconde nessuno;
   - stringa in `messages/en.json` (i18n), test Vitest su: 3.1 → banner; 4.0 → banner; 4.1 → nessun banner;
     sparisce dopo save; Dismiss; la demo resta 4.1 (nessun banner);
   - se l'IPC `database_info` non è già chiamato dopo l'unlock, si usa il comando esistente, senza nuovi comandi
     IPC.
   - **Correzione nel core** (revisione M4A-01): `Vault::write` non modifica più `self.db.config.version` prima di
     sapere se il file è stato sostituito. Serializza una configurazione con la versione 4.1 (o imposta 4.1 e la
     **ripristina** su ogni ritorno prima della sostituzione: errore di serializzazione, `TooExpensive`,
     `ExternalChange`, `Io`); la versione resta 4.1 solo quando il file è stato sostituito (`Ok` e `NotDurable`).
     Così `DatabaseInfo.version` è sempre la versione del file su disco.
     - Test del core: apertura della copia di `keepassxc-kdbx31.kdbx` → `info().version == (3, 1)`; salvataggio
       rifiutato (`ExternalChange` con il file modificato da fuori, e `TooExpensive` con un `max_file_bytes` ridotto)
       → ancora `(3, 1)` e file su disco invariato; salvataggio riuscito → `(4, 1)` e il file riaperto è 4.1.
     - Test frontend: dopo un salvataggio fallito il banner resta; dopo uno riuscito sparisce.
3. PRD, README e `SECURITY.md` descrivono il comportamento.

### 5. `SECURITY.md` definitivo
Riscritto in inglese, senza "Draft", con questa struttura (contenuto esistente conservato e riorganizzato, nessuna
affermazione non verificabile nel codice):
1. Reporting (invariato) e **Supported versions** aggiornate a `0.1.x` (la tabella dice che fino al rilascio vale
   `main`).
2. **Scope e stato**: non auditato da terzi; cosa copre la v0.1.
3. **Assets** (DB, master password, chiave derivata in memoria, segreti in clipboard, file `settings.json` — non
   segreto, ma rivela percorsi).
4. **Trust boundaries** con diagramma Mermaid: processo Rust ↔ webview (IPC), Rust ↔ file system / provider di sync,
   Rust ↔ clipboard di sistema, Rust ↔ OS (lock schermo, sospensione).
5. **Attaccanti considerati** (tabella: capacità → mitigazione → dove nel codice/test → rischio residuo): ladro del
   file, provider di sync ostile, contenuto del webview compromesso (XSS ipotetico: CSP, capabilities minime,
   percorsi solo da dialog/recenti, nessun segreto nelle liste), persona con accesso fisico a sessione bloccata/
   sbloccata, altri processi dello stesso utente, catena di fornitura.
6. **Mitigazioni** (l'elenco attuale, con riferimenti a file/test). Il confine webview è descritto con esattezza
   (revisione M4A-02): liste, dettaglio e albero non contengono password; copia negli appunti in Rust; il webview
   riceve la password in chiaro solo modificando una voce (`get_entry_for_edit`), rivelandola (`reveal_field`) e
   generandola; da lì vale il limite già scritto sulle stringhe JavaScript non azzerabili, e un webview compromesso
   (XSS ipotetico) può leggere ciò che è aperto in modifica o rivelato e chiamare i comandi IPC consentiti, ma non
   scegliere percorsi di file. Ogni frase di `SECURITY.md` e README che dica di più viene corretta.
7. **Non protegge** (elenco attuale + residui sopra).
8. **Superficie IPC**: elenco dei comandi e cosa espongono (riferimento a `crates/app/src/commands.rs`), test di
   configurazione di sicurezza (`crates/app/tests/`).
9. **Regole di sviluppo** (attuali) e **verifica dei rilasci**: rimando a 4b, senza promettere date; frase al futuro
   esplicita ("Starting with v0.1.0, release artifacts will be signed…").
Coerenza: ogni affermazione è stata confrontata con il codice; dove il codice e `SECURITY.md` divergono si corregge
il documento (o si segnala come bug, fuori scope).

### 6. README curato
In inglese, al posto dell'attuale (status M2 obsoleto):
1. intestazione, badge CI, una frase; **stato**: "alpha — not audited; keep backups" (nessun "production ready");
2. screenshot (`vault.png` grande, gli altri in una riga);
3. **Features** (solo ciò che esiste), **Not yet / out of scope** (dal PRD);
4. **Security in one minute**: 5–7 punti con link a `SECURITY.md`;
5. **Architecture**: diagramma Mermaid (core / app Rust / webview / os / file KDBX / clipboard / OS) + 3 righe;
6. **Compatibility**: KeePassXC (test automatici a due vie), KeePassDX/Strongbox (non ancora verificati
   manualmente: detto esplicitamente finché la voce di TASKS è aperta), KDBX 3.1/4.0 → 4.1;
7. **Keyrust vs KeePassXC**: tabella onesta (auto-type, integrazione browser, keyfile, TOTP, YubiKey, import, KeeShare
   presenti in KeePassXC e assenti in Keyrust v0.1; Keyrust: UI minimale, Rust memory-safe, password assenti da liste e dettagli e copia negli appunti fatta in Rust
   (non "segreti mai nel webview": in modifica, rivelazione e generazione la password passa al webview, revisione
   M4A-02),
   compatibilità verificata in CI) con "as of KeePassXC 2.7"; conclusione: per chi ha bisogno di quelle funzioni,
   KeePassXC resta la scelta giusta;
8. **Building** per Linux/Windows/macOS (comandi brevi, rimando a DEVELOPMENT), **Testing** (core, UI, a11y, e2e);
9. Documentazione, licenza (invariati).
I link relativi sono verificati (vedi Verifica).

### 7. Roadmap e documenti
- `docs/TASKS.md`: spunte per audit, e2e, `SECURITY.md`, README; voci nuove della 4a; 4b invariata.
- `docs/DEVELOPMENT.md`: come eseguire `pnpm test:web`, `pnpm screenshots`, l'e2e (solo CI su Linux, comandi per chi
  ha WebKitGTK), checklist manuale accessibilità.
- `docs/DESIGN.md` § Accessibilità: riferimento a `docs/ACCESSIBILITY.md`, eventuali token cambiati.
- `docs/plans/milestone-4a-review-log.md`: log della revisione e dell'ispezione.

## Approccio e decisioni
- **Due harness distinti, ciascuno per ciò che sa fare**: Playwright/Chromium sulla demo per la copertura completa
  delle schermate (gira in locale e in CI), WebdriverIO/tauri-driver per la prova che backend, IPC e WebKitGTK reali
  funzionano insieme (solo CI). L'audit non dipende dall'e2e.
- **Nessun hook di test nel prodotto**: l'e2e passa per la stessa via di un utente (file recente), così la regola
  "percorsi solo da dialog o recenti" resta senza eccezioni.
- **Correzioni sui token, non sui singoli componenti**, per il contrasto: la V2 (tema scuro) erediterà lo stesso test
  con il secondo set di token.
- **Documenti in inglese** (pubblico del repo), piani e log in italiano (come nelle milestone precedenti).
- **Non-goal**: tema scuro; traduzione italiana; creazione del database nell'e2e; e2e su Windows/macOS (tauri-driver
  non supporta macOS e su Windows richiede msedgedriver allineato: rinviato); release, firma, build riproducibili
  (4b); KeePassDX/Strongbox (task manuale dell'utente); conformità WCAG certificata.

## Rischi residui
- axe vede il DOM in Chromium, non in WebKitGTK/WebView2: differenze di rendering del contrasto sono improbabili
  (stessi colori sRGB), quelle degli annunci dei lettori di schermo restano coperte solo dalla checklist manuale.
- Playwright 1.48.2 è vecchio, scelto per riusare il browser in cache (disco); aggiornarlo è un cambiamento isolato.
- WebKitWebDriver su xvfb può essere instabile: un fallimento intermittente si corregge con attese su condizioni, non
  con retry ciechi; se resta instabile il job si marca come informativo solo con decisione dell'utente.
- Il percorso della config dir Tauri su Linux usa l'identificatore (`io.github.downlevel.keyrust`): se Tauri lo
  calcolasse diversamente, il test lo segnala subito (§ 2.2).

## Toolchain per provider
- **Claude (builder)**: Node/pnpm già presenti; Playwright 1.48.2 + chromium-1140 in cache; Vitest; `cargo test -p
  keyrust-core` e clippy dei crate compilabili qui. Nessuna build Tauri locale. Controllo `df` prima dei passi
  pesanti; dopo i test cancellare `target/debug/incremental` e gli eseguibili di test in `target/debug/deps`.
- **Codex (reviewer/inspector)**: sola lettura del repository; non esegue comandi.

## Verifica
Locale (Linux, questa macchina):
1. `cd crates/app/ui && pnpm install --frozen-lockfile && pnpm check && pnpm lint && pnpm format:check && pnpm test`
   → tutto verde, inclusi `contrast.test.ts` e i test del banner di versione.
2. `pnpm test:web` → axe 0 violazioni `serious`/`critical` su tutti gli stati elencati; tastiera, focus e reduced
   motion verdi. Prima delle correzioni, l'output della prima esecuzione (violazioni trovate) è salvato nel log di
   revisione come baseline: la prova che l'audit misura qualcosa.
3. `pnpm screenshots` → i 4 PNG prodotti, < 400 KB ciascuno, controllati a occhio (nessun segreto in chiaro).
4. `cargo test -p keyrust-core` e `cargo clippy -p keyrust-core --all-targets -- -D warnings` (nessuna regressione;
   nel core cambia solo la gestione della versione in `Vault::write`, con i test del § 4.2); `cargo clippy -p keyrust-app --all-targets --target x86_64-pc-windows-msvc`
   con il finto `llvm-rc` (memoria) se il crate app viene toccato.
5. Link del README/SECURITY: script che estrae i link relativi e verifica l'esistenza dei file (`docs/…`,
   `docs/screenshots/…`, ancore escluse); i diagrammi Mermaid controllati con il renderer di GitHub sulla pagina
   del branch.
6. `pnpm --dir crates/app/e2e install --frozen-lockfile && pnpm --dir crates/app/e2e exec tsc --noEmit` (l'e2e non
   gira qui, ma compila).

CI (su push di `milestone-4a`, stato letto dall'API pubblica di GitHub):
7. Job `core` ×3, `app` ×3 (con `pnpm test:web` aggiunto nel job Linux, Chromium installato con
   `playwright install --with-deps chromium`), `deny`, `fuzz`: verdi.
8. Job `e2e`: verde, con lo scenario completo e il passo `keepassxc-cli show` sul file salvato dall'app che mostra il
   titolo modificato.

Manuale (utente, Windows, nessuna installazione): checklist di accessibilità di `docs/DEVELOPMENT.md` con Narrator e
zoom 200%; esito annotato in `docs/ACCESSIBILITY.md`. Non blocca la PR, ma la voce di TASKS si spunta solo dopo.
