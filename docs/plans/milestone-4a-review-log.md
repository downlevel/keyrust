# Log di revisione — Milestone 4a (accessibilità, e2e smoke, documentazione definitiva)

- Ruoli: host/planner/builder **Claude** (Claude Code, Opus 5.5), reviewer e inspector **Codex** (CLI, modello di default).
- Piano: `docs/plans/milestone-4a.md`. Base `a747fef`, branch `milestone-4a`.
- Autorizzazione: preparare il piano con claudex-loop; implementazione solo su conferma dell'utente; push solo sul branch.
- Decisioni utente (2026-10-28): e2e con file recente pre-caricato (nessun hook); audit axe + Playwright sulla demo;
  screenshot Playwright sulla demo; KDBX 3.1/4.0 con avviso non bloccante.
- Limiti: MAX_ROUNDS=5, MAX_FIX_ROUNDS=2, MAX_INSPECTION_ROUNDS=2, inspect=on.

## Round 1 — Codex codex-cli 0.164.0 (sessione 01a18315-f210-7f43-bbb7-805b403ae655) — REVISE (1 high, 2 medium) — plan sha256 46b42158…a40
- Result: `/tmp/claude-0/-opt-apps-password-manager/ed4fe123-bb03-4dee-b36b-94caf3d40ad8/scratchpad/claudex/r1/claudex-8aypp4_e/result.json`
- M4A-01 (medium) `Vault::write` porta la versione a 4.1 prima della sostituzione e non la ripristina su errore →
  ACCETTATO (verificato): correzione nel core, versione 4.1 solo a file sostituito; test di salvataggi falliti/riusciti.
- M4A-02 (high) il README avrebbe detto "segreti mai nel webview", falso (modifica, reveal, generatore) → ACCETTATO:
  confine descritto con esattezza in README e SECURITY.md.
- M4A-03 (medium) controllo del focus aggirabile (outline 0, trasparente, ombra immutata) → ACCETTATO: indicatore reso,
  opaco, che cambia col focus, contrasto calcolato sul colore effettivo.

## Round 2 — stessa sessione — REVISE (1 medium) — plan sha256 09216da4…081
- Result: `/tmp/claude-0/-opt-apps-password-manager/ed4fe123-bb03-4dee-b36b-94caf3d40ad8/scratchpad/claudex/r2/claudex-7q1lhhpp/result.json`. M4A-01/02/03 risolti secondo il reviewer.
- M4A-04 (medium) `innerText` non vede nodi nascosti, attributi e valori degli input → ACCETTATO: scansione di
  textContent, attributi e `.value` su tutto il documento, in lista e con il dettaglio aperto.

## Round 3 — stessa sessione — **APPROVED**, 0 findings — plan sha256 0ff713d9…67b
- Result: `/tmp/claude-0/-opt-apps-password-manager/ed4fe123-bb03-4dee-b36b-94caf3d40ad8/scratchpad/claudex/r3/claudex-u5ikznlh/result.json`. Limiti dichiarati: approvazione del piano, non dell'implementazione; revisione statica; browser,
  accessibilità desktop e CI non verificati.

## Build — Claude (builder), base `6229bfe`
- Autorizzazione: l'utente ha confermato l'implementazione il 2026-10-29. Approvazione del piano verificata
  (`runner.py check`): sha256 `0ff713d9…67b` invariato.
- Commit: `2fc628e` (core), `6a5e1a4` (UI, audit), `aad5931` (e2e, screenshot, CI), `3674b65` (documenti),
  poi le correzioni emerse in CI (sotto), fino a `ed7ebc2`.
  Autore di tutto il codice: Claude. Codex non ha scritto codice.

### Baseline dell'audit (prima delle correzioni, `pnpm test:web a11y`)
Prima esecuzione: 7 test su 14 falliti, nessuna violazione moderate/minor; gli stati dopo il primo errore
di ciascun test non erano raggiunti.
- `color-contrast` (serious): `muted-foreground` (zinc-500) sulla riga selezionata (`selection`, 4,32:1) —
  vault-entry, editor-edit, confirm; bottone primario in hover `primary/80` (4,39:1) — unlock-wrong-password.
