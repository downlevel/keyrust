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
