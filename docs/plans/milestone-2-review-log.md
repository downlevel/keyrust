# Log di revisione — Milestone 2

- Ruoli: host/planner/builder **Claude** (Claude Code, Opus 5.5), reviewer del piano e inspector **Codex** (CLI, modello di default).
- Piano: `docs/plans/milestone-2.md`. Base `f5ba055`, branch `milestone-2`.
- Autorizzazione: piano + implementazione richiesti dall'utente ("procedi con il piano della Milestone 2", workflow claudex-loop delle milestone precedenti); push solo sul branch.
- Decisioni utente: modalità demo con avviso visibile; command palette di base inclusa; direzione visiva del prototipo https://claude.ai/artifact/7NiAtLZoAk2V5uZmCcyydh approvata.
- Limiti: MAX_ROUNDS=5, MAX_FIX_ROUNDS=2, MAX_INSPECTION_ROUNDS=2, inspect=on.

## Round 1 — Codex codex-cli 0.164.0 (sessione 01a17ec5-6cdb-70c1-bc63-7c821253f1d2) — REVISE — plan sha256 f86510f2…d900
- Result: `/tmp/claude-0/-opt-apps-password-manager/8dafc736-041e-41d6-8225-8c7780d29f3a/scratchpad/claudex/m2r1/claudex-d1okiqa7/result.json`
- **M2-001 (medium) — risposte asincrone tardive possono ripristinare segreti dopo lock/cambio voce** → ACCETTATO:
  generazioni di sessione e di selezione verificate prima di applicare ogni risultato asincrono; test con promesse differite.
- **M2-002 (medium) — bozza modificata persa da percorsi diversi da Cancel/Esc** → ACCETTATO: guardia unica
  `leaveEditor` (Save / Discard / Keep editing) per selezione, New Entry, palette, menu nativo; Lock resta immediato.

## Round 2 — stessa sessione — **APPROVED**, 0 findings — plan sha256 b847a662…2001
- Result: `/tmp/claude-0/-opt-apps-password-manager/8dafc736-041e-41d6-8225-8c7780d29f3a/scratchpad/claudex/m2r1/claudex-xwvsz758/result.json`
- Limiti dichiarati: approvazione del piano, non dell'implementazione; prototipo esterno, anteprima web, CI e comportamento
  nativo dei menu non verificati dal reviewer.

## Build (host Claude)
- Commit 81d71d6 (implementazione) — CI run 40580583112 **success** su tutti gli 8 job al primo tentativo
  (incluso `cargo clippy --workspace` e `cargo test -p keyrust-app` con i test di `menu.rs` e `security_config.rs`).
