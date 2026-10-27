# Review log — Milestone 0

- Data: 2026-10-27
- Ruoli: host/planner/builder = Claude Code (Opus 5.5); plan reviewer + final inspector = Codex CLI 0.164.0 (modello: default CLI, non specificato).
- Piano: docs/plans/milestone-0.md
- Scope: Milestone 0 di docs/TASKS.md (setup repo, workspace, spike keepass-rs, scaffold Tauri/Svelte, config qualità, SECURITY.md).
- Autorizzazione: implementazione autorizzata dall'utente ("Procediamo con i primi step"); git init + commit locali autorizzati; nessun push/repo remoto.
- Limiti: MAX_ROUNDS=5, MAX_FIX_ROUNDS=2, MAX_INSPECTION_ROUNDS=2, inspect=on.
- Artefatti runner: fuori dal checkout (/tmp/claude-0/-opt-apps-password-manager/8dafc736-041e-41d6-8225-8c7780d29f3a/scratchpad/claudex/).

## Round 1 — Codex codex-cli 0.164.0 (effective_model=gpt-6.1-sol, session 01a17d1f-fe88-7560-ad10-08129b32a48a) — plan sha256 f442699fa49567886f4c61642581eadb1797372cdc63a9a89c2ef846e545ba75
- Result: `/tmp/claude-0/-opt-apps-password-manager/8dafc736-041e-41d6-8225-8c7780d29f3a/scratchpad/claudex/r1/claudex-z33muyag/result.json`

```json
{
 "verdict": "REVISE",
 "summary": "The plan is generally well scoped, but its fixture validation can produce unsupported lossless PASS claims.",
 "findings": [
  {
   "id": "R1",
   "severity": "medium",
   "path": "docs/plans/milestone-0.md",
   "evidence": "Fixture generation imports handwritten XML through KeePassXC, while T2 compares only exports of the resulting database. T1 does not explicitly validate CustomData, AutoType, or custom icons. If import drops one of these features, both T2 exports lack it and the comparison passes without exercising its preservation.",
   "fix": "Validate the imported fixture against explicit expected values for every capability before round-trip testing. Require populated CustomData and AutoType; for optional custom icons, record unsupported import as unverified rather than PASS."
  },
  {
   "id": "R2",
   "severity": "medium",
   "path": "docs/plans/milestone-0.md",
   "evidence": "T2 promises coverage of DeletedObjects, but the fixture specification requires only a recycled entry. A recycled entry remains in the group tree and does not exercise deletion tombstones. A writer that discards all DeletedObjects can therefore pass the planned test with an empty collection.",
   "fix": "Include at least one nonempty DeletedObjects tombstone with a known UUID and deletion time, verify it survives fixture import, and assert its preservation after saving. If import cannot preserve it, document that capability as unverified and provide another fixture source."
  }
 ],
 "coverage": [
  "Verified the plan file SHA256 matches the supplied hash.",
  "Read BRIEF.md, PRD.md, ARCHITECTURE.md, DESIGN.md, DESIGN_PROMPT.md, TASKS.md, and the milestone review log.",
  "Reviewed planned workspace boundaries, scaffold security, compatibility oracle, fixture generation, test gating, gap handling, and Windows verification.",
  "Enumerated repository files; the checkout currently contains documentation only, with no implementation callers or shared-state writers to trace."
 ],
 "limitations": [
  "This is a review of the proposed plan, not an implemented milestone.",
  "No files were edited, tests executed, or work delegated.",
  "Dependency source, CLI behavior, generated fixtures, build configuration, and runtime behavior were not available for inspection."
 ]
}
```

### Disposizioni host (round 1)
- R1 accettato: aggiunto T0, CustomData/AutoType obbligatori nella fixture, stato UNVERIFIED, T2 asserisce la presenza delle capability nell'originale.
- R2 accettato: tombstone DeletedObjects noto nella fixture, verificato in T0/T1/T2.

## Round 2 — Codex codex-cli 0.164.0 (effective_model=gpt-6.1-sol, session 01a17d1f-fe88-7560-ad10-08129b32a48a) — plan sha256 0dfe450b619e8a45ac110966122b7dc2970ab3befa00f842495996465843830f
- Result: `/tmp/claude-0/-opt-apps-password-manager/8dafc736-041e-41d6-8225-8c7780d29f3a/scratchpad/claudex/r2/claudex-smng_8u7/result.json`

