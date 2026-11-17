# Tasks — library-starts-with-a-deck

## 1. The deck a new library starts with

- [x] 1.1 Seed one deck in `open_from_environment` when `db.snapshot()?.decks.is_empty()`, named `Default`,
      after the database opens and before the schema facts are logged.
      Verification: `crates/typr/tests/startup.rs` case `a_library_that_does_not_exist_starts_with_a_deck`
      opens a folder that does not exist, then the same folder again, and finds one deck both times.

- [x] 1.2 Leave a library that holds a deck alone.
      Verification: the same case asserts the second open is still one deck, so the seed does not repeat.

## 2. The empty deck's action

- [x] 2.1 Hand the shell's new-card closure to the grid at both call sites (`shell.rs`, the library and the
      missing-card fallback) instead of `|| {}`.
      Verification: `crates/typr/tests/shell.rs` case
      `an_empty_deck_offers_a_new_card_that_opens_its_editor` presses the button the empty state draws, sees the
      editor, and reads one card out of the library file.

- [x] 2.2 Check the button on the running app, not only headlessly: a fresh `TYPR_DATA_DIR`, the sidebar's deck,
      the empty grid, and the editor that follows the press.
      Verification: the window showed `Default`, "This deck has no cards yet", the button "New card", and the
      editor with "Card source" and "Attach image…" after the press.

## 3. Gates

- [x] 3.1 `cargo test --workspace` once, at the end.
      Verification: 519 passed, 0 failed.