- Verifiche locali: svelte-check 0 errori/0 warning, ESLint e Prettier puliti, Vitest 74/74 (flussi UI, stato con
  promesse differite, guardia dell'editor, registro comandi, controlli statici di sicurezza), build senza risorse remote.
- Anteprima web (build di produzione su dati demo): https://claude.ai/artifact/Ag3YbVLHrPxb9whfu2K7wN. Solo nella
  copia pubblicata `_app` è rinominata `app` (nomi con `_` riservati dal servizio), gli import iniziali sono resi
  relativi e un carattere U+FFFD letterale di un polyfill `CSS.escape` è scritto come `�`.
- Deviazioni dal piano:
  - router SvelteKit in modalità hash (rimosso `+layout.ts` con `ssr = false`, che in modalità hash è implicito):
    serve all'anteprima web e non cambia nulla in Tauri (un solo route);
  - acceleratori nativi aggiunti oltre a quelli elencati: `CmdOrCtrl+Q` (Quit) e `CmdOrCtrl+Comma` (Settings), che
    non interferiscono con la modifica del testo (test dedicati in TS e Rust);
  - nuovo comando IPC `exit_app` per Quit dopo la guardia delle modifiche; `COMMANDS` e `security_config.rs` aggiornati;
  - componenti shadcn aggiunti e poi rimossi perché non usati (badge, checkbox, collapsible, kbd, label, scroll-area,
    select, separator, slider, tabs, tooltip): i controlli nativi (`input type=range`, `checkbox`, `select`) bastano
    e sono più semplici da testare;
  - il link URL della voce in Tauri non apre ancora il browser di sistema (serve `tauri-plugin-opener`, Milestone 3);
    nella demo il clic è ignorato.

## Inspection 1 — Codex codex-cli 0.164.0 (sessione nuova 01a17f21-8bb4-7393-a722-bc0a20f0f352) — REVISE, 8 finding medium
- Result: `/tmp/claude-0/-opt-apps-password-manager/8dafc736-041e-41d6-8225-8c7780d29f3a/scratchpad/claudex/m2i1/claudex-9hplg87j/result.json` (base f5ba055, snapshot di 81d71d6 + c043757).
- Tutti **ACCETTATI**, ognuno con un test di regressione che fallisce sul codice precedente e passa con la correzione
  (verificato eseguendo i nuovi test contro `app.svelte.ts`/`demo.ts` di c043757: 8 falliti, 74 passati):
  - **M2-I01** Save fallito eseguiva comunque la transizione → `persist()` restituisce l'esito, `saveEditor` lo propaga.
  - **M2-I02** Lock durante Save eseguiva la transizione vecchia → `resolveLeave` riverifica la sessione dopo il
    salvataggio; `saveEditor` non ricarica la voce se nel frattempo l'utente ha cambiato selezione.
  - **M2-I03** un unlock/create obsoleto poteva sostituire o bloccare una sessione più recente → le operazioni che
    cambiano sessione (unlock, create, lock) sono serializzate; un completamento obsoleto viene richiuso prima che
    parta la successiva.
  - **M2-I04** la password generata sopravviveva al Lock → azzerata al Lock, alla chiusura dell'editor e prima di ogni
    rigenerazione ("Use password" resta disabilitato finché non arriva la risposta).
  - **M2-I05** risposte del generatore fuori ordine → contatore di richiesta più generazioni di sessione/selezione.
  - **M2-I06** la lunghezza predefinita nelle impostazioni non arrivava al generatore → sincronizzata dopo l'aggiornamento.
  - **M2-I07** "Use the Recycle Bin" disattivato non aveva effetto → eliminazione definitiva e testo di conferma relativo.
  - **M2-I08** Delete / Ctrl+B / Ctrl+U agivano con testo selezionato → stessa guardia di `handleCopy` per tutti i
    comandi `outsideText`.
- CI su c043757 (hash router): run 40581659418 success su tutti i job.

## Inspection 2 — Codex codex-cli 0.164.0 (sessione nuova 01a17f28-a25f-7850-930b-277481fd8eae) — REVISE, 4 medium + 1 low
- Result: `/tmp/claude-0/-opt-apps-password-manager/8dafc736-041e-41d6-8225-8c7780d29f3a/scratchpad/claudex/m2i2/claudex-ktb9r0rh/result.json` (snapshot 5ffa728). CI su 5ffa728: run 40582782254 success.
- Tutti **ACCETTATI** (secondo e ultimo round di correzioni); i test nuovi falliscono sul codice di 5ffa728 (4 falliti) e
  passano dopo le correzioni (135/135):
  - **M2-R201 / M2-R202** salvataggi dell'editor concorrenti, modifiche e navigazione durante un salvataggio →
    un solo salvataggio alla volta (`savingEntry`): form disabilitato, `updateDraft` e `requestLeave` ignorati
    finché il salvataggio non termina (Lock resta immediato).
  - **M2-R203** scritture delle impostazioni fuori ordine → coda seriale; ogni scrittura invia lo stato desiderato più
    recente e solo l'ultima risposta viene applicata.
  - **M2-R204** dopo lo sblocco la lista non era raggiungibile con Tab → la prima voce è il tab stop quando nessuna voce
    visibile è selezionata.
  - **M2-R205 (low)** etichette "Close" e default inglesi nei componenti vendorizzati → spostate in `en.json`; rimosso
    `command-dialog.svelte` (non usato); il controllo "untranslated text" ora copre anche `components/ui` e i
    default delle prop.
- CI su d773220: run 40584064526 success su tutti gli 8 job.
- **Budget di ispezione esaurito** (MAX_INSPECTION_ROUNDS=2): le correzioni di questo round non sono state ispezionate
  da Codex. Lo si segnala all'utente, che decide se chiedere un'ulteriore ispezione prima del merge.
