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

## Implementazione — scostamenti dal piano (Claude)
- **D-Bus in `crates/os`, non nel crate app**: `on_suspend` su Linux (logind `PrepareForSleep(true)`, bus di sistema) e
  nuovo `on_screen_lock` (`ActiveChanged(true)` di qualsiasi interfaccia `*.ScreenSaver`, quindi anche GNOME, MATE e
  Cinnamon oltre a `org.freedesktop.ScreenSaver`). Motivo: il crate app non si compila sulla macchina di sviluppo (niente
  WebKitGTK), `crates/os` sì, e il codice è comunque safe. `zbus` 5 era già nel lockfile tramite tauri-plugin-opener.
  `SuspendGuard` è diventato `Guard`.
- **Un solo thread locker**: tutti i trigger inviano un `Reason` su un canale; né la callback di power di Windows né il
  rilevatore di salti d'orologio aspettano il mutex del vault, che un salvataggio tiene durante la KDF (altrimenti un
  salvataggio lento sembrerebbe una sospensione).
- Il polling dello schermo (Windows, macOS) agisce sul **fronte** bloccato/sbloccato, non a ogni giro.
- La logica pura (`Session`, `idle_expired`, `suspended`, `should_lock`) sta in `crates/app/src/autolock/policy.rs`;
  verificata in locale con un crate di prova che la include, il resto con `cargo clippy --target x86_64-pc-windows-msvc`.

## Ispezione 1 — Codex codex-cli 0.164.0, sessione nuova 01a1819b-d830-7d51-8e1b-167567550c28 (modello effettivo gpt-6.1-sol, sandbox read-only) — REVISE (2 high, 2 medium, 1 low)
- Snapshot: base `1f07b58`, HEAD `d98233e`, diff sha256 a6386940…8199. Result:
  `/tmp/claude-0/-opt-apps-password-manager/9c131df7-ecce-4811-bfd9-d7aa204db9e5/scratchpad/claudex/m3c-i1/claudex-szgsw7y9/result.json`
- M3C-R01 (high) KDBX 3: il controllo leggeva il **primo** `TransformRounds`, keepass-rs usa l'**ultimo**; un header oltre 64 KiB
  passava → **ACCETTATO** (verificato in keepass-rs 0.17 `format/kdbx3.rs`; in più un campo da 9 byte veniva accettato
  e keepass-rs ne legge i primi 8). Nuovo `kdbx::header::kdbx3_transform_rounds`: ultima occorrenza, larghezze esatte,
  id ≤ 10, fine header entro il tetto, altrimenti errore.
- M3C-R02 (high) header KDBX 3 malformati arrivano alle letture non controllate di keepass-rs (panic) → **ACCETTATO**:
  `check_version` rifiuta con `InvalidFormat` ogni header che non passa la validazione. Test unitari e a livello di
  vault sulla fixture KeePassXC 3.1 con campi inseriti; fixture 3.1 aggiunta al corpus (`kdbx31-header.kdbx`).
  Il lato KDBX 4 (`header::peek`) era già rigoroso e coerente con keepass-rs (ultima occorrenza vince).
- M3C-R03 (medium) richiesta idle obsoleta dopo un salvataggio lento → **ACCETTATO**: `lock_session_if` ricontrolla la
  scadenza sotto il mutex della sessione; `install` aggiorna l'attività sotto lo stesso mutex.
- M3C-R04 (medium) CI non esegue i test di `keyrust-os` né `fuzz_seeds` → **ACCETTATO**: step `Test (core, fuzzing harness)`
  e `Test (os)` nel job core sui tre OS.
- M3C-R05 (low) campi KDF delle impostazioni del database senza `max` → **ACCETTATO**: `KDF_LIMITS` e `clampKdf` condivisi
  da creazione e impostazioni.
- Piano aggiornato nei passaggi interessati (criterio 1 KDBX 3.1 e frontend, criterio 4 inattività): nuovo sha256
  addbb735…cbe.
- Prove dopo le correzioni (Claude): `cargo test -p keyrust-core --features fuzzing` con KeePassXC (66 + 11 + 2 + 3 + 5
  passati), clippy core con `fuzzing`, clippy `keyrust-app` per Windows, frontend 169 test + lint + check.
