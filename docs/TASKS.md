# Roadmap — Keyrust

## Milestone 0 — Setup
- [x] `git init` (repo locale, branch `main`)
- [ ] Repo GitHub pubblico `keyrust`, branch `main` protetto *(in attesa di autorizzazione)*
- [x] `LICENSE` GPL-3.0-or-later, `README.md` scheletro, `CONTRIBUTING.md`, `.editorconfig`, `.gitignore`
- [x] Workspace Cargo con `crates/core` e `crates/app`; `rust-toolchain.toml` con toolchain fissata
- [x] Verificare il supporto KDBX4 in scrittura di `keepass-rs` (feature flag, round-trip lossless di campi custom/allegati/cronologia) con uno spike → [spike 0001](spikes/0001-keepass-rs-kdbx4.md): adottato
- [ ] Fixture KeePassXC create dalla GUI con Argon2id + ChaCha20 e Argon2d *(su Windows, vedi DEVELOPMENT.md)*
- [x] Scaffold Tauri 2 + SvelteKit (SPA) / Svelte 5 + TypeScript + Vite in `crates/app` (pnpm) — compilazione e avvio *(da verificare su Windows)*
- [x] Tailwind CSS + shadcn-svelte, font Inter e JetBrains Mono bundlati, icone Lucide
- [x] Configurare `rustfmt`, `clippy` (`-D warnings`), ESLint + Prettier, `cargo deny`
- [x] Bozza `SECURITY.md`: threat model (cosa protegge / cosa no) e policy di segnalazione vulnerabilità

## Milestone 1 — Infrastruttura
- [x] GitHub Actions: matrice Linux/Windows/macOS con fmt, clippy, test Rust, lint/test frontend, `tauri build`
- [x] Job `cargo deny` in CI (advisories, licenze, sorgenti; copre anche `cargo audit`)
- [x] Generare fixture `.kdbx` con KeePassXC (campi custom, allegati, cronologia, cestino; KDBX 4.1 e 3.1) in `tests/fixtures/`
- [ ] Fixture KeePassXC con Argon2id/Argon2d e ChaCha20: `keepassxc-cli` non permette di sceglierli, serve la GUI (task utente)
- [x] Installare `keepassxc-cli` nei job Linux, Windows e macOS per i test di compatibilità
- [x] Setup `cargo-fuzz` in `fuzz/` con target `kdbx_header_peek` e run breve in CI
- [x] Tauri: capabilities minime, CSP restrittiva, nessun accesso FS/shell dal frontend
- [x] `tracing` configurato con policy "nessun valore di campo nei log"
- [x] Infrastruttura i18n (libreria scelta, `en` come lingua base, estrazione stringhe)
- [x] Persistenza impostazioni app (`settings.json` nella config dir, senza segreti)

## Milestone 2 — Design (UI scaffolding)
- [x] Prototipare l'interfaccia partendo da DESIGN_PROMPT.md e validare la direzione visiva (prototipo approvato)
- [x] Impostare design system / libreria componenti e tema (token CSS zinc + indaco, tipografia, raggi), predisposto al dark
- [x] Realizzare layout e navigazione: Unlock → Vault a 3 colonne, Settings, menu nativo, routing/stato schermate
- [x] Creare i componenti UI ricorrenti: sidebar tree, search input, entry list item, field row, secret input, strength meter, toast, banner, dialog di conferma
- [x] Stati vuoto / errore / caricamento per Unlock, Vault, ricerca e salvataggio
- [x] Gestione scorciatoie da tastiera, pannello "Keyboard Shortcuts" e command palette
- [x] Design logo / icona app

## Milestone 3 — Feature core
### core (Rust)
- [x] `Vault::create` con KDF Argon2id (default 64 MiB / p=2 / iterazioni calibrate ~1 s) e cifratura KDBX4
- [x] `Vault::open` da file e da bytes, errori tipizzati (password errata, formato non valido, versione non supportata)
- [x] Salvataggio atomico (tmp + fsync + rename) e rilevamento modifiche esterne (mtime/hash)
- [x] Segreti con `secrecy`/`zeroize`; `Debug` redatto; test che verificano l'assenza di segreti in log/debug
- [x] Modello gruppi: albero, crea / rinomina / sposta / elimina
- [x] Voci: CRUD con campi standard, cronologia aggiornata a ogni modifica, preservazione dei campi sconosciuti
- [x] Eliminazione tramite cestino KDBX e ripristino
- [x] Ricerca su titolo / username / URL / note (esclusa la password), < 50 ms su 5.000 voci
- [x] Generatore password con il CSPRNG di sistema (`getrandom`) + stima robustezza (zxcvbn); test sui vincoli dei set di caratteri
- [x] Calibrazione e modifica parametri Argon2id; cambio master password

