# Piano — Milestone 2 (Design: UI scaffolding) di Keyrust

Riferimenti: `docs/TASKS.md` (Milestone 2), `docs/DESIGN.md`, `docs/ARCHITECTURE.md`, `docs/PRD.md`,
`docs/plans/milestone-1.md`. Base: `main` @ `f5ba055`. Branch: `milestone-2`.

## Obiettivo
Realizzare in Svelte l'interfaccia di Keyrust approvata dall'utente nel prototipo
(https://claude.ai/artifact/7NiAtLZoAk2V5uZmCcyydh): design system, schermate Unlock / Create database / Vault a
3 colonne / Settings, componenti ricorrenti, stati vuoto/errore/caricamento, menu nativo, scorciatoie da tastiera,
pannello "Keyboard Shortcuts", command palette di base e icona dell'app. Tutto gira su **dati demo** dietro
un'interfaccia API identica a quella che in Milestone 3 parlerà con il core Rust, così la Milestone 3 sostituisce
solo l'implementazione.

## Contesto e vincoli (confermati)
- Macchina di sviluppo Linux con **~1,9 GB liberi** e senza librerie di sistema Tauri: il crate `keyrust-app` si
  compila solo in CI (clippy, test, build). Il frontend si verifica in locale (`pnpm check/lint/test/build`).
- L'utente testa su un **PC Windows aziendale**: non installa build non firmate e non cambia impostazioni di sicurezza.
  La verifica visiva avviene quindi su un'**anteprima web** (la build statica del frontend pubblicata come pagina
  privata su claude.ai), dove l'app gira con i dati demo.
- Decisioni dell'utente:
  - (a) l'app compilata funziona in **modalità demo** (qualsiasi password sblocca un database di esempio) con un avviso
    sempre visibile "Demo data";
  - (b) la **command palette** di base è inclusa.
- Direzione visiva approvata: quella del prototipo (zinc + indaco, Inter / JetBrains Mono, raggi 8/12 px, iniziale
  colorata per le voci, toast scuro in basso, banner ambra/rosso).
- **Autorizzato**: commit e push sul branch `milestone-2`. **Non autorizzato**: push su `main` (PR aperta e mergiata
  dall'utente).
- Versioni installate: SvelteKit 2.70.3, Svelte 5.57.2, bits-ui 2.20.2, shadcn-svelte 1.7.1 (stile "vega"),
  `@tauri-apps/api`/`cli` 2.12.4, crate `tauri` 2.12.5, Paraglide JS 2.29.1, Vitest 5.

## Criteri di accettazione
1. **Design system.**
   - I token in `crates/app/ui/src/app.css` seguono `DESIGN.md`:
     - `--primary` è indigo-600, con hover indigo-700 e `--ring` indaco;
     - aggiunti i token semantici `--success` (emerald-600), `--warning` (amber-500) e `--destructive` (red-600),
       ciascuno con la propria variante `-foreground`;
     - testo secondario zinc-500, raggio base 8 px (`--radius: 0.5rem`) e modali a 12 px.
   - Il blocco `.dark` resta come predisposizione, **non attivabile** in V1.
   - I componenti shadcn-svelte necessari (dialog, alert-dialog, popover, tooltip, select, switch, checkbox, slider,
     separator, scroll-area, tabs, collapsible, command, kbd o equivalenti) sono aggiunti con la CLI e usano i token,
     senza colori hard-coded fuori da `app.css`.
   - Fanno eccezione i colori delle iniziali delle voci: una palette di 6 coppie sfondo/testo, definite come token
     `--avatar-N-bg` / `--avatar-N-fg`.
2. **API frontend senza segreti in vista** (`src/lib/api/`).
   - **Tipi** (`types.ts`) allineati a `ARCHITECTURE.md`: `EntrySummary` (senza password), `EntryDetail`
     (`hasPassword`, mai il valore), `GroupNode`, `Field`, `GeneratorOptions`, `StrengthEstimate`, `KdfParams`.
   - **Interfaccia `VaultApi`**: `unlock`, `lock`, `create`, `tree`, `listEntries`, `getEntry`, `revealField`,
     `copyField`, `saveEntry`, `deleteEntry`, `restoreEntry`, `createGroup`, `renameGroup`, `deleteGroup`, `save`,
     `generatePassword`, `estimateStrength`, `benchmarkKdf`.
     - La password arriva al frontend **solo** da `revealField` (azione "mostra") e dall'editor (`getEntry` con
       `includeSecrets` esplicito per la modifica).
     - `copyField` non restituisce il valore.
   - **Implementazione `DemoVault`** (`demo.ts`):
     - dati di esempio inventati (gli stessi del prototipo);
     - latenza simulata di circa 900 ms su `unlock` e `create`;
     - `"wrong"` o una password vuota producono l'errore di password errata;
     - nessuna scrittura su disco, `localStorage`, `sessionStorage`, IndexedDB o appunti;
     - `copyField` simula soltanto la copia.
   - **`AppApi`** (`app.ts`): `appInfo`, `getSettings`, `updateSettings`. Dentro Tauri usa i comandi esistenti
     (`invoke`); fuori da Tauri (anteprima web, test) usa un'implementazione in memoria. Le impostazioni della tab
     General sono quindi **reali** nell'app compilata.
   - Un unico punto di composizione (`src/lib/api/index.ts`) sceglie le implementazioni.
