# Review log — Milestone 1

- Data: 2026-10-27
- Ruoli: host/planner/builder = Claude Code (Opus 5.5); plan reviewer + final inspector = Codex CLI 0.164.0 (modello: default CLI).
- Piano: docs/plans/milestone-1.md — branch `milestone-1`, base `main` @ 9c46045.
- Scope: Milestone 1 di docs/TASKS.md (CI 3 OS, deny, fuzz, tracing, i18n Paraglide, settings, fixture KDBX 3.1, header peek).
- Autorizzazione: implementazione autorizzata ("procedi con la Milestone 1"); push solo su branch `milestone-1`; PR/merge su main a cura dell'utente.
- Decisioni utente: Paraglide JS 2; artifact CI non firmati per i 3 OS.
- Limiti: MAX_ROUNDS=5, MAX_FIX_ROUNDS=2, MAX_INSPECTION_ROUNDS=2, inspect=on.

## Round 1 — BLOCKED (operativo), non conta come approvazione
- Result: `/tmp/claude-0/-opt-apps-password-manager/8dafc736-041e-41d6-8225-8c7780d29f3a/scratchpad/claudex/m1r1/claudex-1f8_grnv/result.json` — Codex non ha usato la shell read-only (solo 2 chiamate MCP a risorse vuote) e ha dichiarato di non poter leggere i file. Stessi flag/sandbox dei run riusciti (che usavano command_execution): variazione del modello. Azione: un solo nuovo tentativo in sessione nuova.
- Ritentativo: `/tmp/claude-0/-opt-apps-password-manager/8dafc736-041e-41d6-8225-8c7780d29f3a/scratchpad/claudex/m1r1b/claudex-r64tt707/result.json` — di nuovo BLOCKED. Causa trovata dall'host: il piano diceva "Reviewer/Inspector ... non esegue comandi", che Codex ha applicato anche alla lettura dei file (0 command_execution vs 2–20 nei run precedenti). Corretto il testo ("non esegue i comandi di prova, build o test"). Nuova review in sessione nuova.

## Round 1 (valido) — Codex codex-cli 0.164.0 (effective_model=gpt-6.1-sol, sessione 01a17dcd-ad8e-71f3-be41-6dbcfab222cf) — plan sha256 47885fde6dbabb92bc438c5d480f60b884fa0194c49d3139c8676ad1d28a5841
- Result: `/tmp/claude-0/-opt-apps-password-manager/8dafc736-041e-41d6-8225-8c7780d29f3a/scratchpad/claudex/m1r1c/claudex-6rz1ba1g/result.json`

```json
{
 "verdict": "APPROVED",
 "summary": "The plan is consistent with the inspected baseline and stated milestone scope. No concrete material defects identified. This approves the plan, not an implementation or milestone completion.",
 "findings": [],
 "coverage": [
  "Verified the plan file SHA256 matches the supplied hash.",
  "Reviewed PRD, roadmap, architecture, development guidance, contributing policy, and fixture documentation.",
  "Traced existing Tauri command registration, frontend IPC caller, capability configuration, CSP, and build configuration.",
  "Reviewed Cargo manifests, toolchain configuration, dependency policy, frontend package scripts, and Vite configuration.",
  "Reviewed fixture generation, KeePassXC discovery and subprocess helpers, compatibility tests, and database creation parameters."
 ],
 "limitations": [
  "Milestone 1 implementation is not present: CI workflow, settings module, header parser, fuzz crate, security tests, and Paraglide integration cannot yet be inspected.",
  "No tests, builds, fixture generation, or delegation performed.",
  "Remote CI results, action SHA validity, platform installation paths, and generated artifacts were not verified.",
  "Concurrency, atomic replacement, parser validation, and logging behavior require implementation review before milestone completion can be approved."
 ]
}
```

## Build (host Claude) — note di implementazione e CI
- Implementazione su branch `milestone-1`: a145152 (infrastruttura), 3892e18 (fix CI), 0ad2c3b (fix compatibilità allegati).
- CI run 40526834534: Vitest su Windows falliva per `D:\D:\…` (URL → path) → `fileURLToPath`; i job core Windows/macOS
  fallivano senza log leggibili (API log 403 senza credenziali) → script `.github/scripts/annotate_test_failures.py`
  che espone i test falliti come annotazioni, e ricerca di KeePassXC nei percorsi standard.
- CI run 40529323891 e PC Windows dell'utente: T2 falliva con KeePassXC recente, che esporta `Ref="0"` per ogni
  allegato (verificato dall'utente con l'export della fixture). Il confronto ora identifica gli allegati per
  (UUID voce, versione in cronologia, nome): voci correnti lette con `keepassxc-cli attachment-export`, versioni in
  cronologia con keepass-rs; dove i `Ref` sono indici di pool (KeePassXC 2.7.x) restano un controllo incrociato.
  Mutation check in entrambe le modalità (export reale e `Ref` forzati a 0): alterare `blob.bin` o `readme.txt`
  nel file risalvato fa fallire il diff canonico, anche con il controllo delle capacità disabilitato.
- Deviazione dal piano: per le versioni in cronologia, con KeePassXC recente, il contenuto degli allegati non è più
  verificato da KeePassXC ma da keepass-rs (la CLI non li esporta). Su Linux (2.7.6) la verifica indipendente resta.

## Inspection 1 — Codex codex-cli 0.164.0 (sessione nuova 01a17e44-50ad-75f1-a032-06edeb2fc149) — REVISE
- Result: `/tmp/claude-0/-opt-apps-password-manager/8dafc736-041e-41d6-8225-8c7780d29f3a/scratchpad/claudex/m1i1/claudex-4vji4_od/result.json`
- Base d9c532d, plan sha256 47885fde…5841, 44 file ispezionati.
- **M1-001 (medium) — tmp condiviso tra salvataggi concorrenti** → ACCETTATO. File temporaneo creato con `create_new`
  e nome unico (`<nome>.<pid>-<contatore>.tmp`); rimosso solo il proprio tmp in caso di errore. Test
  `concurrent_saves_never_mix_contents` (8 thread × 20 salvataggi: JSON integro, nessun tmp residuo).
- **M1-002 (medium) — numeri fuori range invalidano tutto il file** → ACCETTATO. Deserializzazione saturante
  (`saturating_u32`: negativi → 0, oltre u32 → MAX, frazioni troncate) poi clamp in `normalized()`. Test
  `negative_and_oversized_numbers_are_clamped_without_losing_other_settings`.
- **M1-003 (medium) — header: terminatore, residui e tipi VariantDictionary non validati** → ACCETTATO con una
  precisazione: il terminatore vuoto è legittimo (keepass-rs scrive `write_with_len(&[])`, KeePassXC `\r\n\r\n`),
  quindi sono accettate esattamente queste due forme. Il dizionario deve esaurire il campo; ogni tipo è validato
  (Bool 0/1, I32 4 byte, I64 8, String UTF-8, U32/U64 a larghezza fissa), tipi sconosciuti rifiutati. Test
  `rejects_malformed_structures`.
- Mutation check: rimuovendo ciascuna correzione (saturazione, tmp unico, controllo residui, validazione tipi,
  terminatore) il test corrispondente fallisce; con le correzioni 15 unit + 11 compat + 2 logging verdi.
