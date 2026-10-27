# Piano — Milestone 3a (core vault in Rust) di Keyrust

Riferimenti: `docs/TASKS.md` (Milestone 3, sezioni "core" e "Compatibilità"), `docs/ARCHITECTURE.md`, `docs/PRD.md`,
`docs/spikes/0001-keepass-rs-kdbx4.md`, `docs/plans/milestone-1.md`. Base: `main` @ `342bb7f`. Branch: `milestone-3`.

## Obiettivo
Dare a `keyrust-core` un `Vault` completo e testato contro KeePassXC:
- aprire, creare e salvare in modo atomico file KDBX4, rilevando le modifiche esterne;
- operazioni su gruppi, voci, cronologia e cestino;
- ricerca, generatore di password, stima di robustezza (zxcvbn), calibrazione e modifica di Argon2id, cambio della
  master password;
- segreti che non finiscono mai in `Debug` né nei log.

La Milestone 3 è divisa in tre fasi, ognuna con il proprio piano, branch e PR, per decisione dell'utente:
- **3a (questa)**: libreria core;
- **3b**: comandi IPC, UI collegata al vault reale al posto della demo, appunti in Rust con auto-clear, dialoghi
  file;
- **3c**: auto-lock (inattività, blocco schermo sui 3 OS, minimizzazione), limiti KDF in apertura e fuzzing del
  parsing completo.

## Contesto e vincoli (confermati)
- **Macchina di sviluppo**: Linux con ~1,5 GB liberi e `keepassxc-cli` 2.7.6. Il core si compila e si testa in
  locale; la CI ripete tutto sui 3 OS, dove gira un KeePassXC recente, che esporta `Ref="0"` per gli allegati: gli
  helper di test lo gestiscono già.
  - In locale si compila con `CARGO_PROFILE_DEV_DEBUG=line-tables-only` per contenere `target/`, che a fine lavoro
    viene ripulito.
- **Decisioni dell'utente**:
  - (a) tre fasi;
  - (b) modifiche esterne: **rilevamento e avviso**, senza merge;
  - (c) robustezza con **zxcvbn** (crate `zxcvbn`, MIT).
- **keepass-rs `=0.17.0`** (spike 0001):
  - i valori protetti sono `SecretBox` (zeroize);
  - `DatabaseKey` deriva `Debug`;
  - `History::add_entry` inserisce la versione **in testa** (indice 0), mentre KeePass/KeePassXC salvano la cronologia
    dalla più vecchia alla più recente (T1 lo verifica sulla fixture);
  - `EntryTrack` (`edit_tracking`) usa `add_entry`, quindi non lo usiamo.
