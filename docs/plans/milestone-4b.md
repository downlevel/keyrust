# Piano — Milestone 4b (pipeline di release, firma, build riproducibili, v0.1.0) di Keyrust

Riferimenti: `docs/TASKS.md` § "4b — rilascio", `docs/ARCHITECTURE.md` § Deploy e Domande aperte, `SECURITY.md`
§ "Verifying releases", `README.md`, `docs/DEVELOPMENT.md`, `.github/workflows/ci.yml`, `crates/app/tauri.conf.json`.
Base: `main` @ `0325d9a`. Branch: `milestone-4b`.

## Obiettivo
1. Un **workflow di release** (`.github/workflows/release.yml`) che, su tag `v*` di un commit di `main`, costruisce
   l'app in release sui tre OS, calcola i checksum SHA-256, li **firma con cosign keyless** e crea una **bozza** di
   GitHub Release con installer, `SHA256SUMS` e bundle di firma. La pubblicazione della bozza resta manuale.
2. **Firma codice OS opzionale**: Windows (Authenticode) e macOS (Developer ID + notarizzazione) si attivano solo se
   i secret corrispondenti esistono; senza secret la release esce non firmata a livello OS, e il log lo dice.
3. **Build riproducibili "best effort"**: input fissati, timestamp e percorsi normalizzati, un controllo automatico
   che ricostruisce il binario Linux due volte e confronta gli hash, e la procedura documentata per verificarlo.
4. Il repository è **pronto per `v0.1.0`**: versione 0.1.0 ovunque, `CHANGELOG.md`, documentazione di download e
   verifica. Il tag lo crea e lo pusha l'utente dopo il merge.

## Contesto e vincoli (confermati)
- **Decisioni dell'utente (2026-10-29)**:
  - (a) firma degli artefatti: **cosign keyless** (Sigstore, identità OIDC del workflow GitHub), niente minisign;
  - (b) pacchetti: Windows **NSIS + MSI**, Linux **.deb + AppImage**, macOS **un .dmg universale** (x86_64 + arm64);
  - (c) firma codice Windows/macOS: **opzionale**, attivata dalla presenza dei secret; nessun certificato acquistato ora;
  - (d) prova: un **dry run** che costruisce e firma senza creare release; il tag `v0.1.0` lo crea **l'utente** dopo
    il merge, la bozza la pubblica l'utente.
