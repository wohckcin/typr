# Design — deck views

## Context

`crates/typr/src/library.rs` builds one list: a virtualised stack of 44 px rows. `Library` already holds
`mode` (active/archived/trash), `selected_deck`, `selected_card`, `query`, `tag` and `visible`. `Snapshot`
carries `decks` and `cards`. There is no place today to store "this deck is read in the notebook view".

Mochi's shortcut panel, read on the running app (2026-11-14), names four deck-page views with distinct key
sets: grid, list, notebook and column. The grid view is the only one that was measured in detail (see
`card-grid`); the others were identified from the panel's own headings and key lists.

## Goals

- One selection model, four presentations, no duplicated key dispatch.
- The view is a property of the deck, stored with the library, so two decks can be read two ways.
- Each view reuses an existing builder: tiles from `card-grid`, rows from today's list.

## Non-Goals

- No drag-and-drop reordering; the move keys are the only reorder affordance.
- No per-deck column widths, no custom view definitions (`custom-views` owns saved filters).
- No card content edited in place: the notebook view edits through the editor.

## Decisions

### The view is a column on `deck`, not a setting

`settings` holds library-wide values; a view is per deck, so migration V12 adds `deck.view TEXT NULL` with
`NULL` meaning `grid`, and `Snapshot::decks` carries it. A single global `deck_view` setting was rejected: the
whole point of four views is that a deck of long cards wants notebook while a deck of vocabulary wants list.

### One enum decides both the builder and the available keys

`pub enum ViewMode { Grid, List, Notebook, Column }` in `crates/typr-core/src/library_view.rs`, carried in
`Snapshot`, and `fn view<S: Host>(library: &Library) -> El` matches on it. The key set is not duplicated per
view; the dispatch asks `keymap-panel`'s table for the surface `<screen, ViewMode>`.

### Selection is an index into `visible`, not a card id

`selected_card: Option<Id>` stays because other screens use it, but movement works on the index of that card in
`Library::visible`, so a filtered deck moves within what is shown and cannot select a card the filter hides.
A stored index instead of an id was rejected: the deck changes underneath it whenever a card is graded.

### Notebook edits open the editor; nothing is edited in place

Inline editing in four views means four cursors, four draft paths and four ways to lose a keystroke. The
notebook view is a reading layout plus one key: edit opens the editor, which already owns drafts and the
unsaved-changes dialog.

### The column view's find reuses the existing search folds

`folded` from `crates/typr-core/src/search` is already the library's matcher; find inside a card searches the
selected card's `markdown` with the same fold, so accents and case behave as search does. An independent matcher
was rejected as a second truth.

### Reorder is one command, applied to the deck's order

`Command::MoveCardTo { card, position }` writes the card's `position` column and renumbers the deck's cards in
one transaction. Sorting by `updated_at` was rejected: a deck's order is the user's, not a timestamp's.

## Risks / Trade-offs

- **Five surfaces to keep in step.** Mitigated by one builder per view behind one dispatch, and by a test that
  every view offers edit, preview and delete.
- **Migration V12 touches every deck row.** It is nullable and read-only, so no data is rewritten. The
  number is V12 because the chain lands V4 cloze, V5 reverse, V6 diagram, V7 templates, V8 custom views,
  V9 and V10 names and references, and V11 FSRS before this one; a change that hard-codes a number must
  name the version it follows, or two changes will claim the same step.
- **The column view's find is scoped to one card**, which is weaker than library search; that is what the
  measured app does.

## What I could not measure

- The notebook and column layouts' real geometry: only their existence and key sets come from the running app,
  not their spacing, widths or breakpoints. Those are stated as intent here and should be settled against the
  app before implementation rather than invented in code.

## Migration Plan

V7 adds `deck.view`; `Command::SetDeckView` writes it; `Snapshot` reads it with `COALESCE(view, 'grid')`.
Forward-only, no downgrade path, consistent with `project-structure`.

## Open Questions

- Whether the notebook view shows one card per line at full width or two columns at wide windows.