- **Autorizzato**: commit e push su `milestone-3`. **Non autorizzato**: push su `main` (PR aperta e mergiata
  dall'utente).

## Criteri di accettazione
1. **API pubblica** (`keyrust_core::vault`, più `model`, `error`, `generator`, `strength`, `kdf`, `search`).
   - Il modello KDF descrive ciò che c'è nel file e non lo inventa:
     - `KdfInfo::AesKdf { rounds }` oppure `KdfInfo::Argon2 { variant: Argon2d | Argon2id, memory_bytes,
       iterations, parallelism }`, letti dall'header;
     - i parametri **modificabili** sono un tipo separato, `Argon2idParams { memory_mib, iterations, parallelism }`,
       usato da `create`, `set_kdf` e `calibrate`;
     - un database AES-KDF o Argon2d resta tale finché `set_kdf` non viene chiamato esplicitamente;
     - il contratto dei tipi del frontend (`types.ts`, `KdfParams`) viene aggiornato di conseguenza nella fase 3b.
   - Nessun tipo pubblico espone una password tranne:
     - `SecretString` restituita da `Vault::reveal_field(id, Field::Password)`;
     - `EntryDraft`, la bozza dell'editor, che contiene la password in un `SecretString`.
   - Gli id sono UUID (`EntryId`, `GroupId`) serializzabili per l'IPC della fase 3b.
   - I tipi di vista (`EntrySummary`, `EntryDetail`, `GroupNode`, `DatabaseInfo`, `KdfParams`, `GeneratorOptions`,
     `StrengthEstimate`) corrispondono a quelli del frontend (`ui/src/lib/api/types.ts`).
2. **Apertura**.
   - `Vault::open(path, password)` e `Vault::open_bytes(bytes, password)` restituiscono errori tipizzati (`thiserror`)
     senza segreti né percorsi nei messaggi:
     - `WrongKey`;
     - `InvalidFormat`;
     - `UnsupportedVersion { major, minor }`, ricavato da `kdbx::header` oppure dall'errore di keepass-rs;
     - `Io(kind)`.
   - Si aprono le fixture `keepassxc-rich.kdbx` (KDBX 4.1) e `keepassxc-kdbx31.kdbx` (KDBX 3.1). Salvando un file
     3.1 lo si scrive in KDBX4: è documentato.
3. **Creazione**.
   - `Vault::create(path, CreateOptions { name, password, cipher, kdf })` crea un database KDBX4 con:
     - gruppo radice con il nome del database e cestino abilitato;
     - Argon2id con i parametri dati;
     - cifratura AES-256 o ChaCha20.
   - `kdf::default_params()` = Argon2id 64 MiB, p=2, con iterazioni calibrate a ~1 s da `kdf::calibrate`.
   - KeePassXC apre il file creato e `db-info` riporta cifrario, KDF e parametri.
4. **Salvataggio atomico e modifiche esterne**.
   - **Stampo legato ai byte analizzati**: `open` legge il file una sola volta in memoria, analizza quei byte e ne
     calcola lo stampo (lunghezza + SHA-256) dagli stessi byte. Dopo un salvataggio lo stampo è quello dei byte
     appena scritti. Una scrittura esterna avvenuta durante l'apertura non può quindi finire associata al modello in
     memoria.
   - **Sequenza di `save()`**:
     1. prende un lock advisory esclusivo (`File::lock`) su un file accanto al database, `.<nome>.keyrust-lock`, che
        coordina più istanze di Keyrust;
     2. rilegge il file e confronta lo stampo;
     3. serializza in un file temporaneo esclusivo nella stessa directory, poi `fsync`;
     4. **ricontrolla lo stampo subito prima del `rename`**;
     5. esegue il `rename` e, su Unix, `fsync` della directory;
     6. rilascia il lock.
   - Se lo stampo non corrisponde, in un controllo o nell'altro, `save()` restituisce `ExternalChange` senza
     sostituire il file. Le alternative sono `save_overwrite()` ("Keep mine") e `reload()` ("Reload").
   - **Rischio residuo documentato**: uno scrittore che non usa il lock (KeePassXC, un client di sincronizzazione)
     può ancora scrivere nell'intervallo tra il secondo controllo e il `rename`. La finestra si riduce a una sola
     chiamata di sistema, ma non si può chiudere senza supporto del sistema operativo.
   - **Test deterministici dell'interleaving**: un hook di test (`#[cfg(test)]`, o un trait interno per il file
     system) scrive contenuto esterno tra lettura e analisi in `open`, tra il primo controllo e la serializzazione, e
     tra `fsync` e `rename`. In tutti e tre i casi il risultato è `ExternalChange`, oppure, per `open`, un modello
     coerente con lo stampo.
   - **Credenziali**: il vault distingue le **credenziali confermate** (quelle con cui è cifrato il file su disco) da
     quelle **in attesa** impostate da `change_master_password`.
     - `save()` usa quelle in attesa e, solo dopo il `rename` riuscito, le rende confermate.
     - `reload()` usa sempre le credenziali confermate e sostituisce il modello solo dopo un'analisi riuscita: se
       fallisce, il vault resta com'era, con le modifiche e le credenziali in attesa.
     - Test: reload prima e dopo il salvataggio di un cambio password.
   - Un errore di scrittura lascia intatto il file originale; il test lo simula con una directory in sola lettura.
   - Uno stato `dirty` dice se ci sono modifiche non salvate.
5. **Gruppi**:
   - `tree()` (con conteggi e flag del cestino);
   - `create_group(parent, name)`, `rename_group`;
   - `move_group(id, new_parent)`, che rifiuta cicli, la radice e i movimenti dentro il cestino;
   - `delete_group(id)`: va nel cestino se il cestino è abilitato e il gruppo non è già lì, altrimenti
     l'eliminazione è definitiva;
   - ogni eliminazione **definitiva** (voce, gruppo e tutti i discendenti) registra una tombstone in
     `deleted_objects` (UUID e ora), così un merge in un altro client KeePass non fa ricomparire gli oggetti. Si usano
     le API di rimozione tracciata di keepass-rs (che scrivono le tombstone) solo per l'eliminazione, oppure si scrive
     la tombstone esplicitamente; la modifica tracciata resta esclusa per la cronologia;
   - il cestino viene creato al primo uso, come fa KeePassXC (`meta.recyclebin_uuid`, icona 43).
