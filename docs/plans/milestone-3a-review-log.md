# Log di revisione — Milestone 3a (core vault)

- Ruoli: host/planner/builder **Claude** (Claude Code, Opus 5.5), reviewer e inspector **Codex** (CLI, modello di default).
- Piano: `docs/plans/milestone-3a.md`. Base `342bb7f`, branch `milestone-3`.
- Autorizzazione: "procedi con la Milestone 3" (piano + implementazione, come per le milestone precedenti); push solo sul branch.
- Decisioni utente: tre fasi (3a core, 3b IPC/UI/appunti, 3c auto-lock/fuzzing); modifiche esterne rilevate e segnalate (niente merge); robustezza con zxcvbn.
- Limiti: MAX_ROUNDS=5, MAX_FIX_ROUNDS=2, MAX_INSPECTION_ROUNDS=2, inspect=on.

## Round 1 — Codex codex-cli 0.164.0 (sessione 01a17f44-0dcb-7003-9dce-55ada876b38e) — REVISE — plan sha256 6babb5c1…a3f
- Result: `/tmp/claude-0/-opt-apps-password-manager/8dafc736-041e-41d6-8225-8c7780d29f3a/scratchpad/claudex/m3ar1/claudex-axrs4lpl/result.json`
- **R1 (high)** race tra controllo dello stampo e rename; stampo non legato ai byte analizzati → ACCETTATO: stampo
  (lunghezza + SHA-256) degli stessi byte, lock advisory tra istanze, ricontrollo subito prima del rename, test di
  interleaving deterministici, rischio residuo documentato.
- **R2 (medium)** Reload dopo un cambio password non salvato → ACCETTATO: credenziali confermate e in attesa.
- **R3 (medium)** il modello KDF non descrive AES-KDF/Argon2d → ACCETTATO: `KdfInfo` dall'header, `Argon2idParams`
  modificabili separati, nessuna conversione implicita.
- **R4 (medium)** eliminazioni definitive senza tombstone → ACCETTATO: `DeletedObjects` per voci, gruppi e discendenti.

## Round 2 — stessa sessione — **APPROVED**, 0 findings — plan sha256 ade814a9…5fc
- Result: `/tmp/claude-0/-opt-apps-password-manager/8dafc736-041e-41d6-8225-8c7780d29f3a/scratchpad/claudex/m3ar1/claudex-6rfpmgee/result.json`
