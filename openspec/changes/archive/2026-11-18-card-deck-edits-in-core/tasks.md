# Tasks

## 1. Backend rules (`typr_core::edits`)

- [x] 1.1 Add `crates/typr-core/src/edits.rs` (with `pub mod edits;` in `lib.rs`), containing
  `has_content` moved from `crates/typr/src/screens/editor/mod.rs`, `deck_of_key`,
  `deck_for_new_card`, `selection_after_trash` and `deck_name`. Verify with unit tests in the
  module for:
  - the selected open deck
  - a selected archived deck, which falls back
  - an archived first deck, which is skipped
  - no open deck, which gives `None`
  - a neighbour after a trash, the last card, and an empty list
  - a padded name, a blank name and a whitespace card
- [x] 1.2 Verify that `cargo tree -p typr-core -e normal` still lists no `nsh-*` crate.

## 2. Store checks

- [x] 2.1 Add `StoreError::BlankName { what }` and `StoreError::EmptyCard { card }` in
  `store/mod.rs`. Verify that `cargo build -p typr-core` passes and that every exhaustive match on
  `StoreError` is updated.
- [x] 2.2 In `store/db.rs`, make `CreateDeck` and `RenameDeck` use `edits::deck_name`: store the
  trimmed name and refuse a blank one. Verify with tests in `store/tests/cards.rs` that:
  - ` Spanish ` is stored as `Spanish`
  - `   ` is refused and the deck count is unchanged
  - ` Spanish` next to `Spanish` is refused as `DuplicateName`
  - a rename to blank is refused
- [x] 2.3 In `store/db.rs`, make `SaveCard` on a text card refuse empty text with `EmptyCard` (after
  the diagram and template checks) and delete the card's draft in the same dispatch. Verify with
  tests that:
  - a whitespace save is refused and the text is unchanged
  - a save leaves `Command::Draft` returning `None`
  - a refused save keeps the draft
- [x] 2.4 Verify with a test in `crates/typr-core/tests/transfer.rs` that an archive holding a card
  with empty text still imports.

## 3. Desktop crate calls the backend

- [x] 3.1 In `screens/editor/mod.rs`, replace the local `has_content` with
  `typr_core::edits::has_content`, and update its callers and any test imports. Verify that the
  editor tests pass (`cargo test -p typr --test editor`, or the file that holds them).
- [x] 3.2 In `screens/sidebar.rs`, gate the new-deck submit on `typr_core::edits::deck_name` instead
  of a local `trim`. Verify that the sidebar tests "Naming a deck creates it" and "A blank name
  creates nothing" pass.
- [x] 3.3 In `screens/library.rs`, make `cards_in` use `edits::deck_of_key`. Verify that the library
  tests pass.
- [x] 3.4 In `shell.rs`, rewrite `new_card` as `deck_for_new_card` plus `write(CreateCard, true)`,
  and `delete_card` as `write(TrashCard, true)` plus `selection_after_trash`. Remove their direct
  `db.apply` and `reopen()` calls. Verify that the shell tests for a new card and for trashing a
  neighbour pass, and add a shell test showing that a new card from the Inbox, with the first deck
  archived, opens in the second deck.
- [x] 3.5 In `shell.rs`, make `on_save` send only `SaveCard` and keep `on_discard`'s `SetDraft`.
  Verify that the editor's "saved card reopens clean" path passes in the shell tests.

## 4. Integration

- [x] 4.1 Verify that `cargo clippy --workspace --all-targets -- -D warnings` is clean and that
  `cargo test --workspace` passes.
- [x] 4.2 Verify with `rg 'trim\(\)|decks\s*\.first\(\)|SetDraft \{\s*card: id,\s*text: None' crates/typr/src`
  that no deck-name trim, first-deck fallback or post-save draft clear is left in the desktop crate,
  apart from `on_discard`.

## Workflow follow-up

- Archive the change with `openspec-archive-change` once it has been reviewed.