6. **Voci**.
   - Operazioni:
     - `list_entries()` restituisce `EntrySummary`, cestino incluso, con `group_id`;
     - `get_entry(id)` restituisce `EntryDetail` senza password;
     - `reveal_field(id, field)`;
     - `entry_for_edit(id)` restituisce `EntryDraft`.
   - `save_entry(draft)` crea o aggiorna una voce:
     - aggiorna titolo, username, password (sempre protetta), URL, note e gruppo;
     - **non tocca** campi personalizzati, allegati, dati personalizzati, auto-type, tag, icone né la protezione degli
       altri campi.
   - Ogni modifica effettiva aggiunge la versione precedente alla **fine** della cronologia, dalla più vecchia alla
     più recente come KeePass. Una modifica senza cambiamenti non crea versioni.
   - La cronologia viene potata secondo `meta.history_max_items` (default 10) e `history_max_size` (default 6 MiB),
     togliendo le versioni più vecchie.
   - Le date di modifica e accesso vengono aggiornate; la data di creazione resta invariata.
   - `move_entry`.
   - `delete_entry` sposta nel cestino, salvando il gruppo precedente in `previous_parent_group`; dal cestino, o con
     il cestino disabilitato, l'eliminazione è definitiva. `restore_entry` riporta la voce nel gruppo precedente
     (oppure nella radice).
7. **Ricerca**.
   - `search(query)` cerca su titolo, username, URL e note (mai sulla password né sui campi protetti), senza
     distinzione tra maiuscole e minuscole, con più termini in AND.
   - Si appoggia a un indice minuscolo ricostruito a ogni modifica.
   - Su 5.000 voci impiega meno di 50 ms in build di test (`cargo test`, il profilo usato anche in CI).
8. **Generatore**.
   - `generator::generate(&GeneratorOptions)` usa il CSPRNG del sistema (`getrandom`) con campionamento senza bias
     (rejection sampling).
   - Contiene almeno un carattere per ogni set attivo quando la lunghezza lo permette.
   - "Exclude ambiguous" toglie `Il1O0o`.
   - La lunghezza è nell'intervallo 8–128 e almeno un set resta sempre attivo; le regole coincidono con
     `Settings::normalized`.
   - Test: vincoli dei set, assenza degli ambigui e distribuzione grossolanamente uniforme (chi-quadro su
     10.000 caratteri con soglia larga, deterministico nei limiti statistici).
9. **Robustezza**.
   - `strength::estimate(&str)` usa `zxcvbn` e restituisce `StrengthEstimate { level, bits }`:
     - `level` 0 per una password vuota; altrimenti punteggio zxcvbn 0–1 → 1 (Weak), 2 → 2 (Fair), 3 → 3 (Good),
       4 → 4 (Strong);
     - `bits` = `log2` dei tentativi stimati, arrotondato.
   - Test: `Password123!` è Weak o Fair; una frase casuale lunga è Strong.
10. **KDF e master password**.
    - `kdf::calibrate(memory_mib, parallelism, target)` misura Argon2id (`rust-argon2`, la stessa implementazione di
      keepass-rs) e restituisce le iterazioni, almeno 2.
    - `set_kdf(params)` e `set_cipher(cipher)` cambiano i parametri; `change_master_password(new)` cambia la chiave.
      Hanno effetto dal salvataggio successivo, e KeePassXC lo verifica: si apre con la nuova password, `db-info`
      riporta i nuovi parametri, la vecchia password viene rifiutata.
    - `rename_database`, `set_recycle_bin_enabled`.
11. **Segreti**.
    - La master password è tenuta come `SecretString` (secrecy/zeroize) e la `DatabaseKey` viene creata solo per la
      durata di open e save.
    - `Debug` di `Vault`, `EntryDraft` e degli errori è scritto a mano e redatto.
    - Il drop del `Vault` (lock) azzera la master password.
    - Test: `format!("{vault:?}")` e `format!("{draft:?}")` non contengono password, username, URL né note delle
      fixture; `logging_policy.rs` esteso alle operazioni del vault (open, save, save_entry, delete) a livello INFO,
      senza valori né percorsi.
12. **Compatibilità con KeePassXC** (oracolo indipendente), nuovi test in `crates/core/tests/vault_keepassxc.rs`:
    - **V1**: un database creato e popolato con l'API del `Vault` (gruppi annidati, voci, cestino, una voce modificata
      due volte) viene letto da KeePassXC con contenuto atteso, cronologia in ordine dal più vecchio e password
      protette.
    - **V2**: aprire `keepassxc-rich.kdbx`, modificare una voce con `save_entry` e salvare. Nell'export KeePassXC
      cambiano solo i campi modificati, l'ultima versione in cronologia è quella precedente e le versioni esistenti
      restano in ordine. Tutto il resto, campi personalizzati, allegati, auto-type e dati personalizzati compresi,
      resta identico (diff canonico esistente, limitato alla voce modificata).
    - **V3**: delete, restore e delete_group, verificati nell'export (gruppo del cestino, `PreviousParentGroup`). Le
      eliminazioni definitive di una voce e di un gruppo con discendenti producono nuove voci in `DeletedObjects`
      con gli UUID attesi, e le tombstone preesistenti della fixture restano.
    - **V4**: cambio di password, KDF e cifrario, verificati con `db-info` e con il rifiuto della password vecchia.
      Il file AES-KDF della fixture, aperto e salvato senza `set_kdf`, resta AES-KDF con gli stessi round.
    - **V5**: il file salvato dopo la modifica esterna di un altro processo produce `ExternalChange`; `reload()`
      vede il contenuto nuovo.
    - **T1–T6** restano verdi.

