# Accessibility

Keyrust aims at **WCAG 2.1 level AA** for its whole interface: complete keyboard operation, visible
focus, sufficient contrast, roles and announcements for screen readers, and respect for reduced
motion. This is a target checked by automated tests and a manual checklist, not a certified
conformance claim.

## What is checked automatically

Every check runs in CI, in the Linux `app` job. The browser audits run on the demo build in
Chromium, through Playwright (`crates/app/ui/e2e-web/`, `pnpm test:web`).

| Check | How | File |
|---|---|---|
| WCAG 2.0 / 2.1 A and AA rules | axe-core on every screen and overlay: Unlock (recent files, wrong password, first run), Create (with advanced settings and a mismatch), Vault (list, selected entry, toast, search without results, recycle bin, empty group), entry editor (new, edit, generator popover), standalone generator, Settings (each tab), Keyboard shortcuts, Command palette, Confirm dialog, external-change and save-failed banners. Fails on any *serious* or *critical* violation; *moderate* and *minor* ones are reported in the test output | `a11y.spec.ts` |
| Keyboard | Tab order in the vault (the group tree and the entry list are one tab stop each, Shift+Tab is the exact reverse); arrows, Home and End in the tree and in the list, Enter opens the editor; focus enters Settings, the Confirm dialog and the palette, stays inside, and returns to the element that opened them on Escape; every shortcut of the command registry produces its effect; menu commands without a shortcut are in the palette | `keyboard.spec.ts` |
| Visible focus | On every tab stop of Unlock, Create, Vault, the editor, each Settings tab, the palette and the Confirm dialog: an opaque outline, or an opaque box-shadow ring with a spread, appears when the element receives keyboard focus, with at least 3:1 contrast against the background next to it, measured on the computed colour | `focus.spec.ts` |
| Reduced motion | With `prefers-reduced-motion: reduce`, no animation or transition lasts more than 0.01 s and nothing scrolls smoothly while Settings, the palette, a toast and the generator popover are open; without the preference the animations remain | `motion.spec.ts` |
| Colour contrast of the design tokens | Every text/background pair the components use, at 4.5:1 for text and 3:1 for non-text elements (input borders, the focus ring, strength-meter segments, meaningful icons). Colours are read from `src/app.css` and Tailwind's palette and converted from OKLCH, so a changed token is measured as it is | `src/tests/contrast.test.ts` (Vitest) |
| Roles and names | Component tests assert the roles and names of the tree, the list, dialogs and fields | `src/tests/*.test.ts` |

The colour conversion and the WCAG contrast formula live in `src/lib/a11y/contrast.ts`; the token
test and the focus test both use it.

## What the audit changed (Milestone 4a)

The first run found 7 kinds of serious or critical axe violations, and the other tests found more.
All are fixed:

- `--muted-foreground` darkened from zinc-500 to `oklch(52% …)`: zinc-500 was 4.3:1 on the
  selected-row and muted backgrounds.
- `--input` darkened to `oklch(63% …)` so that field borders reach 3:1; the search field uses it too.
- Strength meter segments: amber-600, emerald-600 and emerald-700 instead of lighter shades below
  3:1.
- Primary buttons on hover use `--primary-hover` (indigo-700) instead of a translucent indigo at
  4.4:1.
- Focus rings are 2 px and opaque everywhere (they were 20–50 % translucent), and the command
  palette's field shows one (it had none).
- The command palette's field declares the list it controls; the list can be focused for scrolling.
- An empty entry list is no longer an empty `listbox`. The list is named after the current group
  or search ("Entries in Work", "Entries matching “git”"), and Home and End work in it.
- Group tree items are named with their count ("Work, 3 entries") and expose `aria-setsize` and
  `aria-posinset`, since the rows are flat in the page.
- The clipboard toast announces one sentence per copy instead of every second of its countdown.
  The search result count is announced once typing pauses.
- Unlock and Create errors are alerts linked to their fields; the lock notice describes the
  password field; the strength caption describes the password it rates.
- Show/hide and reveal buttons keep one name and expose their state with `aria-pressed`.
- The generator popover in the editor is a named dialog.

## Exceptions

No axe rule is disabled. Two choices are deliberate:

- **Empty strength-meter segments** use the border colour, below 3:1. The segments are decorative
  (`aria-hidden`): the strength is always written next to them ("Strong · 131 bits") and that text
  meets 4.5:1.
- **The command palette has a single tab stop.** Arrow keys in its field move through the results,
  as in a combobox; Tab does not visit each result.

## Known limits

- The audit runs in Chromium, while the app runs in WebKitGTK (Linux), WebView2 (Windows) and
  WKWebView (macOS). Colours and the DOM are the same, so contrast and roles carry over; how a
  screen reader announces them depends on the engine and is covered only by the manual checklist.
- The end-to-end test of the real app (`crates/app/e2e/`) checks behaviour, not accessibility.
- Windows high-contrast mode is not specifically styled; the app must stay usable in it.

## Manual checklist

The steps, for Windows with Narrator and no extra software, are in
[DEVELOPMENT.md](DEVELOPMENT.md#manual-accessibility-checklist). Record each run here.

| Date | Platform | Screen reader | Result |
|---|---|---|---|
| — | — | — | Not run yet |