```json
{
 "verdict": "APPROVED",
 "summary": "The revision addresses R1 and R2 through explicit fixture validation, populated tombstone coverage, and PASS/GAP/UNVERIFIED reporting. No material unresolved plan defects identified.",
 "findings": [],
 "coverage": [
  "Verified the on-disk plan SHA256 matches the supplied revision.",
  "Checked revised fixture requirements and T0–T2 against both prior findings.",
  "Reviewed compatibility testing, gap handling, scaffold security, workspace isolation, and deferred Windows verification against repository specifications read in the preceding round.",
  "Re-enumerated repository files; only documentation exists, so no implementation callers or shared-state writers are available to trace."
 ],
 "limitations": [
  "Approval applies to the plan, not an implementation or a demonstrated compatibility result.",
  "No tests were run, files edited, or work delegated.",
  "Generated fixtures, dependency source, build configuration, and runtime behavior remain unverified."
 ]
}
```
- Check approvazione: eseguito su /tmp/claude-0/-opt-apps-password-manager/8dafc736-041e-41d6-8225-8c7780d29f3a/scratchpad/claudex/r2/claudex-smng_8u7/result.json (vedi output sopra).

## Build — stato al 2026-10-27 (pausa su richiesta dell'utente)
- Builder: Claude (host). Base commit pre-build: `aba9c2f7b353f4b9ea511ba95ba3329e7ad7c05a`. Lavoro NON ancora committato.
- Piano rivisto durante la build (deviazioni marcate *rev. build*: SvelteKit SPA + `ui/build`, script `pnpm tauri` = `cd .. && tauri`,
  niente `version` in tauri.conf.json, `clsx`+`tailwind-merge` al posto di `cn`, normalizzazioni T2 ridotte/aggiornate,
  cargo-deny da crates.io). Nuovo plan sha256: vedere `sha256sum docs/plans/milestone-0.md` alla ripresa.
- Verificato finora: `cargo fmt --check` ok, `cargo clippy -p keyrust-core --all-targets -D warnings` ok,
  `KEYRUST_REQUIRE_KEEPASSXC=1 cargo test -p keyrust-core` 7/7, mutation check T2 (4 mutazioni → tutte rilevate),
  `pnpm check`/`lint`/`build` ok, `tauri info` da crates/app e da ui ok.
- Da fare alla ripresa: completare `cargo deny check` (installazione cargo-deny avviata in background), rieseguire tutti i
  comandi di prova del piano, `.gitignore` di `crates/app/ui`, revisione finale dei file, ispezione Codex in sessione nuova
  (`runner.py inspect --base aba9c2f...`), commit.

## Build — ripresa 2026-10-27 pomeriggio
- cargo-deny 0.20.2 installato da crates.io. `deny.toml`: ignorata RUSTSEC-2024-0370 (proc-macro-error unmaintained, transitivo via GTK3 di Tauri su Linux, nessun fix), rimosse licenze non usate.
- Prove (host): fmt OK; clippy core OK; gen-fixtures OK (AES-KDF 2.88M round su questa macchina); test core 7/7; cargo deny: advisories/bans/licenses/sources ok; cargo metadata OK; ui install --frozen-lockfile/check/lint/format:check/build OK; tauri info da crates/app OK (CSP, frontendDist ui/build); unico .kdbx da committare: tests/fixtures/keepassxc-rich.kdbx.
- Nota: format:check è fallito una volta su button.svelte (formattazione generata prima di modifiche ad app.css lette dal plugin Tailwind); riformattato, stabile su 3 passaggi.
- Piano rivisto durante la build (passaggi *rev. build*), sha256 8a2bce98ee67770be374ba9ac31715f3e623f2a7467a92805b1ef34b5d373046. Le revisioni non sono state ri-sottoposte a plan review: l'ispezione finale le valuta contro questo piano.

## Ispezione 1 — Codex codex-cli 0.164.0 (effective_model=gpt-6.1-sol, sessione nuova 01a17da0-bfd1-7562-a6c1-dc11aff2deac), base aba9c2f
- Result: `/tmp/claude-0/-opt-apps-password-manager/8dafc736-041e-41d6-8225-8c7780d29f3a/scratchpad/claudex/i1/claudex-9820xf14/result.json`

