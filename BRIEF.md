# Brief iniziale — Password manager open source

> Input per `/new-app`. Decisioni già prese durante la valutazione di fattibilità (2026-10-27):
> non rimetterle in discussione, usale come risposte alle domande dell'intervista.

## Obiettivo
- Alternativa open source a KeePass, **progetto da portfolio** pubblicato sul repo GitHub personale.
- Nessuno scopo di lucro: conta la qualità ingegneristica visibile (sicurezza, test, documentazione), non la monetizzazione.
- Posizionamento: "un client KeePass moderno" — UX pulita e minimale rispetto a KeePassXC.

## Decisioni tecniche (prese)
- **Formato DB: KDBX4**, compatibile con KeePassXC / KeePassDX (Android) / Strongbox (iOS).
  Niente formato proprietario. Mobile e sync arrivano "gratis" dall'ecosistema.
- **Linguaggio core: Rust.**
- **Desktop: Tauri 2**, UI web (Svelte o React — da scegliere nell'intervista).
- **Target: Linux, Windows, macOS.** Mobile non nel perimetro iniziale.
- **Struttura a workspace Cargo:**
  - `core` — vault, crypto, formato KDBX (nessuna dipendenza dalla UI)
  - `app` — Tauri + frontend
  - (futuro) `cli` / TUI che riusa `core`
- **Crate di riferimento:** `keepass` (keepass-rs) per KDBX4, RustCrypto (`argon2`, `aes-gcm`, `chacha20poly1305`),
  `zeroize` + `secrecy` per i segreti in memoria, `getrandom` per il CSPRNG, `arboard` per la clipboard.

## Regole di sicurezza (non negoziabili)
- Nessuna primitiva crittografica scritta a mano; solo librerie standard e mantenute.
- KDF Argon2id con parametri configurabili; cifratura autenticata come da specifica KDBX4.
- Segreti azzerati in memoria (`zeroize`), mai loggati, mai serializzati in chiaro su disco.
- Clipboard auto-clear dopo N secondi; auto-lock per inattività e al blocco schermo.
- `SECURITY.md` con threat model esplicito (cosa protegge / cosa no) e policy di segnalazione vulnerabilità.

## Scope MVP (V1)
1. Crea / apri / salva DB KDBX4 con master password.
2. Voci: titolo, username, password, URL, note; organizzazione in gruppi.
3. Ricerca, generatore password configurabile, copia con auto-clear, auto-lock.
4. **Test di compatibilità bidirezionale con KeePassXC** (file creato da noi apribile in KeePassXC e viceversa).

## V2 (dopo l'MVP)
- Keyfile, TOTP integrato, import CSV (Bitwarden, Chrome), check HIBP via k-anonymity, tema scuro.

## Fuori scope (esplicitamente)
- Auto-type (problematico su Wayland).
- Estensione browser.
- Server di sync proprio → il file si sincronizza con Syncthing / Nextcloud / Dropbox.

## Qualità da portfolio
- CI (GitHub Actions): build, test, clippy, fmt su tre OS.
- Fuzzing del parser con `cargo-fuzz`.
- Release firmate, idealmente build riproducibili.
- README curato con screenshot, threat model, architettura.

## Da decidere nell'intervista `/new-app`
- Framework frontend (Svelte vs React) e libreria UI.
- Nome del progetto.
- Licenza (suggerite: GPL-3.0 come KeePassXC, oppure Apache-2.0/MIT).
- Design: stile visivo, palette, layout (lista + dettaglio, sidebar gruppi).
