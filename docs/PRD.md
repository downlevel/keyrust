# PRD — Keyrust

## Visione
Keyrust è un client KeePass moderno, scritto in Rust: un password manager desktop open source,
compatibile al 100% con il formato KDBX4, con UX pulita e minimale e una qualità ingegneristica
(sicurezza, test, documentazione) visibile e verificabile.

## Problema
I client KeePass esistenti sono solidi ma datati nell'esperienza d'uso (KeePassXC su tutti):
interfacce dense, molte opzioni esposte, poca cura per i flussi quotidiani. Le alternative moderne
(1Password, Bitwarden) richiedono un account e un server di sync. Keyrust offre la UX di un
password manager moderno senza account né cloud proprietario, restando nel formato standard KDBX4:
il database resta un file dell'utente, sincronizzabile con qualsiasi strumento e apribile con
KeePassXC, KeePassDX (Android) e Strongbox (iOS).

## Utenti target
- **Utenti KeePass esistenti** su desktop (Linux, Windows, macOS) che vogliono un client più piacevole
  senza migrare formato.
- **Utenti attenti alla privacy** che non vogliono affidare le password a un servizio cloud e
  sincronizzano il file con Syncthing / Nextcloud / Dropbox.
- **Contesto secondario (portfolio)**: recruiter e sviluppatori che valutano il repo GitHub;
  codice, test, threat model e CI fanno parte del "prodotto".

## Feature
### Must-have (V1 / MVP)
1. **Gestione database KDBX4**: crea, apri, salva con master password.
   KDF Argon2id con parametri configurabili, cifratura conforme a KDBX4.
2. **Voci**: titolo, username, password, URL, note. Crea / modifica / elimina (tramite cestino KDBX).
3. **Gruppi**: albero di gruppi; crea / rinomina / sposta / elimina; spostamento voci tra gruppi.
4. **Ricerca** istantanea su titolo, username, URL e note (mai sulla password).
5. **Generatore password** configurabile: lunghezza, set di caratteri (maiuscole, minuscole, cifre,
   simboli), esclusione dei caratteri ambigui; indicatore di robustezza.
6. **Copia negli appunti con auto-clear** (default 20 s, configurabile), esclusa dalla cronologia
   appunti dove la piattaforma lo consente.
7. **Auto-lock**: per inattività (default 5 min), al blocco schermo e, opzionalmente, alla minimizzazione.
8. **Compatibilità bidirezionale con KeePassXC**: un file creato da Keyrust si apre in KeePassXC e
   viceversa, senza perdita di dati (campi, gruppi, cronologia, attributi custom preservati).
9. **Navigazione completa da tastiera** e scorciatoie per le azioni frequenti.
10. **Elenco dei file recenti** nella schermata di sblocco (solo percorsi, nessun segreto).

