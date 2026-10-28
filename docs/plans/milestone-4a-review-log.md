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