- `aria-required-attr` (critical): combobox del palette senza `aria-controls`.
- `scrollable-region-focusable` (serious): lista del palette.
- `aria-required-children` (critical): `listbox` vuoto (gruppo senza voci).
- Test dei token (Vitest): 9 coppie sotto soglia (muted-foreground su muted/accent/selection; bordi degli
  input 1,5:1; strength-2/3 sotto 3:1).
- Test del focus: anelli al 20–50 % di alfa su input, bottoni, switch; palette senza anello (`shadow-none!`
  e `has-[data-slot=input-group-control]` mai vero). Tastiera: Home/End assenti nella lista.

### Deviazioni dal piano (motivate)
- Ordine di tab: il DOM segue la lettura sidebar → ricerca → lista → dettaglio (come DESIGN.md). Il test
  parte dalla ricerca e verifica ricerca → lista → dettaglio → (giro) albero, una tappa ciascuno, e
  Shift+Tab esattamente inverso.
- Create non ha passi: un solo modulo; coperti stato iniziale, avanzate aperte, conferma diversa.
- Banner di versione non raggiungibile nella demo (sempre 4.1): coperto da `version-notice.test.ts`.
- Il campo di ricerca usa la classe `border-input` invece di `border-border` (è un campo, non un divisore):
  cambio di classe, non di token.
- Focus: il test attende la fine delle transizioni (`document.getAnimations()`) invece di emulare reduced
  motion, che lascia comunque transizioni di 0,01 ms.
- Screenshot: la password generata è mascherata (`mask`), per la regola "nessuna password in chiaro".
- e2e: WebdriverIO 9.32 (la 10 ha tre settimane, la guida Tauri usa la 9), tauri-driver 2.1.0 (la 3 è
  alpha); salvataggio tramite il pulsante Save dell'editor, che scrive il file.
- In più rispetto al piano: Home/End nella lista, ruolo `dialog` del popover del generatore, verifica in CI
  che la copia KDBX 3.1 resti identica (nessun salvataggio).

### Verifica locale
- `cargo test -p keyrust-core` verde (68 + 11 + 5 + 3), clippy `-D warnings` e fmt puliti.
- UI: `pnpm check` (incluso `tsc -p e2e-web`), `lint`, `format:check`, `pnpm test` 246 test verdi.
- `pnpm test:web`: 49 test verdi, 0 violazioni axe di qualsiasi impatto.
- `pnpm screenshots`: 4 PNG da 89–219 KB a scala 2, controllati a occhio.
- e2e: `pnpm --dir crates/app/e2e check` verde; preparazione dati eseguita in locale (copie identiche,
  `settings.json` nel percorso XDG atteso). Lo scenario gira solo in CI.
- Link relativi di README, SECURITY e documenti: nessuno rotto.
- `keyrust-app` (Rust) non modificato: clippy Windows con il finto `llvm-rc` non necessario.

### CI e difetto trovato dall'e2e
- Audit a11y in CI: il pulsante *Benchmark* di Create ritorna attivo con `transition-all` alla fine del
  benchmark automatico; axe lo misurava a metà dissolvenza. L'audit ora attende le animazioni
  (`Promise.allSettled`, perché quelle annullate rifiutano `finished`).
- E2E: senza bus di sessione GTK aspettava ~25 s il portale → `dbus-run-session`. Log non leggibili senza
  login: in caso di errore la coda filtrata del log diventa un'annotazione pubblica, con lo stato della
  pagina e del bootstrap di SvelteKit.
- **Difetto dell'app (non del test)**: su Linux (e quasi certamente macOS) la UI è servita da
  `tauri://localhost`, URL non speciale con percorso vuoto e origin opaco; il router di SvelteKit non trovava
  la rotta e ricaricava la pagina all'infinito: finestra vuota. Windows (`http://tauri.localhost/`) non era
  colpito. Correzione `4ed52f7`: finestra caricata a `"url": "/"`, con test di configurazione
  (`the_window_loads_a_url_with_a_path`). macOS da verificare a mano.
- WebKitWebDriver restituisce testo vuoto per gli elementi `truncate`: lo smoke test legge `textContent`.
- Run finale `ed7ebc2`: tutti i job verdi — core ×3, app ×3 (audit a11y su Linux), deny, fuzz, e2e (scenario
  completo e `keepassxc-cli show` sulla copia salvata con il titolo modificato; copia KDBX 3.1 identica).
