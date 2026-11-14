# Proposal: sidebar as the app's navigator

## Why

The exploration of https://app.mochi.cards/ measured a sidebar that carries far more
than a deck list: an account state row, a search row with its chord, a grouped deck
area where a deck states how much work it holds, and a utility group. typr has a
sidebar (`fn deck_sidebar` in `crates/typr/src/library.rs`), but it is a bare
indented deck list 200 points wide: no grouping, no counts, no icon column, no
active treatment, no collapse, and `Import`/`Settings`/`Trash` have no place in it
at all.

## What Changes

- The sidebar becomes its own module with three groups: navigation (`Dashboard`,
  `Inbox`), decks (with `New Deck`), and utilities (`Import`, `Settings`, `Trash`).
- Every row states what it is with an icon column, and every deck row states how
  much work it holds with a right-aligned count of prompts due in its subtree.
- A deck row that holds cards can be collapsed, hiding its subtree, and a collapsed
  deck stays collapsed while the app runs.
- The current deck is marked by a filled row rather than by colour alone.
- The search row shows the chord that focuses it, taken from the keymap table
  rather than written twice.
- A section whose content is empty does not render (a tag section appears only once
  tags exist).
- The account/state row states the library's state, in a tone that carries the
  meaning (a closed library reads in the danger tone, not grey).

## New Capabilities

- `sidebar`: the navigator column, its groups, its rows and their states.

## Modified Capabilities

None.

## Impact

- `crates/typr/src/sidebar.rs` (new), `crates/typr/src/library.rs` (the deck list
  moves out), `crates/typr/src/main.rs` (the library screen composes the module),
  `crates/typr-core/src/store/{db,mod,task}.rs` (a due count per deck),
  `crates/typr/src/tests.rs`, and a scene per sidebar state.
- No schema change, no behaviour change to scheduling.
