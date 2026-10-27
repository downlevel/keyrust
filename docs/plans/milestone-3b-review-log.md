# Log di revisione — Milestone 3b (IPC, UI reale, appunti, dialoghi)

- Ruoli: host/planner/builder **Claude** (Claude Code, Opus 5.5), reviewer e inspector **Codex** (CLI, modello di default).
- Piano: `docs/plans/milestone-3b.md`. Base `3fbafab`, branch `milestone-3b`.
- Autorizzazione: "proseguiamo?" dopo il merge della 3a, nel flusso a fasi deciso dall'utente (piano + implementazione); push solo sul branch.
- Limiti: MAX_ROUNDS=5, MAX_FIX_ROUNDS=2, MAX_INSPECTION_ROUNDS=2, inspect=on.

## Round 1 — Codex codex-cli 0.164.0 (sessione 01a17f6f-1852-7af1-82dc-be77e0a28055) — REVISE — plan sha256 691e5f27…ace
- Result: `/tmp/claude-0/-opt-apps-password-manager/8dafc736-041e-41d6-8225-8c7780d29f3a/scratchpad/claudex/m3br1/claudex-jv8llyeb/result.json`
- **R1 (high)** la guardia guardava solo l'editor: un salvataggio fallito seguito da Quit/Close perdeva le modifiche →
  ACCETTATO: `vaultDirty`/`saving` indipendenti dall'editor e guardia unificata (Retry save / Discard / Cancel).
- **R2 (medium)** `update_settings` con un'istantanea vecchia cancellava i file recenti → ACCETTATO: `recent_files`
  gestiti solo dal backend.

## Round 2 — stessa sessione — **APPROVED**, 0 findings — plan sha256 28fae930…5b0
- Result: `/tmp/claude-0/-opt-apps-password-manager/8dafc736-041e-41d6-8225-8c7780d29f3a/scratchpad/claudex/m3br1/claudex-598ntqu_/result.json`

## Build (host Claude)
- c20f2c1 (lato Rust): CI run 40594205057 **success** su tutti gli 8 job al primo tentativo (clippy e test del crate app
  sui 3 OS). 3711836 (frontend): CI run 40595922925 **success**.
- `cargo deny`: BSL-1.0 aggiunta all'allowlist (clipboard-win, via arboard); RUSTSEC-2026-0370/0371/0372 ignorati
  (sotto-crate di zbus "non mantenuti come crate separati", informativi, via tauri-plugin-dialog).

## Inspection 1 — Codex codex-cli 0.164.0 (sessione nuova 01a17f80-e3b5-7ab1-ab51-c31f25d3b228) — REVISE, 1 high + 2 medium
- Result: `/tmp/claude-0/-opt-apps-password-manager/8dafc736-041e-41d6-8225-8c7780d29f3a/scratchpad/claudex/m3bi1/claudex-5s7ks0qd/result.json`. Tutti **ACCETTATI**:
  - **M3B-01 (high)** tra una modifica (gruppo, ripristino, eliminazione, impostazioni) e l'inizio del suo salvataggio
    Quit/Close non vedevano nulla; un refresh fallito saltava il salvataggio → ogni modifica è tracciata dal primo await
    (`track`/`change`), `vaultDirty` vale dall'inizio, uscire attende tutto il lavoro in corso e chiede lo stato
    `dirty` al backend (`database_info`); un refresh fallito non salta il salvataggio.
  - **M3B-02** un errore temporaneo degli appunti perdeva l'hash e lasciava la password → `get` distingue "vuoto" da
    "illeggibile", tentativi ripetuti (timer 5×500 ms, lock/uscita 3×50 ms) e hash conservato finché non si riesce.
    Verificato in locale compilando il modulo in un crate temporaneo (8 test verdi), poi in CI.
  - **M3B-03** messaggio sbagliato dopo un cambio password non salvato → esiti espliciti (`changed`, `notDurable`,
    `pending` con "Discard change" che ricarica il file, `rejected`).
- I 4 test nuovi del frontend falliscono sul codice di 3711836 e passano dopo le correzioni (152/152).
- CI su 7d75d07 (run 40597683847): clippy del workspace fallito sui 3 OS per `AtomicU32::fetch_update` deprecato nei test
  di `clipboard.rs` (rinominato `try_update`). Il crate temporaneo usato per la verifica locale non applicava i lint del
  progetto: da qui in poi la verifica locale del modulo gira con gli stessi lint e `clippy -D warnings`.

## Inspection 2 — Codex codex-cli 0.164.0 (sessione nuova 01a17f8d-e713-7a51-884e-b96373c575e3) — REVISE, 1 high + 2 medium
- Result: `/tmp/claude-0/-opt-apps-password-manager/8dafc736-041e-41d6-8225-8c7780d29f3a/scratchpad/claudex/m3bi2/claudex-wzhq_110/result.json`. Tutti **ACCETTATI** (secondo e ultimo round di correzioni):
  - **M3B-R01 (high)** la guardia e "Retry save" eseguivano l'azione rimandata anche dopo un Lock → ogni continuazione
    cattura la sessione e si interrompe dopo ogni attesa se è cambiata. Il primo test scritto passava anche sul codice
    vecchio (lì il salvataggio rifiutava già la sessione obsoleta); sostituito da un test sull'attesa nella guardia, che
    fallisce reintroducendo solo il buco e passa con la correzione.
  - **M3B-R02** le voci di un gruppo eliminato (spostato intero nel cestino dal core) risultavano attive → insieme dei
    gruppi nel cestino derivato dall'albero, usato per conteggi, filtri, modifica, conferma definitiva, gruppi
    dell'editor, sidebar e palette; la demo ora elimina i gruppi come il core.
  - **M3B-R03** le password digitate nel dialog restavano nel componente dopo il Lock → azzerate appena il dialog non è più
    aperto (qualunque sia il motivo); un completamento dopo il Lock non riapre le impostazioni.
- Frontend 156/156; clipboard 8/8 con i lint del progetto.
- **Budget di ispezione esaurito** (MAX_INSPECTION_ROUNDS=2): queste correzioni non sono state ispezionate da Codex.
