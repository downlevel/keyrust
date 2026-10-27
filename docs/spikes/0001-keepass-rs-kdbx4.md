# Spike 0001 — keepass-rs e compatibilità KDBX4 con KeePassXC

- **Data**: 2026-10-27
- **Versione testata**: `keepass = "=0.17.0"` (feature `save_kdbx4`), licenza MIT
- **Oracolo**: KeePassXC 2.7.6 (`keepassxc-cli`, Ubuntu 24.04)
- **Test**: `crates/core/tests/compat_keepassxc.rs` (helper in `crates/core/tests/common/mod.rs`)
- **Comando**: `cargo test -p keyrust-core` (con `keepassxc-cli` installato), esito **7 passed, 0 failed, 0 ignored**

## Domanda
Il crate `keepass` (keepass-rs) legge e scrive KDBX4 in modo compatibile con KeePassXC e senza perdere dati?
È il rischio tecnico principale di Keyrust (PRD, requisito MVP 4).

## Metodo
KeePassXC è l'oracolo indipendente: il contenuto di un file viene stabilito da `keepassxc-cli export --format xml`
(e da `attachment-export` per i byte degli allegati), mai da keepass-rs stesso.

1. **Fixture**: `tests/fixtures/src/rich.xml` (XML KeePass 2 scritto a mano) viene importato da KeePassXC con
   `scripts/gen-fixtures.sh`, producendo `tests/fixtures/keepassxc-rich.kdbx` (KDBX **4.1**, AES-256, AES-KDF).
2. **T0**: l'export KeePassXC della fixture viene confrontato con valori attesi espliciti per ogni capability,
   così da sapere cosa l'import ha davvero conservato.
3. **T1**: keepass-rs apre la fixture; asserzioni esplicite su ogni capability, inclusa la cronologia nel modello in
   memoria (esattamente 2 versioni nell'ordine del file, con campi, flag di protezione, timestamp e byte degli allegati).
4. **T2**: keepass-rs apre la fixture e la risalva; si confrontano gli export KeePassXC dell'originale e della
   copia: (a) le stesse asserzioni di T0 su entrambi, (b) confronto completo dell'XML canonicalizzato (ogni
   elemento e attributo, allegati risolti per contenuto), con le sole normalizzazioni elencate sotto.
5. **T3**: keepass-rs crea un DB da zero (Argon2id + AES-256, e Argon2id + ChaCha20); KeePassXC lo apre e
   ne verifica contenuto, cipher e KDF (`db-info`).
6. **T4**: password errata rifiutata con un errore sia da keepass-rs sia da KeePassXC (che non stampa contenuto).