- **Fatti dal repository**:
  - toolchain Rust già fissata: `rust-toolchain.toml` → `1.99.1` (profilo minimal, `rustfmt`, `clippy`);
  - lockfile committati: `Cargo.lock`, `crates/app/ui/pnpm-lock.yaml` (`@tauri-apps/cli` 2.12.4, crate `tauri` 2.12.5);
    `packageManager: pnpm@12.15.0`; Node in CI è `24` (non una versione esatta);
  - `[profile.release]` già con `lto = true`, `codegen-units = 1`, `strip = true`, `panic = "abort"`;
  - `tauri.conf.json` non ha `version` → Tauri usa la versione del crate (`workspace.package.version = "0.0.1"`);
    `ui/package.json` è `0.0.1`; `bundle.targets = "all"`, nessuna configurazione `windows`/`macOS` di firma;
  - la CI esistente (`ci.yml`) costruisce già bundle **debug** non firmati su PR e `main` (`nsis`, `deb`, `dmg`);
    le action sono fissate a SHA; `permissions: contents: read` a livello di workflow;
  - `SECURITY.md` § "Verifying releases" e `ARCHITECTURE.md` § Domande aperte ("Firma codice Windows/macOS … DA
    DEFINIRE") vanno chiusi;
  - un workflow `workflow_dispatch` si può avviare solo se il file esiste sul branch di default: finché `release.yml`
    è solo su `milestone-4b`, il dry run deve partire da un altro evento (§ 1.1).
- **Macchina di sviluppo**: niente WebKitGTK né build Tauri locali, ~3 GB liberi → il workflow si verifica in CI;
  localmente solo lint statico (actionlint/YAML) e i test esistenti.
- **Macchina di test dell'utente**: PC Windows aziendale che blocca installer non firmati. Questa milestone non lo
  risolve finché non c'è un certificato Authenticode: va detto nella documentazione, senza promettere altro.
- **Autorizzato**: commit e push su `milestone-4b`. **Non autorizzato**: push su `main`, creazione di tag o release
  (li fa l'utente). L'implementazione parte solo dopo l'approvazione del piano e la conferma dell'utente.

## Criteri di accettazione

### 1. Workflow di release (`.github/workflows/release.yml`)
1. **Eventi**:
   - `push` di tag `v*` → release vera (bozza);
   - `workflow_dispatch` → dry run (utile dopo il merge, quando il file è su `main`);
   - `pull_request` verso `main` con `paths` limitati a `.github/workflows/release.yml`, `scripts/release/**`,
     `crates/app/tauri.conf.json`, `crates/app/tauri.release*.json` → dry run, così la pipeline si prova sulla PR di
     questa milestone senza creare tag. Nessun secret di firma è esposto agli eventi `pull_request` (§ 2.4).
   - Dry run = stesse build, stessi checksum e (dove possibile) stessa firma cosign; gli output vanno in un artifact
     del workflow (`keyrust-release-dryrun`, retention 7 giorni) invece che in una release.
   - **PR da fork e di Dependabot** (revisione M4B-R04): non hanno token OIDC. Lì il dry run fa build, checksum e
     riproducibilità, ma **salta la firma cosign** e lo scrive nel summary (`Signature: skipped (no OIDC token on
     fork/Dependabot PRs)`). La firma nel dry run avviene solo per PR dello stesso repository
     (`github.event.pull_request.head.repo.full_name == github.repository` e `github.actor != 'dependabot[bot]'`) e
     per `workflow_dispatch`.
2. **Permessi**: `permissions: {}` a livello di workflow; ogni job dichiara solo ciò che usa. Poiché i permessi valgono
   per l'intero job (revisione M4B-R03), firma e pubblicazione sono **job separati**: `build` e `reproducibility` →
   `contents: read`; `sign` → `contents: read` + `id-token: write`; `release` (solo su tag) → `contents: write`, senza
   `id-token`. `concurrency` per ref, senza cancellare una release in corso.
3. **Job `verify`** (ubuntu): su tag controlla che
   - il tag sia esattamente `v` + `workspace.package.version` di `Cargo.toml` e uguale alla `version` di
     `crates/app/ui/package.json` (script `scripts/release/check-version.sh`, testabile in locale);
   - il commit del tag sia raggiungibile da `origin/main` (`git merge-base --is-ancestor`), così un tag su un branch
     non pubblica nulla.
   Calcola e passa agli altri job `SOURCE_DATE_EPOCH` = timestamp del commit (`git log -1 --format=%ct`) e l'output
   **`release`** = `true` solo se `github.event_name == 'push' && startsWith(github.ref, 'refs/tags/v')` (revisione
   M4B-R08). È l'**unico predicato** che seleziona il comportamento da release vera: controlli del tag, nome
   dell'artifact (`keyrust-release` vs `keyrust-release-dryrun`), firma codice OS (§ 2.4) e job `release`. Un
   `workflow_dispatch` lanciato su un tag esistente resta quindi un dry run.
4. **Job `build`** (matrice):

   | OS runner | target | bundle |
   |---|---|---|
   | `ubuntu-24.04` | `x86_64-unknown-linux-gnu` | `deb,appimage` |
   | `windows-latest` | `x86_64-pc-windows-msvc` | `nsis,msi` |
   | `macos-latest` | `universal-apple-darwin` (aggiunge `x86_64-apple-darwin` e `aarch64-apple-darwin` con rustup) | `dmg` |

   Passi: checkout (stesso SHA dell'action di `ci.yml`), toolchain da `rust-toolchain.toml`, prerequisiti Linux
   come in `ci.yml` (più quanto serve all'AppImage, se manca), pnpm, Node con versione **esatta** da
   `crates/app/ui/.node-version` (nuovo file, usato anche da `ci.yml` per coerenza), `pnpm install --frozen-lockfile`,
   poi `pnpm tauri build --target <target> --bundles <bundle> [--config <override>] -- --locked`.
   **Nessuna cache** (`rust-cache`, cache pnpm) nel workflow di release: una cache avvelenata da un altro branch non
   deve poter finire in un artefatto firmato; la build release è più lenta e va bene così.
   Ambiente: `SOURCE_DATE_EPOCH` dal job `verify` e `RUSTFLAGS` con `--remap-path-prefix` per la cartella di lavoro,
   `$CARGO_HOME` e la home di rustup (§ 3).
   Ogni job carica gli installer prodotti (solo i file di bundle, non tutta `target/`) come artifact
   `bundle-<os>`, con un nome file che contiene versione e architettura (es. `Keyrust_0.1.0_amd64.deb`,
   `Keyrust_0.1.0_x64-setup.exe`, `Keyrust_0.1.0_x64_en-US.msi`, `Keyrust_0.1.0_universal.dmg`): quelli di Tauri se
   già univoci, altrimenti rinominati in uno step esplicito.
   Ogni job scrive nel `$GITHUB_STEP_SUMMARY` lo stato della firma codice: `Code signing: skipped (no secrets)` o
   `Code signing: Authenticode (Azure Trusted Signing)` / `Developer ID + notarization`.
5. **Job `sign`** (ubuntu, dipende da `build`; `contents: read`, `id-token: write`):
   - scarica gli artifact, verifica che ci siano **esattamente i 5 installer** attesi (revisione M4B-R01): `.deb` e
     `.AppImage` amd64, NSIS `-setup.exe` e `.msi` x64, `.dmg` universale, ciascuno con la versione nel nome; fallisce
     altrimenti, elencando quelli mancanti o in più;
   - genera `SHA256SUMS` (formato `sha256sum`, nomi senza percorso, ordinati);
   - installa cosign con `sigstore/cosign-installer` fissato a SHA (versione cosign esatta) e firma:
     `cosign sign-blob --yes --bundle SHA256SUMS.sigstore.json SHA256SUMS` (saltata solo nei casi di § 1.1 senza
     OIDC);
   - **verifica subito** la propria firma con `cosign verify-blob` e l'identità attesa (stesso comando documentato in
     `SECURITY.md`, con il ref effettivo), e `sha256sum -c SHA256SUMS`;
   - carica installer, `SHA256SUMS` e (se c'è) `SHA256SUMS.sigstore.json` come artifact `keyrust-release-dryrun` in
     dry run, o `keyrust-release` su tag (input del job `release`).
6. **Job `release`** (ubuntu, **solo su push di tag**, `contents: write`; `needs: [verify, sign, reproducibility]` con
   `if: always() && needs.sign.result == 'success' && needs.verify.outputs.release == 'true'`, così un mismatch o un
   errore di riproducibilità non blocca la release ma la bozza aspetta il suo esito — revisione M4B-R05):
   - scarica l'artifact `keyrust-release` e controlla che contenga esattamente **7 asset** (5 installer,
     `SHA256SUMS`, `SHA256SUMS.sigstore.json`);
   - note = sezione della versione estratta da `CHANGELOG.md` (script `scripts/release/changelog-section.sh`,
     fallisce se la sezione manca) + una riga "Reproducibility check" generata dall'esito del job `reproducibility`
     (`match` con l'hash, `mismatch` con i due hash, o `not available` se il job è fallito);
   - `gh release create <tag> --draft --verify-tag --title "Keyrust <versione>" --notes-file <note>` con i 7 asset.
     Se la bozza esiste già (rilancio del workflow) gli asset si aggiornano con `gh release upload --clobber` e le note
     con `gh release edit`; se la release è già **pubblicata**, il job fallisce senza toccarla.
7. Nessuna action nuova senza SHA fissato; le action nuove (`sigstore/cosign-installer`, eventualmente
   `actions/download-artifact`) hanno il SHA del tag di release indicato in commento, come in `ci.yml`.

### 2. Firma codice OS opzionale
1. **macOS** (supporto nativo di Tauri): se il secret `APPLE_CERTIFICATE` esiste, lo step di build riceve
   `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY` e, per la notarizzazione,
   `APPLE_API_ISSUER`, `APPLE_API_KEY` e il file della chiave API (`APPLE_API_KEY_PATH`, scritto da un secret base64
   in `$RUNNER_TEMP` e cancellato a fine job). Senza secret: firma **ad hoc** (`signingIdentity: "-"`) tramite
   l'override `crates/app/tauri.release.macos-adhoc.json`, così l'app parte su Apple Silicon dopo l'approvazione
   manuale di Gatekeeper.
2. **Windows**: se i secret di **Azure Trusted Signing** (`AZURE_CLIENT_ID`, `AZURE_CLIENT_SECRET`,
   `AZURE_TENANT_ID`, più le variabili `AZURE_SIGNING_ENDPOINT`, `AZURE_SIGNING_ACCOUNT`,
   `AZURE_SIGNING_PROFILE`) esistono, il job installa `trusted-signing-cli` (versione esatta, `cargo install --locked`)
   e passa a Tauri un override con `bundle.windows.signCommand` che lo invoca su `%1`: Tauri firma così l'eseguibile,
   l'installer NSIS e l'MSI. Senza secret: nessun override, build non firmata.
3. La presenza dei secret si rileva in uno step che espone solo un booleano (`steps.signing.outputs.macos == 'true'`);
   i valori dei secret entrano solo nell'`env` dello step di build che li usa, mai a livello di job o workflow.
4. Quando `needs.verify.outputs.release != 'true'` (`pull_request` e ogni `workflow_dispatch`, anche su un tag) la
   firma codice è **sempre disattivata** (i secret non vengono
   letti): solo un tag di `main` produce binari firmati.
5. **Documentazione** (`docs/RELEASING.md`, nuovo): come attivare ogni opzione, con costi e vincoli indicati come
   "verificare al momento dell'acquisto":
   - Windows: **Azure Trusted Signing** (cablato), **SignPath Foundation** (gratuito per OSS, su candidatura; firma
     tramite il loro servizio dopo la build: richiede di adattare il workflow, non cablato ora), **certificato OV/EV**
     su token hardware (non usabile su runner ospitati senza servizio di firma remoto);
   - macOS: **Apple Developer Program** (99 $/anno), certificato "Developer ID Application" esportato in `.p12`,
     chiave API App Store Connect per la notarizzazione;
   - avvertenza: anche con firma, SmartScreen può avvisare finché il certificato non ha reputazione, e una policy
     aziendale può bloccare comunque.
6. **Limite dichiarato**: i percorsi con secret non si possono provare in CI finché i secret non esistono. Il piano
   verifica il percorso "senza secret" e la correttezza statica degli override (JSON valido, schema Tauri, comando
   `signCommand` con `%1`).

### 3. Build riproducibili (best effort)
1. **Input fissati**: toolchain Rust (`rust-toolchain.toml`), Node esatto (`.node-version`), pnpm esatto
   (`packageManager`), `--frozen-lockfile` e `cargo … --locked`, action fissate a SHA, runner con immagine nominata
   (`ubuntu-24.04`; per Windows e macOS l'immagine `-latest` cambia nel tempo: documentato come limite).
2. **Normalizzazione**: `SOURCE_DATE_EPOCH` = data del commit; `--remap-path-prefix` per checkout, `CARGO_HOME` e
   rustup, così i percorsi assoluti non entrano nel binario (con `strip = true` restano comunque pochi).
   **Versione del frontend** (revisione M4B-R02): SvelteKit, se `kit.version.name` non è impostato, usa
   `Date.now()` e lo scrive in `_app/version.json`, che Tauri incorpora nel binario. `crates/app/ui/svelte.config.js`
   imposta `kit.version.name` a `process.env.SOURCE_DATE_EPOCH` quando è definita (build di release, entrambe le
   ricostruzioni e la procedura documentata), altrimenti lascia il default (sviluppo). Prova locale: due `pnpm build`
   con lo stesso `SOURCE_DATE_EPOCH` producono `build/` identiche (`diff -r`).
3. **Controllo automatico** (job `reproducibility`, ubuntu, su tag e in dry run, `contents: read`): costruisce
   `keyrust` (solo il binario, `pnpm tauri build --no-bundle -- --locked`, frontend incluso) **due volte in due
   cartelle diverse** con lo stesso ambiente e confronta gli SHA-256. Espone come output del job l'esito (`match` /
   `mismatch`) e gli hash, letti dal job `release` (§ 1.6), e li scrive nel `$GITHUB_STEP_SUMMARY` insieme all'hash del
   binario dentro il `.deb` della build ufficiale. Il job è **informativo**: un mismatch non fallisce il job (esito
   `mismatch` nell'output), un errore di build lo fa fallire ma non blocca la release (`not available` nelle note).
   Gli installer (deb, AppImage, dmg, msi) **non** sono dichiarati riproducibili bit per bit.
4. **Documentazione** in `docs/RELEASING.md` § "Reproducing a build": comandi per ricostruire il binario Linux da un
   tag (stessa toolchain, stesse variabili, container `ubuntu:24.04` o runner equivalente) e confrontarlo con quello
   estratto dal `.deb` (`dpkg-deb -x`), con i limiti noti.

### 4. Preparazione di v0.1.0
1. Versione `0.1.0` in `Cargo.toml` (`workspace.package.version`), `Cargo.lock` aggiornato di conseguenza (solo le
   righe dei crate del workspace: `cargo update --workspace` offline o modifica equivalente, nessun aggiornamento di
   dipendenze), `fuzz/Cargo.lock` (riga `keyrust-core`, workspace separato con dipendenza per percorso — revisione
   M4B-R07), `crates/app/ui/package.json`. `check-version.sh` passa con `v0.1.0`.
2. `CHANGELOG.md` (formato Keep a Changelog) con la sezione `## [0.1.0]` che riassume le funzionalità dell'MVP e i
   limiti noti (nessuna firma codice OS se non configurata, auto-lock, KDBX 3.1/4.0 convertiti a 4.1 al salvataggio).
3. **Documentazione**:
   - `SECURITY.md` § "Verifying releases": verifica di `SHA256SUMS` con cosign (comando completo con
     `--certificate-identity` del workflow `release.yml` al ref del tag e `--certificate-oidc-issuer
     https://token.actions.githubusercontent.com`), poi la verifica **del solo installer scaricato** contro la sua
     riga di `SHA256SUMS` (revisione M4B-R06): Linux `grep ' <file>$' SHA256SUMS | sha256sum -c -`, macOS
     `grep ' <file>$' SHA256SUMS | shasum -a 256 -c -` (`sha256sum` non c'è di serie), Windows confronto in PowerShell
     tra `(Get-FileHash <file> -Algorithm SHA256).Hash` e la riga del file; stato della firma codice OS; righe della
     tabella "Supported versions" aggiornate per `0.1.x`.
   - `README.md`: sezione "Download" (link alle release, quale file scegliere per OS, avvisi Gatekeeper/SmartScreen
     per build non firmate, rimando alla verifica).
   - `docs/DEVELOPMENT.md`: rimando a `docs/RELEASING.md`.
   - `docs/RELEASING.md`: procedura di release passo per passo (aggiornare versione e CHANGELOG su branch → PR → merge →
     `git tag -s`/`git tag v0.1.0` su `main` → `git push origin v0.1.0` → controllare la bozza → pubblicare), firma
     codice (§ 2.5), riproducibilità (§ 3.4), cosa fare se il workflow fallisce a metà (rilancio, bozza da cancellare).
   - `docs/ARCHITECTURE.md`: § Deploy aggiornato, domanda aperta sulla firma codice **chiusa** con la decisione (c).
   - `docs/TASKS.md`: voci 4b spuntate tranne "Release `v0.1.0`", che resta all'utente (tag e pubblicazione).
4. `ci.yml`: unica modifica = Node dalla stessa `.node-version` (nessun altro cambio di comportamento).

## Non obiettivi
- Aggiornamenti automatici (Tauri updater), pacchetti per store (winget, Homebrew, Flathub, AUR), build ARM per
  Linux/Windows.
- Acquisto o configurazione reale di certificati; integrazione cablata di SignPath.
- Attestazioni SLSA/`gh attestation`, SBOM: possibili in seguito.
- Creare il tag, pubblicare la release o modificare `main`.

## Rischi
- **AppImage e MSI in CI**: richiedono strumenti scaricati da Tauri durante la build (linuxdeploy, WiX). Se falliscono
  sul runner, si correggono i prerequisiti; non si rinuncia al formato senza chiedere all'utente.
- **Universal macOS**: build due volte più lunga; accettato.
- **Riproducibilità**: dipendenze native (WebKitGTK linkato dinamicamente, quindi fuori dal binario) e codice generato
  da `tauri-build` possono introdurre differenze; il job le rende visibili senza bloccare.
- **cosign keyless**: dipende dalla disponibilità di Fulcio/Rekor al momento della release; se non disponibili il job
  `sign` fallisce e si rilancia. La firma registra nel log pubblico di trasparenza repository, workflow e ref
  (informazioni già pubbliche).
- **Firma codice non verificata**: i rami con secret restano non provati fino alla configurazione reale (§ 2.6).

## Verifica
Locale (questa macchina):
1. `actionlint .github/workflows/release.yml .github/workflows/ci.yml` (binario actionlint scaricato con checksum
   verificato nello scratchpad, non nel repository) → nessun errore; `shellcheck` sugli script `scripts/release/*.sh`.
2. `scripts/release/check-version.sh v0.1.0` → esce 0; con `v0.1.1` e `0.1.0` → esce ≠ 0 con messaggio chiaro.
3. `scripts/release/changelog-section.sh 0.1.0` stampa la sezione; con una versione assente esce ≠ 0.
4. Override Tauri: `python3 -m json.tool` su ogni `tauri.release*.json`; i campi esistono nello schema
   `https://schema.tauri.app/config/2` (validazione con lo schema scaricato, nello scratchpad).
5. `cargo metadata --locked` (versione 0.1.0 coerente nel lockfile), `cargo test -p keyrust-core`, `cargo clippy` come
   nella memoria di sviluppo; pulizia di `target/debug/incremental` e degli eseguibili di test.
CI (sulla PR di questa milestone, aperta dall'utente):
6. `ci.yml` resta verde su tutti i job.
7. `release.yml` in dry run (evento `pull_request` dello stesso repository): job `verify` (senza controlli del tag),
   `build` ×3, `sign`, `reproducibility` completati, `release` saltato; dopo il merge, un `workflow_dispatch` lanciato
   su `main` (e, se il tag esiste già, sul tag) si comporta allo stesso modo: nessuna release, nessuna firma codice; l'artifact `keyrust-release-dryrun` contiene
   esattamente 5 installer, `SHA256SUMS`, `SHA256SUMS.sigstore.json`; il summary di ogni build riporta `Code signing:
   skipped`; `cosign verify-blob` nel job è passato; il summary di `reproducibility` mostra i due hash (atteso
   uguali; se diversi, il limite viene documentato con la causa individuata).
8. Verifica manuale (host, fuori CI): scaricare l'artifact del dry run, `cosign verify-blob` con l'identità del ref
   della PR (binario cosign con checksum verificato, nello scratchpad) e `sha256sum -c SHA256SUMS` → OK; il comando
   per un singolo installer documentato in `SECURITY.md` → OK. Controllo che `.deb`, `.dmg` universale (`lipo -info` non disponibile
   qui: si controlla nel log del job macOS con uno step `lipo -info`), NSIS e MSI abbiano versione 0.1.0 nel nome.
Dopo il merge (utente):
9. `git tag v0.1.0 && git push origin v0.1.0` → bozza di release con 7 asset (5 installer, `SHA256SUMS`,
   `SHA256SUMS.sigstore.json`) e note dal CHANGELOG più la riga di riproducibilità; l'utente la controlla e la
   pubblica. Facoltativo: provare l'installer NSIS sul PC aziendale (atteso: bloccato finché non c'è
   firma Authenticode).
