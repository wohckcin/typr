# Design

## Context

- `screens/sidebar.rs` builds the column from flat `Item`s grouped by `Group` (Navigation, Decks, Views,
  Tags, Utilities). The deck group is drawn with the app's `ui::tree` wrapper over `nsh_base::tree`. The tree
  folds, moves with the arrow keys and tells a screen reader the depth, and its `render_item` draws each row.
  Every other group is drawn as plain `list_row`s with a due `badge`.
- `shell.rs` routes a press by key. `screen_for` maps `settings` and `diagnostics` to screens, and every
  other key filters the grid through `library::cards_in`, which only understands `deck:<id>`. `"all"` is
  hard-coded as the default selection and as the Escape target (four places).
- `paths::open_from_environment` creates a deck named `Default` when `snapshot.decks` is empty.
- `CardSummary` already carries `prompts`, `unscheduled`, `due_at`, `archived` and `trashed_at`, and
  `sidebar::due_counts` already walks the deck chain to skip archived decks. The Inbox can therefore be
  computed from the snapshot with no store query.
- The store already has `CreateDeck`, `RestoreCard` and `EmptyTrash`, and the import dialog and `Toasts`
  already exist in `shell.rs`. Settings already offers Import and Export.
- The app has no icons. `nsh_core` provides `svg(...)` and `ImageSrc::svg_markup`.

## Goals / Non-Goals

**Goals:** match the reference layout with the existing sidebar, tree and row components; derive every new
count and filter from the snapshot already in memory; add no dependency and no schema change.

**Non-Goals:** a per-deck "kind" or icon setting stored in the database; a navigation history (Escape still
returns to one fixed row); restyling the deck tree's rows beyond adding the icon and a chevron.

## Decisions

1. **The Inbox lives in the navigation group and is drawn with the same tree element as the decks.** The
   navigation group becomes a second `ui::tree` with items `dashboard` (disabled via `TreeItem::disabled`),
   and `inbox` with children `inbox:due` and `inbox:new`. Folding, keyboard movement and depth come free,
   and `disabled` gives Dashboard the base element's "dimmed" announcement. *Alternative:* add a fold
   control to `list_row`. Rejected because the tree wrapper notes that a second fold control toggles twice.
   The navigation tree's fold state uses the same `collapsed` signal as the decks, keyed by row key.

2. **Counts and filters are one function.** Add an `inbox` helper next to `due_counts` in `sidebar.rs` that
   returns the card ids for due and for new at `now_ms`, using the same studyable rule as `due_counts`. That
   rule covers archived and trashed cards and archived ancestor decks, and the helper also requires
   `prompts > 0`. Due means `due_at <= now`; new means `unscheduled > 0`; the Inbox is their union.
   `cards_in` takes `now_ms` and gains the three `inbox*` selections by calling the same helper, so a row's
   count and the grid it opens cannot disagree. *Alternative:* store queries (`DuePrompts`/`NewPrompts`).
   Rejected because they work per prompt, are capped by the new-card limit and shuffled, and the sidebar
   needs per-card counts on every refresh. "Due today" therefore means due now, the same instant the deck
   counts use, not the end of the study day.

3. **Default selection and Escape move from `all` to `inbox`.** All four `"all"` sites in `shell.rs` and
   `Sidebar::new`'s "first item" rule now point at the Inbox, so the first item is no longer the default.
   With no "All cards" row, the Inbox is the place the user returns to. The whole library is still reachable
   by selecting a deck's parent or by searching.

4. **Icons are inline SVG strings in one `ui::icon` module.** It holds a small fixed set of outline glyphs
   (search, trend, inbox, check-circle, dashed-circle, book, archive-box, plus, download, gear, trash,
   chevron), drawn with `svg(ImageSrc::svg_markup(..))`, tinted with the row's text token and sized from
   `ui::size`. The icon has no accessibility label. The deck icon is chosen by name: `Notes` gets
   archive-box, and every other deck gets book.
   `shortcut: the icon follows the deck's name; if users want to choose icons, store a deck kind then.`
   *Alternative:* a deck `kind` column. Rejected for now because it needs a schema change for one glyph.
   Lucide is the reference style, and paths are copied under its ISC licence, credited in the module doc.

5. **Utilities are action keys, not selections.** `import` runs the existing `import` closure and leaves the
   selection alone (the sidebar's `pick` skips `state.select` for it). `settings` and `trash` map to
   screens in `screen_for`. `Screen::Trash` joins `Screen` and its `PartialEq`, and the Escape handler
   treats it like Settings.

6. **`New Deck` is an inline field in the decks group, after the tree.** A `Sidebar` signal `naming: bool`
   switches the row between a `list_row` and the app's `ui::input` field. Enter trims the text; if it is
   non-blank, Enter applies `CreateDeck { parent: None, name }` through the shell's `db`, reloads the
   snapshot and selects `deck:<id>`. Escape or a blank name clears `naming`. The sidebar takes an
   `on_new_deck(String)` callback, so the store write stays in the shell (the shell spec's one-writer rule).

7. **The Trash screen is a new `screens/trash.rs`** built from `ui::rows` and `ui::button`. It reads trashed
   cards from the snapshot (`trashed_at.is_some()`, sorted by `trashed_at` descending) and shows each one's
   preview, its deck name and a Restore button (`RestoreCard`). Empty trash swaps its button for an inline
   "Destroy N cards? Cancel / Empty" pair, because the viewer has no modal confirm. Confirming applies
   `EmptyTrash`. Each write is followed by the shell's existing reload. The screen is added to
   `screens::NAMES` so the light/dark appearance test covers it.

8. **First open makes two decks.** In `paths.rs` the `Default` command becomes `Flashcards` then `Notes`,
   under the same `decks.is_empty()` guard. That guard counts archived decks too, so a library whose decks
   are all archived is still left alone.

9. **Diagnostics moves to a Settings row** whose button sets `Screen::Diagnostics`. Settings takes an
   `on_diagnostics` callback, like its existing `on_import`/`on_export`.

## Risks / Trade-offs

- [Existing libraries keep `Default` and lose the "All cards" row] → Expected per the decision to change only
  new libraries. Selecting a deck, the Inbox or searching covers the old use. Noted in the proposal as
  **BREAKING (UI)**.
- [Saved views and tags become unreachable from the UI] → They stay in the store and in exports. A later
  change can bring them back, for example through the command palette. The proposal lists this as out of scope.
- [Inbox counts go stale while the app sits open] → Same as today's deck counts, which refresh on every
  snapshot reload. No timer is added.
- [The navigation tree and the deck tree are two focus stops] → This matches two groups. The arrow keys move
  within each group, and Tab moves between them.
- [A user deck named `Notes` gets the archive icon] → Harmless, and covered by the shortcut note in Decision 4.
