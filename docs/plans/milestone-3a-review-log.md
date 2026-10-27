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

## Build (host Claude)
- 66bf1ea: implementazione. Verifiche locali: 45 unit + T0–T6 + V1–V5 + logging policy verdi, clippy e fmt puliti,
  `cargo deny` ok; mutation check (ordine della cronologia, controllo dello stampo, Debug, set del generatore) tutti
  rilevati.
- Scoperto durante la build: KeePassXC scrive KDBX 4.0 quando non servono funzioni 4.1 e keepass-rs salva solo 4.1,
  quindi il vault porta la versione a 4.1 al salvataggio (coperto da V5).

## Inspection 1 — Codex codex-cli 0.164.0 (sessione nuova 01a17f55-6ab3-7ab0-925d-2f4c48479856) — REVISE, 1 high + 5 medium
- Result: `/tmp/claude-0/-opt-apps-password-manager/8dafc736-041e-41d6-8225-8c7780d29f3a/scratchpad/claudex/m3ai1/claudex-bro4kjbz/result.json`. Tutti **ACCETTATI**; i 5 test nuovi falliscono sul codice di
  66bf1ea e passano dopo le correzioni (50 unit verdi, integrazione invariata):
  - **R1 (high)** gli spostamenti tracciati di keepass-rs aggiungevano una versione in testa alla cronologia → spostamenti
    non tracciati con `location_changed` esplicito; test su move / cestino / ripristino / modifica + spostamento.
  - **R2** una password importata non protetta restava tale → `save_entry` la protegge sempre (senza nuova versione).
  - **R3** il limite di dimensione lasciava sempre una versione e ignorava gli allegati → dimensione con i dati degli
    allegati; limiti a 0 svuotano la cronologia.
  - **R4** il salvataggio rendeva 0644 un vault 0600 → tmp 0600 e permessi dell'originale copiati prima del rename.
  - **R5** `fsync` della directory ignorato (e saltato per percorsi relativi) → errore propagato come `NotDurable` dopo
    il rename, con lo stato del vault allineato al file.
  - **R6** directory dei lock in `/tmp` condivisa tra utenti (e deviazione non documentata dal piano) → lock accanto
    al database come da piano, permessi 0600.
- Effetto collaterale accettato: anche `settings.json` (che usa lo stesso helper per il tmp) ora nasce con permessi 0600.
- CI su 66bf1ea (run 40589699239): core Linux/Windows/macOS, deny e fuzz **success** (V1–V5 anche con KeePassXC recente);
  i job app sono stati annullati dal push successivo.

## Inspection 2 — Codex codex-cli 0.164.0 (sessione nuova 01a17f58-b2c0-76d0-8d5e-536bf0bb4f2e) — REVISE, 3 medium
- Result: `/tmp/claude-0/-opt-apps-password-manager/8dafc736-041e-41d6-8225-8c7780d29f3a/scratchpad/claudex/m3ai2/claudex-h_8ie1_4/result.json` (snapshot 065e4f1). Tutti **ACCETTATI** (secondo e ultimo round di
  correzioni); i 3 test nuovi del vault falliscono sul codice di 065e4f1 e passano dopo (54 unit verdi):
  - **M3A-01** `Debug` derivato delle viste esponeva username, URL, note e nomi → `Debug` redatto per tutte le viste.
  - **M3A-02** parametri Argon2 validati solo singolarmente (lanes vs memoria, overflow della memoria in keepass-rs,
    errori della calibrazione ignorati) → `kdf::validate` condiviso da vault e calibrazione; `calibrate` restituisce
    `Result`.
  - **M3A-03** il salvataggio attraverso un link simbolico sostituiva il link → il vault risolve il percorso reale
    all'apertura e alla creazione (cartella).
- **Budget di ispezione esaurito** (MAX_INSPECTION_ROUNDS=2): queste correzioni non sono state ispezionate da Codex.
