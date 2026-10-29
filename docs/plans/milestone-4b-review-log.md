# Log di revisione — Milestone 4b (pipeline di release, firma, build riproducibili, v0.1.0)

- Ruoli: host/planner/builder **Claude** (Claude Code, Opus 5.5), reviewer e inspector **Codex** (CLI 0.165.0, modello di default, `--effort high`).
- Piano: `docs/plans/milestone-4b.md`. Base `0325d9a`, branch `milestone-4b`.
- Autorizzazione: preparare il piano con claudex-loop; implementazione solo su conferma dell'utente; push solo sul branch; tag e release all'utente.
- Decisioni utente (2026-10-29): firma codice OS opzionale (attiva con i secret); cosign keyless; pacchetti NSIS+MSI, deb+AppImage, dmg universale; dry run senza release, tag `v0.1.0` creato dall'utente.
- Limiti: MAX_ROUNDS=5, MAX_FIX_ROUNDS=2, MAX_INSPECTION_ROUNDS=2, inspect=on.

## Round 1 — Codex codex-cli 0.165.0 (gpt-6.1-sol, effort high; sessione 01a18806-07e0-7272-894f-206767415ecd) — REVISE (1 high, 5 medium, 1 low) — plan sha256 73344e41…dfb
- Result: `/tmp/claude-0/-opt-apps-password-manager/396c1774-6943-4819-a44e-4e13330ae930/scratchpad/claudex/r1/claudex-vn6xsixz/result.json`
- M4B-R01 (high) gli installer sono 5, non 6 (asset release 7, non 8) → ACCETTATO.
- M4B-R02 (medium) SvelteKit scrive `Date.now()` in `_app/version.json`, incorporato nel binario → ACCETTATO (verificato): `kit.version.name` da `SOURCE_DATE_EPOCH`.
- M4B-R03 (medium) i permessi valgono per job: `contents: write` e OIDC nello stesso job anche in dry run → ACCETTATO: job `sign` e `release` separati.
- M4B-R04 (medium) PR da fork/Dependabot senza OIDC → ACCETTATO: lì niente firma, dichiarato nel summary.
- M4B-R05 (medium) l'esito della riproducibilità non arrivava alle note → ACCETTATO: `release` attende `reproducibility` senza esserne bloccato.
- M4B-R06 (medium) `sha256sum` assente su macOS e verifica di un solo installer → ACCETTATO.
- M4B-R07 (low) `fuzz/Cargo.lock` registra `keyrust-core` 0.0.1 → ACCETTATO.

## Round 2 — stessa sessione — REVISE (1 medium) — plan sha256 a85a2750…087
- Result: `/tmp/claude-0/-opt-apps-password-manager/396c1774-6943-4819-a44e-4e13330ae930/scratchpad/claudex/r2/claudex-qlrhaay8/result.json`. M4B-R01…R07 risolti secondo il reviewer.
- M4B-R08 (medium) un `workflow_dispatch` su un tag avrebbe avviato il job `release` → ACCETTATO: un solo predicato `release` (push di tag) calcolato in `verify` e usato ovunque.

## Round 3 — stessa sessione — **APPROVED**, 0 findings — plan sha256 3147bcd6…071
- Result: `/tmp/claude-0/-opt-apps-password-manager/396c1774-6943-4819-a44e-4e13330ae930/scratchpad/claudex/r3/claudex-sem_dyuc/result.json`. Limiti dichiarati: approvazione del piano, non dell'implementazione; revisione statica; impostazioni GitHub, credenziali di firma e installer non verificati.

## Build — Claude (builder), base `612bc2d`
- Autorizzazione: l'utente ha confermato l'implementazione il 2026-10-29, precisando che la strada senza firma codice
  (nessuna spesa) dev'essere pienamente funzionante. Approvazione del piano verificata (`runner.py check`).
- Prove locali: actionlint 1.7.12 (con shellcheck) su `release.yml` e `ci.yml` → 0 errori; shellcheck su
  `scripts/release/*.sh` → 0; `check-version.sh` (senza tag, `v0.1.0` → 0; `v0.1.1`, `v0.2.0`, `0.1.0` → 1);
  `changelog-section.sh` (0.1.0 → sezione senza link finali; 9.9.9, Unreleased → 1); `check-assets.sh` (5 file → 0;
  mancante/extra → 1 con elenco); override Tauri (ad hoc macOS e `signCommand` Windows generato) validi contro
  `config.schema.json` di `@tauri-apps/cli` 2.12.4 dopo il merge con `tauri.conf.json`; due `pnpm build` con lo stesso
  `SOURCE_DATE_EPOCH` → `diff -r` identico, senza → diversi; `pnpm lint/format:check/check/test` (250 test) verdi;
  `cargo metadata --locked --offline` nei due workspace; `cargo test -p keyrust-core` verde.
- Deviazioni dal piano: l'override Windows non è un file committato ma è generato nello step (endpoint, account e
  profilo vengono da variabili del repository); la riproducibilità usa due job `rebuild` paralleli (due runner, due
  cartelle) più il job `reproducibility` che confronta; `fuzz/Cargo.lock` era già disallineato da 4a (mancava
  `roxmltree`), riallineato alla stessa versione del lockfile principale; la verifica di partial secrets fa fallire
  `verify` invece di produrre installer non firmati in silenzio.