### Nice-to-have (V2)
- Keyfile come secondo fattore.
- TOTP integrato (generazione codici).
- Import CSV da Bitwarden e Chrome.
- Controllo password compromesse con HIBP via k-anonymity (opt-in).
- Tema scuro (in V1 i token di colore sono già predisposti).
- Traduzione italiana (in V1 l'infrastruttura i18n è già predisposta).
- CLI / TUI che riusa il crate `core`.

## User stories
- Come nuovo utente voglio creare un database protetto da una master password per iniziare a salvare le mie credenziali.
- Come utente KeePassXC voglio aprire il mio file `.kdbx` esistente in Keyrust per usarlo senza migrazioni.
- Come utente voglio che il file salvato da Keyrust resti apribile in KeePassXC/KeePassDX per non restare vincolato a un solo client.
- Come utente voglio organizzare le voci in gruppi per ritrovarle in base al contesto (lavoro, banca, email).
- Come utente voglio cercare una voce digitando parte del titolo, dello username o dell'URL per trovarla in pochi tasti.
- Come utente voglio copiare username o password con un clic o una scorciatoia e veder svuotare gli appunti dopo pochi secondi, per non lasciare il segreto in giro.
- Come utente voglio generare una password robusta con regole configurabili per creare credenziali nuove senza pensarci.
- Come utente voglio che il vault si blocchi da solo quando mi allontano o blocco lo schermo, così nessuno vede le mie password.
- Come utente voglio eliminare una voce e poterla recuperare dal cestino, per correggere un errore.
- Come utente avanzato voglio regolare i parametri Argon2id del database per bilanciare sicurezza e velocità di sblocco.
- Come utente da tastiera voglio fare tutto senza mouse per essere veloce.

## Modello dati (concettuale)
Il modello è quello di KDBX4; Keyrust non aggiunge entità proprie nel file.
- **Database**: metadati (nome, impostazioni KDF/cifratura, cestino abilitato), credenziali composite
  (master password; keyfile in V2), un **Gruppo radice**.
- **Gruppo**: nome, icona, UUID, timestamp; contiene sottogruppi e voci (albero).
- **Voce**: UUID, campi standard (Title, UserName, Password, URL, Notes, con Password protetta in memoria),
  campi custom, tag, timestamp, **cronologia** (versioni precedenti della voce), allegati (preservati, non gestiti in UI in V1).
- **Cestino**: gruppo speciale KDBX in cui finiscono le voci/gruppi eliminati.
- **Impostazioni app** (fuori dal DB, su disco locale): file recenti, timeout clipboard/auto-lock,
  opzioni generatore. Nessun segreto.

## Requisiti non funzionali
- **Sicurezza** (non negoziabili):
  - nessuna primitiva crittografica scritta a mano, solo librerie standard e mantenute;
  - Argon2id configurabile, cifratura autenticata come da spec KDBX4;
  - segreti azzerati in memoria (`zeroize`/`secrecy`), mai loggati, mai serializzati in chiaro su disco;
  - clipboard con auto-clear, auto-lock per inattività e blocco schermo;
  - `SECURITY.md` con threat model esplicito e policy di segnalazione vulnerabilità.
- **Integrità dei dati**: salvataggio atomico (scrittura su file temporaneo + rename) per non corrompere
  il DB in caso di crash o di sync concorrente; nessuna perdita di campi sconosciuti al round-trip.
- **Performance**: sblocco dominato dal KDF (circa 1 s con i default); ricerca sotto i 50 ms su 5.000 voci;
  avvio dell'app sotto 1 s escluso il KDF.
- **Piattaforme**: Linux (X11 e Wayland), Windows 10+, macOS 12+.
- **Accessibilità**: navigazione da tastiera completa, focus visibile, contrasto WCAG AA, ARIA sui componenti.
- **i18n**: UI in inglese; tutte le stringhe estratte in file di traduzione.
- **Privacy**: nessuna telemetria, nessuna chiamata di rete in V1.
- **Qualità**: CI su 3 OS (build, test, clippy, fmt), fuzzing del parsing con `cargo-fuzz`, release firmate.

## Fuori scope (v1)
- Auto-type (problematico su Wayland), anche oltre la V1.
- Estensione browser, anche oltre la V1.
- Server di sync proprio: la sincronizzazione è delegata a Syncthing / Nextcloud / Dropbox.
- App mobile (coperte dall'ecosistema: KeePassDX, Strongbox).
- Keyfile, TOTP, import CSV, HIBP, tema scuro, italiano (→ V2).
- Gestione allegati in UI (gli allegati esistenti vengono comunque preservati).
- Merge automatico dei conflitti di sync.

## Domande aperte
- Strategia sui **conflitti di sync** (file modificato da un altro client mentre è aperto): in V1 basta
  rilevare la modifica esterna e avvisare prima di sovrascrivere? DA DEFINIRE.
- Gestione in sola lettura dei **database KDBX3.1** (aprire e proporre l'upgrade a KDBX4?). DA DEFINIRE.