3. **Stato e navigazione.**
   - Un solo route (`+page.svelte`) con una macchina a stati in rune Svelte (`src/lib/state/*.svelte.ts`): `welcome`
     (nessun recente) | `unlock` | `create` | `vault`. Niente routing SvelteKit per schermata, così Tauri e
     l'anteprima web si comportano allo stesso modo.
   - Settings, conferme, generatore e palette sono overlay.
   - Lock (`Ctrl+L`, pulsante nella sidebar, menu) azzera nel frontend la voce selezionata, la password rivelata, la
     bozza dell'editor e i toast, poi torna a Unlock con il database preselezionato. Il campo master password viene
     svuotato dopo ogni tentativo.
     - Lock è immediato e scarta la bozza **senza** conferma: chi blocca vuole far sparire i dati subito.
   - **Risposte asincrone obsolete**:
     - Lo stato tiene due contatori:
       - una **generazione di sessione**, incrementata a ogni unlock, lock, create e cambio di database;
       - una **generazione di selezione**, incrementata a ogni cambio di voce, apertura o chiusura dell'editor e
         annullamento.
     - Ogni chiamata asincrona (`revealField`, `getEntry` con segreti, `listEntries`, `generatePassword`,
       `estimateStrength`, `benchmarkKdf`, `unlock`, `create`, `save`) cattura le generazioni alla partenza e applica
       il risultato solo se coincidono ancora; altrimenti lo scarta.
     - Così una risposta tardiva non può ripopolare una password rivelata, una bozza o una sessione dopo un lock o un
       cambio di voce.
   - **Guardia delle modifiche non salvate**:
     - Una sola funzione di transizione (`leaveEditor`) protegge ogni azione che lascia o sostituisce un editor
       modificato, da mouse, tastiera, palette o menu nativo:
       - selezione di un'altra voce o di un gruppo;
       - New Entry, Edit di un'altra voce, apertura di una voce dalla palette;
       - New / Open / Close Database e Quit dal menu.
     - Mostra un dialog con **Save** / **Discard** / **Keep editing**, e l'azione richiesta prosegue solo dopo Save
       (riuscito) o Discard.
     - Lock è l'unica eccezione, come detto sopra.
