# Prompt per Claude Design — Keyrust

> Incolla il blocco qui sotto in claude.ai/design per generare un prototipo dell'interfaccia.

---

Crea un prototipo dell'interfaccia per **Keyrust**, un password manager desktop open source (Linux, Windows, macOS)
compatibile con KeePass: un "client KeePass moderno" per chi vuole tenere le password in un file locale, senza
account né cloud, con un'esperienza più pulita e minimale di KeePassXC. L'interfaccia è in inglese.

**Schermate da prototipare** (in ordine di priorità):
1. **Vault (schermata principale, 3 colonne)**: a sinistra la sidebar gruppi (~220px) con nome database, "All entries",
   albero gruppi con contatore (Email, Work, Banking, Social…), "Recycle Bin" in fondo, "+ New group" e pulsante Lock;
   al centro la lista voci (~320px) con barra di ricerca (hint `Ctrl F`) e righe con iniziale colorata, titolo,
   username e dominio; a destra il dettaglio della voce selezionata: titolo grande, campi Username, Password
   (mascherata `••••••••` con azioni reveal e copy), URL cliccabile, Notes, metadati created/modified e pulsante "Edit".
   Mostra un toast in basso "Password copied · clears in 20s" con countdown.
2. **Unlock**: card centrata (~400px) con logo, nome database e percorso abbreviato, campo master password con toggle
   mostra/nascondi e avviso Caps Lock, pulsante primario "Unlock"; sotto, lista "Recent" di file `.kdbx` e i link
   "Open another file…" e "Create new database".
3. **Entry editor**: il pannello di dettaglio in modalità form (Title, Username, Password con indicatore di robustezza
   e pulsante "Generate", URL, Notes, selettore gruppo) con Save / Cancel.
4. **Password generator (popover)**: anteprima della password in monospace grande con pulsante rigenera, slider lunghezza
   (default 20), toggle Lowercase / Uppercase / Digits / Symbols / Exclude ambiguous, barra robustezza con etichetta
   testuale, pulsante "Use password".
5. **Create database (wizard)**: scelta del file, nome database, master password + conferma con indicatore di robustezza,
   sezione "Advanced" collassata con parametri Argon2id (memory, iterations, parallelism) e pulsante "Benchmark (1s)".
6. **Settings** (dialog con tab laterali): General (clipboard clear after N seconds, auto-lock after N minutes,
   lock on screen lock, lock on minimize), Database (nome, cifratura, Argon2id, cestino, cambio master password), About.

**Navigazione**: si parte da Unlock → Vault; dal Vault si seleziona un gruppo per filtrare la lista, una voce per vederne il
dettaglio, "Edit" per modificarla inline; Lock riporta a Unlock. Menu nativo File / Entry / Group / Tools / Help e command
palette (Ctrl+K). Tutto è usabile da tastiera (Ctrl+N nuova voce, Ctrl+C copia password, Ctrl+B copia username,
Ctrl+L blocca, Ctrl+G generatore).

**Stile visivo**:
- Palette: neutri scala zinc di Tailwind (sfondo zinc-50/bianco, bordi zinc-200, testo zinc-900 e zinc-500);
  accento indaco (indigo-600, hover indigo-700, selezione indigo-50); semantici emerald-600 (successo), amber-500
  (avviso), red-600 (errore/distruttivo).
- Tipografia: Inter per l'interfaccia (titolo voce 20px semibold, corpo 14px, metadati 12px, intestazioni di sezione
  13px semibold maiuscoletto grigio); JetBrains Mono per password e valori tecnici.
- Tema: solo chiaro (i colori sono token, il dark arriverà dopo).
- Tono visivo: minimale, calmo, professionale; tanto spazio bianco, pochi bordi, gerarchia data da peso e colore del
  testo. Raggio 8px sui componenti, 12px sui modali. Icone Lucide.
- Riferimento / design system: shadcn-svelte + Tailwind CSS; ispirazione Linear, Raycast, 1Password 8.

**Componenti ricorrenti**: albero gruppi, campo di ricerca, riga lista voci con stato selezionato (sfondo indigo-50 e barra
indaco a sinistra), riga campo label/valore con azioni copy/reveal al hover, input segreto, strength meter a 4 livelli
(Weak / Fair / Good / Strong), toast, banner di avviso, dialog di conferma, command palette, indicatore "unsaved changes"
accanto al nome del database.

**Responsive**: app desktop con finestra minima 800×560; sopra 1100px tre colonne complete, tra 800 e 1100px la sidebar
gruppi si collassa a icone; il pannello di dettaglio ha larghezza massima di circa 720px.

**Stati da mostrare**: Vault vuoto ("No entries yet. Create your first one with Ctrl+N"), ricerca senza risultati
("No matches for “…”"), sblocco in corso (spinner nel pulsante, "Unlocking…"), password errata (errore inline
"Wrong password or key. Check Caps Lock and try again."), banner "This database was changed by another app" con Reload /
Keep mine, errore di salvataggio con "Retry" e "Save as…".

**Accessibilità**: contrasto WCAG AA, focus ring indaco ben visibile su ogni elemento interattivo, navigazione completa da
tastiera, robustezza password comunicata anche a testo e non solo col colore.

Logo e icona dell'app: DA DEFINIRE (usa per ora un semplice wordmark "Keyrust" con un'icona chiave stilizzata).

Genera schermate ad alta fedeltà, coerenti tra loro e pronte da iterare.
