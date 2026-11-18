# Design

## Context

`typr_core::store::Db::apply` runs one `Command` in one transaction. The desktop crate holds the
rules around those commands; see proposal.md, Why.

| Rule | Where it lives today |
|---|---|
| Deck for a new card | `shell.rs` `new_card`: parses `deck:<id>`, falls back to `decks.first()` |
| Selection after a trash | `shell.rs` `delete_card`: index of the trashed card in `visible()` |
| Deck name acceptable | `sidebar.rs` `new_deck_row`: `name.trim()`, blank skipped |
| Card has content | `screens/editor/mod.rs` `has_content` |
| Save drops the draft | `shell.rs` `on_save`: `SaveCard`, then a second `SetDraft { text: None }` |

`new_card` and `delete_card` call `db.apply` directly, so they skip the shell's single writer: no
slow-command record, and no stoppage flag on refusal.

## Goals / Non-Goals

**Goals**
- Every rule in the table is a backend function or a store check, with a backend test.
- The store refuses what the UI would refuse, whichever caller asks.

**Non-Goals**
- Moving the store writer (timing, slow log, re-publish, `stopped`) into the backend. It feeds UI
  signals and has no rule in it.
- Session queue building (`start_session`, `entries_for`). That is study, not card or deck editing.
- Deck rename, archive or delete UI.
- Cleaning up deck names already stored untrimmed.
- Changing `write` so that a refused deck name doesn't set `stopped`. The sidebar avoids sending a
  blank name instead; see Decisions.

## Decisions

### Pure rules in a new `typr_core::edits` module, plus store checks

These functions take plain data and return an answer:

- `has_content(markdown) -> bool`, moved from the editor.
- `deck_of_key(key) -> Option<Id>`, which parses `deck:<id>`. `screens/library.rs` `cards_in` uses
  it as well.
- `deck_for_new_card(snapshot, selected_key) -> Option<Id>`.
- `selection_after_trash(before, trashed, after) -> Option<Id>`.
- `deck_name(raw) -> Option<&str>`, which trims and returns `None` when the name is blank.

The store uses the same functions inside `dispatch`:

- `CreateDeck` and `RenameDeck` pass the name through `deck_name`. A blank name returns the new
  `StoreError::BlankName { what: "deck" }`. The trimmed name is what `named_write` inserts, so the
  unique index judges the trimmed name.
- `SaveCard` on a text card calls `has_content`. When there is none it returns the new
  `StoreError::EmptyCard { card }`. This check comes after the existing diagram and template checks,
  which keep their own errors.
- `SaveCard` deletes the card's `editor_draft` row after the update. Both run inside the
  transaction `apply` opens, so a refused save rolls back and the draft survives.

*Alternative considered:* a `Library` service in the backend that wraps `Db` and exposes
`new_card()` and `trash_card()`. It was rejected because it would either duplicate the shell's
writer or pull UI signals into the backend. Pure functions keep the single writer and test without
a database.

### The UI asks the rule before it writes, where a refusal would flag a stoppage

`write` sets `stopped` on any store error. The sidebar and the editor therefore call
`edits::deck_name` and `edits::has_content` before they send a write, as they do today with their
local copies. The store check is the guarantee for other callers, not the UI's normal path.

### `new_card` and `delete_card` go through `write`

Both closures become a call to the rule followed by `write(command, true)`. They pick up the slow
log and the stoppage flag, and the separate `reopen()` they used for the re-publish goes away. The
selection after a trash comes from `selection_after_trash(before, id, visible())`.

### `on_save` sends one command

The second `SetDraft { text: None }` write after a save is removed. `on_discard` keeps its own
`SetDraft { text: None }`, because discarding is not saving.

## Risks / Trade-offs

- [`SaveCard` now refuses whitespace for every caller] → Import goes through
  `ImportArchive`/`ImportMarkdown`, not `SaveCard`, so it is unaffected. A test confirms an import
  holding an empty card still opens.
- [Removing `reopen()` from `new_card` or `delete_card` could make the grid stale] → `write(…, true)`
  re-publishes the snapshot. The existing headless tests "a new card opens the editor" and
  "trashing selects a neighbour" cover it.
- [A front end other than this app sending `CreateDeck` with a blank name now gets an error instead
  of a blank deck] → Intended; the spec says so.
