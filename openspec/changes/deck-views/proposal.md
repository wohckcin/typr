# Proposal — deck views

## Why

A deck has exactly one shape in this app: a list of rows. Mochi offers four, and its shortcut panel names
them — grid, list, notebook and column — each with its own key set. That is not decoration: the same cards
are read differently when you are scanning a wall, editing one after another, or working through a long
card's contents.

What was measured on the running app: the deck header carries a `Grid View` tab, the deck page's keys are
`N` add, `R` review, `L` learn, list and notebook views add `J`/`K` selection with `Ctrl+J`/`Ctrl+K` to
reorder, `E` or `Enter` to edit, `Space` to preview and `D` to delete, and the notebook view adds `O` and
`Shift+O` to add a card below or above the selection, while the column view adds `Cmd+F` to find inside the
selected card.

## What Changes

- Four deck views: grid (cards as tiles), list (one line per card), notebook (full card contents stacked),
  column (card contents in a wide single column with in-card find).
- A deck remembers its view, and the choice is persisted with the library rather than with the window.
- One selection model shared by every view, with the movement keys bound once.
- Notebook view gains add-above and add-below at the selection.

## Capabilities

### New Capabilities

- `deck-views`: which views exist, what each shows, how selection moves, and how the choice survives a restart.

### Modified Capabilities

None. `card-library` still governs which cards a deck lists; this change governs how they are laid out.

## Impact

- Migration V12 adds `deck.view` (nullable text, `grid` when null), read by `Snapshot`; it takes the
  number after `fsrs-scheduling`'s V11, which is the last migration the roadmap chain lands.
- `crates/typr/src/library.rs` gains a view enum and one builder per view; `library/tile.rs` is reused by the
  grid; a new `library/notebook.rs` renders full card contents.
- `crates/typr/src/store/db.rs` and `store/mod.rs` gain `Command::SetDeckView`.
- `crates/typr/src/tests.rs` gains one test per view and a persistence test.
- `keymap-panel` owns the bindings themselves; this change declares the actions and asks that change for keys.

## Dependencies

Depends on `card-grid` for the tile the grid view draws, and on `keymap-panel` for the bindings it declares.
Implement order: `card-grid`, then `keymap-panel`, then this change.
