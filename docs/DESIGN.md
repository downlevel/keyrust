# Design — Keyrust

Principio guida: **"un client KeePass moderno"**, cioè poche cose ben fatte, interfaccia calma, segreti
mai esposti per default, tutto raggiungibile da tastiera. UI in inglese (i testi qui sotto sono in inglese
perché sono il copy reale dell'app).

## Struttura e navigazione
### Sezioni / pagine
- **Welcome / Unlock**: schermata d'avvio; sblocco dell'ultimo DB, elenco file recenti, apri file, crea nuovo DB.
- **Create database** (wizard): percorso file, nome DB, master password con conferma e indicatore di robustezza,
  impostazioni avanzate di cifratura (collassate).
- **Vault**: schermata principale a 3 colonne (gruppi, lista voci, dettaglio voce).
- **Entry editor**: modifica/creazione voce, nel pannello di dettaglio (inline, non in finestra separata).
- **Password generator**: popover dal campo password dell'editor, e accessibile anche dal menu.
- **Settings** (modale/pannello): *General* (clipboard, auto-lock), *Database* (nome, parametri Argon2id,
  cifratura, cestino, cambio master password), *About* (versione, licenza, link a SECURITY.md).
- **Locked**: ritorno alla schermata Unlock con il DB corrente preselezionato.

### Voci di menu
Menu nativo dell'app (Tauri) e command palette con le stesse azioni:
- **File** → New Database…, Open Database…, Open Recent ▸, Save (Ctrl+S), Lock (Ctrl+L), Close Database, Quit
- **Entry** → New Entry (Ctrl+N), Edit (Ctrl+E / Enter), Copy Username (Ctrl+B), Copy Password (Ctrl+C), Copy URL (Ctrl+U), Delete (Del)
- **Group** → New Group, Rename, Delete
- **Tools** → Password Generator (Ctrl+G), Database Settings…
- **Help** → Keyboard Shortcuts (Ctrl+/), About Keyrust

### Sitemap
```
Welcome / Unlock
├── Create database (wizard)
└── Vault
    ├── Sidebar gruppi → filtra la lista
    ├── Lista voci → seleziona → Dettaglio voce
    │                              └── Modifica voce → Generatore password (popover)
    ├── Settings (General | Database | About)
    └── [Lock] → Welcome / Unlock
```

### Flussi principali
- **Primo avvio**: Welcome → "Create new database" → wizard (file, nome, master password) → Vault vuoto con empty state.
- **Sblocco quotidiano**: avvio → Unlock con ultimo DB preselezionato e focus sul campo password → Invio → Vault.
- **Trovare e copiare**: Vault → Ctrl+F → digita → ↓/↑ → Ctrl+C (copia password) → toast "Password copied · clears in 20s".
- **Nuova voce**: Ctrl+N → editor nel pannello di dettaglio → campo password → "Generate" → Save (Ctrl+S salva voce e file).
- **Eliminazione**: Del → conferma → voce nel gruppo "Recycle Bin" → ripristinabile da lì.
- **Auto-lock**: inattività / blocco schermo → Vault sparisce → Unlock con il DB preselezionato.
- **Modifica esterna del file**: banner nel Vault "This database was changed by another app" → Reload / Keep mine (vedi Domande aperte).

## Stile visivo
- **Tono**: minimale, calmo, professionale; tanto spazio bianco, pochi bordi, gerarchia data da peso e colore del
  testo più che da box. Riferimenti: Linear, Raycast, 1Password 8.
- **Palette colori** (definita come CSS custom properties / token shadcn, così il dark in V2 è solo un secondo set):
  - Neutri: scala **zinc** di Tailwind (background `zinc-50`/white, superfici `white`, bordi `zinc-200`,
    testo primario `zinc-900`, secondario `zinc-500`).
  - Primario / accento: **indaco** (`indigo-600`, hover `indigo-700`, sfondi tenui `indigo-50`) per selezione,
    pulsanti primari, focus ring.
  - Semantici: successo `emerald-600` (es. password forte, copiato), avviso `amber-500` (modifica esterna,
    password debole), errore/distruttivo `red-600` (password errata, elimina).
- **Tipografia**: **Inter** per l'interfaccia (bundled, nessun font remoto); **JetBrains Mono** per password
  rivelate, generatore e valori tecnici. Scala: titolo voce 20px/600, intestazioni sezione 13px/600 maiuscoletto
  `zinc-500`, corpo 14px/400, metadati 12px.
- **Spaziature e griglia**: scala Tailwind a base 4px; padding pannelli 16–24px; righe lista alte 52px
  (titolo + username); raggio bordi 8px (componenti), 12px (modali).
- **Icone**: Lucide (coerente con shadcn); icone gruppo semplici, monocromatiche.
- **Tema**: solo chiaro in V1, con tutti i colori espressi come token (`--background`, `--foreground`, `--primary`,
  `--muted`, `--border`, `--ring`…) pronti per il dark mode in V2.
- **Design system / libreria componenti**: shadcn-svelte (su bits-ui) + Tailwind CSS.

## Componenti e layout
### Layout schermate principali
- **Unlock**: finestra centrata, card unica (~400px): logo + nome DB e percorso abbreviato, campo master password
  (con toggle mostra/nascondi e avviso Caps Lock), pulsante "Unlock"; sotto, lista "Recent" e link
  "Open another file…" / "Create new database".
- **Vault (3 colonne)**:
  - *Sidebar gruppi* (~220px, ridimensionabile): nome DB in alto, voce "All entries", albero gruppi con contatore
    voci, "Recycle Bin" in fondo, pulsante "+ New group"; pulsante Lock sempre visibile.
  - *Lista voci* (~320px): barra di ricerca in alto (Ctrl+F), righe con icona/iniziale, titolo, username in
    secondario, dominio URL; ordinamento per titolo; pulsante "+ New entry".
  - *Dettaglio voce* (resto dello spazio): titolo grande, poi campi in coppie label/valore; password mascherata
    `••••••••` con azioni "reveal" e "copy"; ogni campo ha il pulsante copy al hover/focus; URL cliccabile (apre il
    browser di sistema); note in testo multilinea; in fondo metadati (created/modified) e pulsante "Edit".
- **Entry editor**: stesso pannello in modalità form; campi Title, Username, Password (+ generatore, indicatore
  robustezza), URL, Notes, selettore gruppo; azioni Save / Cancel in basso, uscita con Esc (conferma se modificato).
- **Password generator (popover)**: anteprima in monospace grande con pulsante rigenera, slider lunghezza (8–128,
  default 20), toggle per set di caratteri, toggle "exclude ambiguous", barra robustezza, "Use password".
- **Settings**: dialog con tab laterali (General / Database / About); i parametri Argon2id stanno in una sezione
  "Advanced" con spiegazione e pulsante "Benchmark (1s)".

### Componenti ricorrenti
- **Sidebar tree** — albero gruppi navigabile da tastiera (frecce, Invio, F2 rinomina).
- **Search input** — con icona, scorciatoia mostrata (`Ctrl F`), pulsante clear.
- **Entry list item** — titolo, username, dominio; stato selezionato con sfondo `indigo-50` e barra indaco.
- **Field row** — label + valore + azioni (copy, reveal); valore segreto sempre mascherato per default.
- **Secret input** — campo password con toggle visibilità, avviso Caps Lock, autocomplete disabilitato.
- **Strength meter** — barra a 4 livelli con etichetta testuale (Weak / Fair / Good / Strong), non solo colore.
- **Toast** — feedback di copia con countdown ("Password copied · clears in 20s").
- **Banner** — avvisi persistenti nel Vault (modifica esterna, errore di salvataggio).
- **Dialog di conferma** — per eliminazioni e modifiche non salvate.
- **Command palette** (Ctrl+K) — ricerca azioni e voci.
- **Indicatore "unsaved changes"** — puntino accanto al nome DB nella sidebar.

### Responsive
App desktop: finestra minima 800×560.
- ≥ 1100px: 3 colonne complete.
- 800–1100px: sidebar gruppi collassabile a icone (o toggle), lista e dettaglio restano affiancati.
- Il pannello di dettaglio ha larghezza massima di lettura (~720px) e non si allarga oltre su schermi ampi.

## Accessibilità e contenuti
- **Accessibilità**:
  - contrasto WCAG AA su testo e componenti (verificare `zinc-500` su bianco per il testo secondario);
  - focus ring indaco visibile su ogni elemento interattivo; ordine di tab logico sidebar → ricerca → lista → dettaglio;
  - navigazione completa da tastiera e pannello "Keyboard Shortcuts" (Ctrl+/);
  - ruoli ARIA: `tree` per i gruppi, `listbox` per le voci, `aria-live="polite"` per toast e countdown appunti;
  - la robustezza password è comunicata anche via testo, non solo colore; rispetto di `prefers-reduced-motion`.
- **Microcopy / tono**: inglese, breve, diretto, rassicurante, mai allarmista; verbi d'azione sui pulsanti
  ("Unlock", "Copy password", "Create database"). Spiegazioni tecniche solo in "Advanced". Esempi:
  - errore sblocco: "Wrong password or key. Check Caps Lock and try again."
  - lock: "Locked after 5 minutes of inactivity."
  - salvataggio: "Saved" (discreto, nella status area).
- **Stati UI**:
  - *Vuoto*: Vault senza voci ("No entries yet. Create your first one with Ctrl+N"), gruppo vuoto, ricerca senza
    risultati ("No matches for “…”"), nessun file recente in Welcome.
  - *Caricamento*: durante il KDF di sblocco/salvataggio, spinner nel pulsante e "Unlocking…" (il KDF può durare
    circa 1 s), con input disabilitati; mai blocco della UI senza feedback.
  - *Errore*: password errata (inline sotto il campo, campo svuotato), file non trovato o non valido, formato non
    supportato, errore di salvataggio (banner con "Retry" e "Save as…"), file modificato esternamente.

## Decisioni prese in Milestone 2
Dal prototipo approvato (https://claude.ai/artifact/7NiAtLZoAk2V5uZmCcyydh) e dall'implementazione:
- **Icona per voce**: iniziale del titolo su un colore tenue scelto in modo stabile dal titolo (6 coppie
  sfondo/testo, token `--avatar-N-*`, tutte AA). Niente favicon (richiederebbero rete).
- **Pulsanti copy**: Username e URL al passaggio del mouse o al focus da tastiera; Password sempre visibili
  (reveal e copy), perché è l'azione più frequente.
- **Banner**: ambra per "modificato da un'altra app" (Reload / Keep mine), rosso per l'errore di salvataggio
  (Retry / Save as…); stanno sopra il pannello di dettaglio.
- **Toast**: scuro, in basso al centro, con countdown e pulsante per chiuderlo; regione `aria-live`.
- **Logo**: chiave bianca su quadrato indaco arrotondato (`crates/app/icons/source.svg`), riusata nel wordmark.
- **Layout stretto** (800–1100 px): sidebar a icone con pulsante per espanderla.
- **Copia password**: `Ctrl+C` / `⌘C` con una voce selezionata e il focus fuori dai campi di testo; passa
  dall'evento `copy`, quindi non interferisce con la copia del testo selezionato.
- **Modalità demo** (fino alla Milestone 3): badge "Demo data" sempre visibile su Unlock, Create e Vault.

## Domande aperte
- Comportamento esatto del banner "modifica esterna" (dipende dalla strategia di sync, vedi PRD).
- Wordmark tipografico "Keyrust" oltre all'icona: rimandato.