```json
{
 "verdict": "REVISE",
 "summary": "The scaffold largely follows the plan, but compatibility verification has two concrete gaps.",
 "findings": [
  {
   "id": "M0-001",
   "severity": "medium",
   "path": "crates/core/tests/common/mod.rs:require_cli",
   "evidence": "When KeePassXC is absent and KEYRUST_REQUIRE_KEEPASSXC is unset, five tests print with eprintln! and return successfully. Rust's default test harness captures output from successful tests, so the documented plain cargo test command reports seven passing tests without displaying the skip warnings. This violates the plan's requirement that skipped verification remain visible.",
   "fix": "Make CLI-dependent verification explicitly distinguishable in normal test output, or provide a default test runner that displays captured warnings. Keep the required-CLI failure mode."
  },
  {
   "id": "M0-002",
   "severity": "medium",
   "path": "crates/core/tests/compat_keepassxc.rs:t1_keepass_rs_reads_keepassxc_fixture",
   "evidence": "T1 reduces history to filter_map(get_password), sorts it, and compares two passwords. An extra historical entry without a password, reversed history, or missing historical attachments and fields passes this check. The plan requires explicit history count and content assertions. T2 checks serialized output, which does not establish that the in-memory historical entries exposed to future callers are correct.",
   "fix": "Assert exactly two historical entries in the expected library order, and verify their expected fields, protection flags, timestamps and attachment bytes directly. Align the spike report's T1 coverage claims with those assertions."
  }
 ],
 "coverage": [
  "Read the plan, core and app Rust sources, fixture generator, XML fixture, compatibility helpers and tests.",
  "Reviewed frontend routes, components, styles, build configuration, Tauri IPC registration, CSP and capabilities.",
  "Reviewed workspace configuration, project policies, development instructions, task updates and spike report against the referenced product and architecture documents.",
  "Traced fixture generation through both readers and round-trip comparison, and traced core VERSION through app_info to the UI.",
  "Inspected Cargo lock structure and registry sources, frontend lock dependency metadata, binary asset hashes and headers, and Git status/history."
 ],
 "limitations": [
  "No tests, builds, dependency audits or fixture exports were executed; recorded host results were treated as claims.",
  "Encrypted fixture contents were not independently decrypted; inspection covered its XML source, binary header and verification code.",
  "Windows compilation, installer behavior, webview rendering and runtime CSP behavior remain unverified.",
  "Binary icons were inspected structurally, not visually; lockfiles were inspected programmatically and selectively rather than audited dependency by dependency.",
  "Implementation files remain uncommitted in the inspected working tree."
 ]
}
```

### Disposizioni host (ispezione 1)
- M0-001 ACCETTATO: gating invertito. Senza keepassxc-cli i test falliscono; skip solo con opt-out esplicito `KEYRUST_SKIP_KEEPASSXC=1`. Rimosso `KEYRUST_REQUIRE_KEEPASSXC`. Aggiornati piano (passaggio Gating, criterio 5, comandi di prova, checklist Windows), DEVELOPMENT.md, CONTRIBUTING.md, rapporto spike. Verificati i 3 casi: CLI mancante → fallimento con messaggio; opt-out → skip; CLI presente → 7/7.
- M0-002 ACCETTATO: T1 asserisce esattamente 2 versioni nell'ordine del file (gh-old-1, gh-old-2) con titolo/username/password protetta/URL/note, assenza dei campi custom, LastModificationTime e allegato readme.txt per contenuto; `historical(2)` assente. Rapporto spike allineato.
- Prove rieseguite: fmt OK, clippy core OK, test core 7/7.
- Plan sha256 dopo fix ispezione 1: d503e7feb148e71d2108568b5d93c8233220c268b3c9eb06dfe4553da80b4fe4

## Ispezione 2 — Codex codex-cli 0.164.0 (effective_model=gpt-6.1-sol, sessione nuova 01a17da3-79c4-7fb3-b4a0-b6c8a92b2847), base aba9c2f
- Result: `/tmp/claude-0/-opt-apps-password-manager/8dafc736-041e-41d6-8225-8c7780d29f3a/scratchpad/claudex/i2/claudex-9f75ky2z/result.json`

```json
{
 "verdict": "APPROVED",
 "summary": "No material unresolved defects found in the inspected Milestone 0 implementation. The CLI gating and historical-entry assertions address the previous findings. Windows verification remains explicitly pending.",
 "findings": [],
 "coverage": [
  "Verified the plan SHA256 matches the supplied value.",
  "Reviewed core compatibility tests, CLI helpers, fixture XML, generator and spike report; traced fixture creation, reads, saves, attachment resolution and semantic comparison.",
  "Reviewed Rust workspace and app sources, VERSION-to-app_info-to-UI flow, Tauri configuration, capabilities and CSP.",
  "Reviewed frontend routes, components, styles, configuration, project policies, development instructions and referenced product documents.",
  "Inspected lockfile structure and dependency metadata, binary asset hashes and headers, and Git status/history."
 ],
 "limitations": [
  "No tests, builds, audits or fixture exports were executed; host verification records were treated as claims.",
  "Encrypted fixture contents were not independently decrypted; binary icons were inspected structurally, not visually.",
  "Lockfiles were inspected selectively rather than audited dependency by dependency.",
  "Windows compilation, webview behavior, runtime CSP and installer operation remain unverified as documented.",
  "Implementation files remain uncommitted; the planned implementation commit is still pending."
 ]
}
```

- Esito finale: APPROVED. Dopo l'ispezione è stato modificato solo questo log (append dell'esito); nessun file di codice o di piano.
- Round usati: plan review 2/5, ispezioni 2/2, fix round 1/2.
