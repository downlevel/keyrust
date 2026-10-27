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
- [ ] `Vault::create` con KDF Argon2id (default 64 MiB / p=2 / iterazioni calibrate ~1 s) e cifratura KDBX4
- [ ] `Vault::open` da file e da bytes, errori tipizzati (password errata, formato non valido, versione non supportata)
- [ ] Salvataggio atomico (tmp + fsync + rename) e rilevamento modifiche esterne (mtime/hash)
- [ ] Segreti con `secrecy`/`zeroize`; `Debug` redatto; test che verificano l'assenza di segreti in log/debug
- [ ] Modello gruppi: albero, crea / rinomina / sposta / elimina
- [ ] Voci: CRUD con campi standard, cronologia aggiornata a ogni modifica, preservazione dei campi sconosciuti
- [ ] Eliminazione tramite cestino KDBX e ripristino
- [ ] Ricerca su titolo / username / URL / note (esclusa la password), < 50 ms su 5.000 voci
- [ ] Generatore password con `OsRng` + stima robustezza; test sui vincoli dei set di caratteri
- [ ] Calibrazione e modifica parametri Argon2id; cambio master password

### app (Tauri + Svelte)
- [ ] Comandi IPC: unlock, create, save, lock, list_groups, list_entries (solo `EntrySummary`), get_entry, reveal_field, update_entry, delete, search, generate
- [ ] `copy_field` lato Rust con `arboard`: auto-clear dopo N s solo se il contenuto è invariato, hint di esclusione cronologia, clear al lock/uscita
- [ ] Auto-lock per inattività (timer resettato dall'input)
- [ ] Auto-lock al blocco schermo: Linux (D-Bus logind/ScreenSaver), Windows (WTS session notification), macOS (`com.apple.screenIsLocked`)
- [ ] Lock opzionale alla minimizzazione
- [ ] Schermate: Welcome/Unlock con file recenti, wizard Create database, Vault, Entry editor, Generator popover, Settings
- [ ] Indicatore "unsaved changes" e salvataggio (Ctrl+S)

### Compatibilità e sicurezza
- [ ] Test KeePassXC → Keyrust: apertura di tutte le fixture con verifica del contenuto atteso
- [ ] Test Keyrust → KeePassXC: DB generati verificati con `keepassxc-cli ls/show/export`
- [ ] Test round-trip lossless: fixture → salva → export XML con `keepassxc-cli`, nessuna differenza
- [ ] Target fuzz `open_from_bytes` con corpus dalle fixture; run lunga manuale e correzione dei crash
- [ ] Verifica manuale con KeePassDX (Android) e Strongbox (iOS) su un DB sincronizzato

## Milestone 4 — Polish & deploy
- [ ] Audit accessibilità: contrasto AA, focus, ARIA (`tree`, `listbox`, `aria-live`), ordine di tab, reduced motion
- [ ] Test e2e smoke (tauri-driver) su Linux in CI
- [ ] `SECURITY.md` definitivo con threat model completo
- [ ] README curato: screenshot, feature, threat model sintetico, diagramma architettura, istruzioni di build, confronto con KeePassXC
- [ ] Pipeline di release su tag: build tre OS, checksum SHA-256, firma artefatti (minisign/cosign), bozza GitHub Release
- [ ] Decidere ed eventualmente configurare la firma codice Windows (Authenticode) e macOS (Developer ID + notarizzazione)
- [ ] Build riproducibili "best effort" (`SOURCE_DATE_EPOCH`, lockfile, toolchain fissata) e documentazione per verificarle
- [ ] Release `v0.1.0`

## Dopo l'MVP (V2)
- [ ] Keyfile
- [ ] TOTP integrato
- [ ] Import CSV (Bitwarden, Chrome)
- [ ] Check HIBP via k-anonymity (opt-in)
- [ ] Tema scuro (secondo set di token)
- [ ] Traduzione italiana
- [ ] `crates/cli` (CLI/TUI su `core`)