4. **Schermate**, conformi a `DESIGN.md` e al prototipo.
   - **Welcome / Unlock**:
     - card centrata con nome e percorso del database;
     - secret input con mostra/nascondi e avviso Caps Lock (`getModifierState`);
     - errore inline;
     - stato "Unlocking…" con spinner e input disabilitati;
     - elenco Recent, i link "Open another file…" e "Create new database";
     - Welcome (primo avvio) senza recenti.
   - **Create database**: file (pulsante "Choose…", nella demo un percorso fittizio), nome, master password con
     conferma e strength meter, sezione Advanced chiusa per default (cifratura, Argon2id memory / iterations /
     parallelism, "Benchmark (1 s)").
   - **Vault a 3 colonne**:
     - sidebar gruppi (`role="tree"`) con contatori, Recycle Bin in fondo, "New group", Settings, Lock, indicatore
       unsaved e stato "Saved";
     - lista voci (`role="listbox"`) con ricerca (titolo / username / URL, mai la password), ordinamento per titolo,
       "New entry";
     - dettaglio con larghezza massima di 720 px: campi label/valore, password mascherata, reveal, copy, URL che si
       apre nel browser (nella demo il link non naviga), note, metadati, Edit, Delete e Restore nel cestino.
   - **Entry editor** inline:
     - campi Title (obbligatorio), Username, Password, URL, Group, Notes;
     - Password con strength meter e popover del generatore;
     - Save e Cancel, `Esc` per annullare, con conferma se ci sono modifiche.
   - **Password generator** (popover dall'editor e comando Tools):
     - anteprima in monospace, rigenera;
     - slider della lunghezza 8–128 con default dalle impostazioni;
     - set di caratteri con almeno uno sempre attivo, "exclude ambiguous";
     - strength meter, "Use password".
   - **Settings** (dialog con tab):
     - General: clipboard 5–300 s, auto-lock 1–240 min, lock on screen lock, lock on minimize; i valori fuori range
       vengono riportati nel range, come in `Settings::normalized`;
     - Database: nome, cifratura, cestino, Advanced Argon2id + Benchmark, "Change master password…" (nella demo apre
       un dialog informativo);
     - About: versione da `appInfo`, licenza, link al repository e a `SECURITY.md`.
   - **Layout stretto**: tra 800 e 1100 px la sidebar si riduce a icone con un toggle; sopra 1100 px le colonne sono
     tre.
   - **Avviso demo**: un badge "Demo data" con una spiegazione accessibile ("No file is opened and nothing is copied
     or saved") è visibile in Unlock, Create e Vault.
5. **Componenti ricorrenti** in `src/lib/components/` (uno per file, riusati dalle schermate):
   - `SidebarTree`, `SearchInput`, `EntryListItem`, `FieldRow`, `SecretInput`, `StrengthMeter`, `Toast`, `Banner`,
     `ConfirmDialog`, `CommandPalette`, `ShortcutsPanel`, `UnsavedDot`, `Wordmark`.
   - `StrengthMeter` ha 4 livelli e l'etichetta testuale (Weak / Fair / Good / Strong), non solo il colore.
   - `Toast` mostra "{Field} copied · clears in {n}s" con il countdown dalle impostazioni, in una regione
     `aria-live="polite"`.
6. **Stati.**
   - Vuoto: vault senza voci ("No entries yet. Create your first one with Ctrl+N"), gruppo vuoto, ricerca senza
     risultati ("No matches for “…”"), nessuna voce selezionata, Welcome senza recenti.
   - Caricamento: unlock, create, salvataggio, benchmark.
   - Errore:
     - password errata;
     - banner "This database was changed by another app" (Reload / Keep mine);
     - banner di errore di salvataggio (Retry / Save as…);
     - file non trovato tra i recenti.
   - La demo permette di far comparire i banner con un comando "Demo: simulate …" presente solo nella palette in
     modalità demo.
7. **Comandi, scorciatoie e menu nativo.**
   - **Unica fonte di verità**: `crates/app/ui/src/lib/commands/commands.json`. Ogni comando ha `id`, `menu`
     (File / Edit / Entry / Group / Tools / Help o nessuno), `labelKey` (chiave Paraglide), `shortcut`
     (`CmdOrCtrl+N`…) e `scope` (`global` | `outsideText`).
   - **Frontend**: un registro (`commands.ts`) legge il JSON e alimenta gli handler da tastiera, il pannello
     "Keyboard Shortcuts" (`Ctrl+/`) e la palette (`Ctrl+K`, che cerca tra le azioni e tra i titoli delle voci).
   - **Rust** (`crates/app/src/menu.rs`):
     - costruisce il menu nativo dallo stesso JSON (`include_str!`) con le etichette prese da
       `crates/app/ui/messages/en.json`;
     - su macOS aggiunge il menu dell'app e su tutte le piattaforme il menu Edit con le voci predefinite (Undo, Redo,
       Cut, Copy, Paste, Select All), così copia e incolla funzionano negli input;
     - un clic emette l'evento `menu` con l'`id` verso la webview, che esegue lo stesso comando della scorciatoia.
   - **Acceleratori nativi** solo per le scorciatoie che non entrano in conflitto con la modifica del testo:
     `CmdOrCtrl+N`, `+S`, `+L`, `+G`, `+/`, `+Shift+N` (nuovo database), `+O`.
   - **Scorciatoie gestite dalla webview** con `scope: "outsideText"`: Copy Password (`Ctrl+C`), Copy Username
     (`Ctrl+B`), Copy URL (`Ctrl+U`), Edit (`Ctrl+E` / `Enter`), Delete (`Del`). Scattano solo quando il focus non è
     in un campo di testo e non c'è testo selezionato.
     - Copy Password si aggancia all'evento DOM `copy`, che le voci Copy predefinite del menu e la combinazione
       nativa generano su tutte le piattaforme, invece che a `keydown`.
   - **Navigazione da tastiera**:
     - `Ctrl+F` mette il focus sulla ricerca;
     - frecce nella lista e nell'albero, `Enter` apre, `F2` rinomina un gruppo, `Esc` chiude overlay ed editor;
     - ordine di tab: sidebar → ricerca → lista → dettaglio;
     - focus ring indaco visibile su ogni elemento interattivo; `prefers-reduced-motion` rispettato.
8. **Icona.**
   - `crates/app/icons/source.svg` viene rifinita: chiave stilizzata bianca su quadrato indaco arrotondato, leggibile
     a 16–32 px.
   - Il set di icone viene rigenerato con `pnpm tauri icon` e committato.
   - `Wordmark` (icona + "Keyrust") è usato in Unlock, Welcome e About.
9. **i18n.** Ogni stringa visibile, comprese le etichette del menu nativo e dei comandi, sta in `messages/en.json`.
   `untranslated-text.test.ts` resta verde e copre i nuovi file `.svelte`.
10. **Sicurezza** (invariata o rafforzata).
    - CSP e capabilities restano invariate: `security_config.rs` è verde.
    - Il frontend non usa `localStorage`, `sessionStorage` né IndexedDB: un test statico lo verifica sui sorgenti.
    - Nessuna risorsa remota: font locali, nessun `http(s)://` nei `src`/`href` di risorse; i link esterni sono solo
      `<a>` verso il sito del progetto.
    - Le password rivelate non entrano in store persistenti, vengono rimosse al lock e al cambio voce, e gli input
      segreti hanno `autocomplete="off"` e `spellcheck=false`.
11. **Test.**
    - Vitest con `@testing-library/svelte` e jsdom sui flussi di `DESIGN.md`:
      - sblocco con errore e successo, lock che azzera lo stato;
      - ricerca e "no matches";
      - selezione e reveal: password mascherata per default e nascosta al cambio voce;
      - copy, con il toast e il countdown verificati con fake timer;
      - editor: Save, Cancel con conferma, Title obbligatorio;
      - guardia: con una bozza modificata, selezionare un'altra voce, New Entry, aprire una voce dalla palette e
        l'evento di menu `close_database` mostrano Save / Discard / Keep editing, e ciascuna scelta fa la cosa attesa;
        Lock invece scarta subito;
      - risposte obsolete, con promesse differite controllate dal test: un `revealField(A)` risolto dopo la selezione di
        B o dopo il lock non mostra nulla; un `getEntry` con segreti risolto dopo il lock non ricrea la bozza; un
        `unlock` risolto dopo il passaggio a un altro database non apre il vault;
      - delete, cestino e restore;
      - generatore: vincoli dei set e lunghezza;
      - Settings: clamp dei valori;
      - scorciatoie: `Ctrl+F`, `Ctrl+N`, `Ctrl+/`, `Ctrl+K`, `Ctrl+C` fuori e dentro un input;
      - palette: filtro di azioni e voci;
      - ruoli ARIA (`tree`, `listbox`, `aria-live`).
    - Test unitari: matcher delle scorciatoie (incluso `CmdOrCtrl` su macOS e non-macOS), registro dei comandi
      (id unici, `labelKey` presenti in `en.json`), stima di robustezza demo.
    - Test Rust (CI, `cargo test -p keyrust-app`): il menu ricavato da `commands.json` ha id unici, ogni `labelKey`
      esiste in `en.json` e ogni acceleratore si parsa.
12. **Documentazione.**
    - `docs/DESIGN.md` aggiornato con le decisioni prese: iniziali colorate, posizione dei pulsanti copy, banner,
      icona.
    - `docs/ARCHITECTURE.md` aggiornato su API frontend, demo e registro dei comandi.
    - `docs/TASKS.md`: Milestone 2 spuntata.
    - `CONTRIBUTING.md`: come aggiungere un comando o una scorciatoia.
    - `docs/DEVELOPMENT.md`: come aprire l'anteprima web.

## Approccio e decisioni
- **Interfaccia API prima della UI**: i componenti dipendono solo da `VaultApi` / `AppApi`. In Milestone 3 arriva
  `TauriVault` (comandi IPC con gli stessi nomi) e `DemoVault` resta per l'anteprima e i test.
- **Segreti**: anche nella demo la forma dell'API impedisce che la password stia nei dati di lista o dettaglio; serve a
  far rispettare il confine fin da ora.
- **Una sola fonte per comandi e scorciatoie**: menu nativo, handler, pannello e palette non possono divergere. Il
  test Rust e quello TS controllano lo stesso JSON.
- **Copia password tramite l'evento `copy`**:
  - Su macOS gli acceleratori del menu precedono `keydown`, quindi `Cmd+C` arriva come comando Copy, che genera
    l'evento DOM `copy`.
  - Su Windows e Linux `Ctrl+C` genera lo stesso evento.
  - Così un unico punto d'aggancio funziona ovunque e non ruba la copia del testo selezionato negli input.
- **Niente routing**: la schermata è stato applicativo; evita i problemi di path nell'anteprima web e lo stato
  "bloccato" non dipende da un URL.
- **Componenti shadcn-svelte** aggiunti con la CLI e adattati ai token; i componenti applicativi stanno fuori da
  `components/ui/`.
- **Anteprima web**: `pnpm build` produce `ui/build/`, che viene pubblicato come pagina privata su claude.ai (aggiornata
  allo stesso link). È la verifica visiva per l'utente; non sostituisce la build Tauri in CI.

## Non-goals
- Lettura e scrittura reale di file KDBX, IPC del vault, appunti reali, timer di auto-lock e listener del blocco schermo
  (Milestone 3).
- Tema scuro attivabile, traduzioni non inglesi, campi personalizzati / allegati / cronologia nell'interfaccia,
  favicon, drag & drop, dialoghi file nativi (`tauri-plugin-dialog` arriva con l'apertura reale dei file, M3).
- Firma del codice, release (Milestone 4).

## Rischi
- **Comportamento del menu Edit predefinito e dell'evento `copy` su macOS/Windows**: non verificabile in CI con
  interazioni reali.
  - Mitigazione: test jsdom sull'aggancio a `copy`, e verifica manuale rinviata a quando l'utente potrà provare una
    build.
  - Il rischio viene annotato nel log di revisione e in `docs/DEVELOPMENT.md` (checklist).
- **Crate app non compilabile in locale**: gli errori di `menu.rs` emergono solo in CI.
  - Mitigazione: codice piccolo, dati da JSON, test Rust dedicati; i cicli di correzione passano dalla CI.
- **Spazio disco** (~1,9 GB):
  - nuove dipendenze npm limitate a `@testing-library/svelte`, `@testing-library/user-event` e `jsdom`, più i
    componenti shadcn, che sono sorgenti;
  - nessun `target/` locale per il crate app;
  - `df` controllato prima e dopo `pnpm install`.
- **Anteprima web**: lo script di avvio inline di SvelteKit e i path relativi potrebbero comportarsi diversamente
  nella pagina claude.ai. Se non funziona, la verifica visiva ripiega sugli screenshot generati in CI (fuori scope
  salvo necessità).
- **Euristica "untranslated text"**: con molti nuovi componenti può dare falsi positivi. Si possono aggiungere
  eccezioni puntuali, documentate e mai generiche.

## Verifica
Locale (Linux, host):
```
cd crates/app/ui
pnpm install --frozen-lockfile
pnpm check && pnpm lint && pnpm format:check && pnpm test && pnpm build
grep -rnE "localStorage|sessionStorage|indexedDB" src --include=*.ts --include=*.svelte | grep -v "\.test\.ts"   # nessun risultato
cd ../../.. && cargo fmt --all --check && python3 -c "import json;json.load(open('crates/app/ui/src/lib/commands/commands.json'))"
```
Attesi:
- `svelte-check` senza errori né warning;
- ESLint e Prettier puliti;
- Vitest tutto verde, con i test di flusso del criterio 11;
- build in `ui/build/` senza riferimenti `http(s)://` a risorse (`grep -rnE 'src="https?:|href="https?:[^"]*\.(css|js|woff2?)' build` vuoto).

Remota (CI su push di `milestone-2` e sulla PR):
- run `completed/success` con tutti i job verdi (core ×3, app ×3, deny, fuzz), compresi `cargo clippy --workspace -D
  warnings` e `cargo test -p keyrust-app` con i test del menu e `security_config.rs`;
- sulla PR, artifact `keyrust-{windows,macos,linux}-debug` prodotti.

Visiva (utente):
- l'anteprima web, pubblicata come pagina privata su claude.ai, mostra i flussi del criterio 4 e corrisponde al
  prototipo approvato;
- l'utente conferma, oppure le differenze vengono corrette prima del merge.

## Toolchain per provider
- **Builder** (Claude, host): Node 24, pnpm 12.15, Rust 1.99.1 (solo fmt e core; il crate app non si compila in
  locale); rete verso npm, shadcn-svelte.com (registry dei componenti) e api.github.com; push via SSH (account
  `downlevel`).
- **Reviewer/Inspector** (Codex, sandbox read-only): legge file e diff con comandi di sola lettura; non esegue i comandi
  di prova, build o test (li esegue l'host). Non ha accesso di rete a GitHub: lo stato della CI gli viene fornito
  dall'host.