## Approccio e decisioni
- **`Vault` avvolge `keepass::Database`**: ogni operazione lavora sul modello di keepass-rs, così tutto ciò che Keyrust
  non mostra viene conservato. Nessuna conversione verso un modello proprio.
- **Cronologia gestita da Keyrust**:
  - si costruisce un `History` nuovo inserendo la versione nuova e poi le vecchie in ordine inverso, perché
    `add_entry` inserisce in testa; così l'ordine finale è dal più vecchio al più recente;
  - la potatura è secondo i limiti di `Meta`;
  - un test guarda l'ordine nel file tramite KeePassXC.
- **Cestino come KeePassXC**: gruppo creato al primo uso, `recyclebin_uuid` e `recyclebin_changed` aggiornati;
  `previous_parent_group` su voci e gruppi.
- **Rilevamento delle modifiche esterne** per stampo (lunghezza + SHA-256 dei byte), invece dell'mtime: la
  sincronizzazione può preservare l'mtime, e lo stampo calcolato dagli stessi byte analizzati evita di associarlo a
  un contenuto diverso. Il lock advisory coordina solo le istanze di Keyrust; il rischio residuo con altri scrittori
  è documentato in `SECURITY.md` e nell'architettura.
- **CSPRNG con `getrandom`** direttamente (già nel grafo delle dipendenze) invece di `rand` 0.10, la cui API è cambiata
  di recente: il campionamento senza bias è piccolo e testato.
- **Nuove dipendenze del core**:
  - `secrecy` 0.10, `zeroize` 1, `thiserror` 2, `getrandom`, `zxcvbn` 3, `sha2` (già usato da keepass-rs);
  - `rust-argon2` passa da dev-dependency a dipendenza normale;
  - tutte verificate da `cargo deny`; licenze attese MIT/Apache.

## Non-goals
- Comandi IPC, UI reale, appunti, dialoghi file e apertura degli URL (3b).
- Auto-lock, limiti KDF in apertura (protezione dai file ostili), fuzzing di `Database::parse` / `open_bytes` (3c).
- Merge delle modifiche esterne, scrittura in KDBX 3.1, allegati, campi personalizzati e cronologia nell'interfaccia,
  key file e YubiKey.

## Rischi
- **Disco**: la compilazione del core con i test occupa ~1 GB in `target/`. Mitigazioni: `line-tables-only` in locale,
  `df` controllato, pulizia di `target/debug/incremental` a fine lavoro.
- **Tempo dei test con KDF reali**: i test usano Argon2id con parametri piccoli (16 MiB, 2 iterazioni), come T3; la
  calibrazione viene testata con un obiettivo di 50 ms.
- **Differenze tra le versioni di KeePassXC** (2.7.6 in locale, recente in CI) nell'export: gli helper restano basati
  su identità (UUID, versione, nome) e non sulla numerazione.
- **zxcvbn**: dimensione dei dizionari, accettata dall'utente; la licenza viene controllata da `cargo deny`.

## Verifica
Locale (Linux, host):
```
CARGO_PROFILE_DEV_DEBUG=line-tables-only cargo test -p keyrust-core          # unit + T0–T6 + V1–V5 + logging policy
cargo clippy -p keyrust-core --all-targets -- -D warnings
cargo fmt --all --check
cargo deny check
```
Attesi: tutti i test verdi; il test di ricerca sulle 5.000 voci sotto i 50 ms; nessuna nuova licenza fuori dalla
allowlist.

Mutation check (host):
- ripristinare l'ordine di cronologia di keepass-rs (versione nuova in testa) fa fallire V1/V2;
- saltare il controllo dello stampo fa fallire V5;
- includere le note nell'output di `Debug` fa fallire il test dei segreti;
- generare senza garantire i set attivi fa fallire il test del generatore.

Remota: CI su `milestone-3` verde su tutti i job, con i test di compatibilità su Windows e macOS (KeePassXC recente).

## Toolchain per provider
- **Builder** (Claude, host): Rust 1.99.1, `keepassxc-cli` 2.7.6, cargo-deny 0.20.2; rete verso crates.io e
  api.github.com; push via SSH (account `downlevel`).
- **Reviewer/Inspector** (Codex, sandbox read-only): legge file e diff con comandi di sola lettura; non esegue i comandi
  di prova, build o test (li esegue l'host). Non raggiunge GitHub: lo stato della CI viene fornito dall'host.
