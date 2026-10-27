# Log di revisione — Milestone 3c (auto-lock, limiti KDF, fuzzing)

- Ruoli: host/planner/builder **Claude** (Claude Code, Opus 5.5), reviewer e inspector **Codex** (CLI, modello di default).
- Piano: `docs/plans/milestone-3c.md`. Base `1f07b58`, branch `milestone-3c`.
- Autorizzazione: "procedi con la 3c" (piano + implementazione); push solo sul branch.
- Decisioni utente: mini-crate `crates/os` con unsafe isolato per il blocco schermo su Windows/macOS; limiti generosi in apertura (Argon2 2 GiB / 1000 iterazioni / 256 lane, AES-KDF 2·10⁹ round, file 512 MiB).
- Limiti: MAX_ROUNDS=5, MAX_FIX_ROUNDS=2, MAX_INSPECTION_ROUNDS=2, inspect=on.

## Round 1 — Codex codex-cli 0.164.0 (sessione 01a17fba-60b9-7200-8fc7-7787024d3181) — REVISE (4 high, 2 medium) — plan sha256 f64524cd…917
- Result: `/tmp/claude-0/-opt-apps-password-manager/8dafc736-041e-41d6-8225-8c7780d29f3a/scratchpad/claudex/m3cr1/claudex-yk7s92zu/result.json`
- M3C-01 (high) fuzzing fermo all'HMAC → ACCETTATO in parte: target strutturato `vault_roundtrip`; XML ostile oltre l'HMAC
  non raggiungibile (nessun parser pubblico in keepass-rs, serve la chiave) → limite documentato e proposto upstream.
- M3C-02 semi Argon2 impossibili con `Vault::create` → ACCETTATO: semi generati direttamente con keepass-rs e verificati.
- M3C-03 (high) limiti in apertura incoerenti con `kdf::validate` → ACCETTATO: stessi tetti in validazione e frontend.
- M3C-04 (high) letture non limitate (anche nei controlli di modifica esterna) → ACCETTATO: lettura limitata da una handle,
  confronto della lunghezza e hash in streaming.
- M3C-05 (high) uno sblocco in corso poteva reinstallare il vault dopo un auto-lock → ACCETTATO: generazione di sessione nel backend.
- M3C-06 (high) decompressione illimitata in keepass-rs → **RESPINTO con motivazione**: avviene solo dopo l'HMAC con la chiave
  dell'utente (serve la master password); limite documentato in SECURITY.md e proposto upstream.

## Round 2 — stessa sessione — REVISE — M3C-07 (medium): sospensione non gestita su Windows/macOS → ACCETTATO (rilevatore di salti).
## Round 3 — stessa sessione — REVISE — M3C-07: soglia di 30 s troppo alta → ACCETTATO: Windows con notifica nativa
   (`PowerRegisterSuspendResumeNotification`), macOS con thread a 1 s e soglia 4 s; rischio residuo macOS (< 4 s) documentato.
## Round 4 — stessa sessione — **APPROVED**, 0 findings — plan sha256 79b207c3…b60
- Result: `/tmp/claude-0/-opt-apps-password-manager/8dafc736-041e-41d6-8225-8c7780d29f3a/scratchpad/claudex/m3cr1/claudex-fbtpjmrt/result.json`. Limiti dichiarati dal reviewer: rilevamento della sospensione su
  macOS euristico; decompressione e XML ostile restano limiti documentati.