### app (Tauri + Svelte)
- [x] Comandi IPC: unlock, create, save, lock, list_groups, list_entries (solo `EntrySummary`), get_entry, reveal_field, update_entry, delete, search, generate
- [x] `copy_field` lato Rust con `arboard`: auto-clear dopo N s solo se il contenuto è invariato, hint di esclusione cronologia, clear al lock/uscita
- [x] Auto-lock per inattività (timer nel backend resettato dall'input), gestito dal processo Rust (3c)
- [x] Auto-lock al blocco schermo: Linux (D-Bus `*.ScreenSaver`), Windows e macOS (polling di `crates/os`) (3c)
- [x] Auto-lock alla sospensione: Windows (notifica di power), Linux (logind), macOS (salti d'orologio) (3c)
- [x] Lock opzionale alla minimizzazione (3c)
- [x] Limiti di apertura prima della KDF (`TooExpensive`), letture limitate (3c)
- [x] Schermate: Welcome/Unlock con file recenti, wizard Create database, Vault, Entry editor, Generator popover, Settings
- [x] Indicatore "unsaved changes" e salvataggio (Ctrl+S)

### Compatibilità e sicurezza
- [x] Test KeePassXC → Keyrust: apertura di tutte le fixture con verifica del contenuto atteso
- [x] Test Keyrust → KeePassXC: DB generati verificati con `keepassxc-cli ls/show/export`
- [x] Test round-trip lossless: fixture → salva → export XML con `keepassxc-cli`, nessuna differenza
- [x] Target fuzz `vault_open_bytes` (semi generati con keepass-rs) e `vault_roundtrip`; corsa lunga manuale in CI (3c)
- [ ] Verifica manuale con KeePassDX (Android) e Strongbox (iOS) su un DB sincronizzato

## Milestone 4 — Polish & deploy
### 4a — accessibilità, e2e, documentazione
- [ ] Audit accessibilità: contrasto AA, focus, ARIA (`tree`, `listbox`, `aria-live`), ordine di tab, reduced motion
  - [x] verifiche automatiche in CI (axe, tastiera, focus, reduced motion, contrasto dei token) e correzioni (4a)
  - [ ] checklist manuale con Narrator e zoom 200% su Windows *(task utente, [DEVELOPMENT.md](DEVELOPMENT.md#manual-accessibility-checklist))*
- [x] Test e2e smoke (tauri-driver) su Linux in CI (4a)
- [x] `SECURITY.md` definitivo con threat model completo (4a)
- [x] README curato: screenshot, feature, threat model sintetico, diagramma architettura, istruzioni di build, confronto con KeePassXC (4a)
- [x] Domande aperte del PRD chiuse: conflitti di sync, KDBX 3.1/4.0 con avviso di conversione (4a)
- [x] `DatabaseInfo.version` segue il file su disco anche dopo un salvataggio rifiutato (4a)

### 4b — rilascio
- [x] Pipeline di release su tag: build tre OS, checksum SHA-256, firma artefatti (cosign keyless), bozza GitHub Release (4b)
- [x] Decidere ed eventualmente configurare la firma codice Windows (Authenticode) e macOS (Developer ID + notarizzazione):
  opzionale, attivata dai secret; nessun certificato per ora (4b, [RELEASING.md](RELEASING.md#code-signing-optional))
- [x] Build riproducibili "best effort" (`SOURCE_DATE_EPOCH`, lockfile, toolchain fissata) e documentazione per verificarle (4b)
- [x] Release [`v0.1.0`](https://github.com/downlevel/keyrust/releases/tag/v0.1.0) pubblicata il 2026-10-30: firma cosign e checksum di tutti gli installer verificati, build Linux riproducibile

## Dopo l'MVP (V2)
- [ ] Keyfile
- [ ] TOTP integrato
- [ ] Import CSV (Bitwarden, Chrome)
- [ ] Check HIBP via k-anonymity (opt-in)
- [ ] Tema scuro (secondo set di token)
- [ ] Traduzione italiana
- [ ] `crates/cli` (CLI/TUI su `core`)