**Gating**: se `keepassxc-cli` manca i test che lo richiedono **falliscono** (uno skip sarebbe invisibile, perché il
test harness nasconde l'output dei test riusciti); saltarli richiede l'opt-out esplicito `KEYRUST_SKIP_KEEPASSXC=1`.

**Controprova (mutation check, eseguita a mano)**: alterando di proposito il database prima del salvataggio
T2 fallisce in tutti e quattro i casi provati: tombstone `DeletedObjects` svuotati, CustomData del database
svuotati, AutoType rimosso da una voce (intercettati dalle asserzioni esplicite) e note di un gruppo cambiate
(non coperte da alcuna asserzione: intercettate dal confronto XML completo). Il confronto rileva quindi perdite
reali, non solo quelle esplicitamente asserite.

## Risultati

| Capability | Esito | Copertura |
|---|---|---|
| Lettura KDBX 4.1 creato da KeePassXC (AES-256, AES-KDF) | **PASS** | T0, T1 |
| Albero gruppi annidati, note, icone standard, Times | **PASS** | T0–T2 |
| Campi standard, Unicode (emoji, CJK, accenti), note multilinea, caratteri XML speciali | **PASS** | T0–T2 |
| Campi custom con flag protetto / non protetto | **PASS** | T0–T3 |
| Tag | **PASS** | T0–T2 |
| Allegati (testo e binario non UTF-8), anche condivisi con la cronologia | **PASS** | T0–T3 |
| Cronologia delle voci (2 versioni, ordine preservato) | **PASS** | T0–T3 |
| AutoType (sequenza di default e associazione finestra) | **PASS** | T0–T2 |
| CustomData su voce e database | **PASS** | T0–T2 |
| Icona custom (Meta e riferimento dalla voce) | **PASS** | T0–T2 |
| Cestino (flag, UUID, voce eliminata nel gruppo) | **PASS** | T0–T2 |
| Tombstone `DeletedObjects` (UUID + DeletionTime) | **PASS** | T0–T2 |
| Round-trip lossless (export KeePassXC identico dopo save) | **PASS** | T2 |
| Scrittura KDBX4 Argon2id + AES-256 apribile da KeePassXC | **PASS** | T3 |
| Scrittura KDBX4 Argon2id + ChaCha20 apribile da KeePassXC | **PASS** | T3 |
| Password errata → errore, nessun panic / nessun leak | **PASS** | T4 |
| Lettura di file KeePassXC con **Argon2id/Argon2d o ChaCha20** | **UNVERIFIED** | `keepassxc-cli import` non permette di scegliere KDF/cipher: da coprire con file creati dalla GUI KeePassXC (task Windows) |
| Valore di `_LAST_MODIFIED` nei CustomData | **UNVERIFIED** | non osservabile: KeePassXC lo riscrive a ogni export (vedi sotto) |
| KDBX 3.1, keyfile, challenge-response | fuori scope | non testati in questo spike |

**Nessun GAP** (perdita di dati o incompatibilità) trovato.

### Normalizzazioni del confronto (T2)
Tutte motivate da come KeePassXC esporta, non da keepass-rs:
- **Item CustomData `_LAST_MODIFIED` esclusi**: `keepassxc-cli export` lo valorizza con l'ora dell'**export**, non
  del file (due export dello stesso file a 3 secondi di distanza differiscono). Per far emergere sempre eventuali
  altri valori dipendenti dall'ora, T2 attende 1,1 s tra i due export.
- **Item CustomData confrontati come mappa** (ordinati per chiave): KeePassXC li esporta in ordine non deterministico.
- **Riferimenti agli allegati (`Ref`) risolti per contenuto**: sono indici del pool interno, il cui ordine può cambiare.
- Nessun'altra normalizzazione: anche `Meta/Generator` resta identico dopo il salvataggio con keepass-rs.

## Osservazioni rilevanti per l'implementazione (Milestone 3)
1. **Default di keepass-rs deboli**: `DatabaseConfig::default()` usa **Argon2d con 1 MiB** di memoria e 50
   iterazioni. Keyrust deve sempre impostare esplicitamente la propria configurazione (Argon2id, 64 MiB, p=2,
   iterazioni calibrate, come da ARCHITECTURE.md).
2. **`DatabaseKey` deriva `Debug`**: `format!("{key:?}")` stampa la master password in chiaro. Il core deve
   incapsularla (es. in `secrecy::SecretString`), costruire il `DatabaseKey` solo all'ultimo momento e non
   esporlo mai a log o errori. `Value::Protected` usa invece `SecretBox` e non stampa il segreto; i campi
   non protetti sì.
3. **Argon2 tramite `rust-argon2`**: keepass-rs non usa il crate `argon2` di RustCrypto citato nel brief ma
   `rust-argon2` 3.x. È una dipendenza transitiva: va considerata nell'audit delle dipendenze.
4. **Versione scritta**: keepass-rs scrive KDBX **4.1** (`KDBX4_CURRENT_MINOR_VERSION = 1`); KeePassXC 2.7.6
   la apre correttamente. Il README upstream definisce la scrittura "experimental": i test di compatibilità
   restano quindi obbligatori in CI a ogni aggiornamento del crate (versione fissata con `=`).
5. **Prestazioni in debug**: senza ottimizzazioni il KDF è lentissimo; il workspace compila le dipendenze con
   `opt-level = 3` anche nel profilo dev (`[profile.dev.package."*"]`).
6. **Allegati e `--stdout`**: `keepassxc-cli attachment-export --stdout` altera i byte non UTF-8; gli helper di
   test esportano su file.

## Decisione
**Adottare keepass-rs 0.17.0** come layer KDBX di `keyrust-core`, con la versione fissata e la suite di
compatibilità come guardia di regressione. Nessun contributo upstream necessario per l'MVP.

Restano aperti, come task:
- fixture create dalla **GUI di KeePassXC su Windows** con Argon2id + ChaCha20 e Argon2d (KeePassXC → Keyrust);
- verifica manuale con KeePassDX / Strongbox (Milestone 3, TASKS.md).
