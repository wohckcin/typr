# Tasks

## 1. Starter decks

- [x] 1.1 In `crates/typr/src/paths.rs`, replace the `Default` deck with `Flashcards` then `Notes` under the existing `decks.is_empty()` guard; add tests in `crates/typr/tests/startup.rs` that a fresh `TYPR_DATA_DIR` opens with exactly those two top-level decks and that a library already holding `Default` gains nothing (`cargo test -p typr --test startup` passes)

## 2. Inbox selection

- [x] 2.1 Add the inbox helper beside `due_counts` in `screens/sidebar.rs` (due ids, new ids, using the studyable rule and `prompts > 0`); unit-test due/new/both/note/trashed/archived-deck cases (`cargo test -p typr --test sidebar` passes)
- [x] 2.2 Teach `library::cards_in` the `inbox`, `inbox:due` and `inbox:new` selections via that helper, threading `now_ms` through its callers in `shell.rs`; extend `crates/typr/tests/library.rs` so each selection returns exactly the expected cards and matches the helper's counts

## 3. Icons

- [x] 3.1 Add `ui/icon.rs` with the fixed glyph set from design Decision 4 (inline SVG, ISC credit in the module doc, no accessibility label), tinted by the caller's token; add a gallery/appearance check that every glyph renders in light and dark (`cargo test -p typr --test appearance` passes)

## 4. Sidebar layout

- [x] 4.1 Restructure `Group`/`items_from`: remove Views, Tags, `all` and `diagnostics`; add `dashboard` (disabled), `inbox` with `inbox:due`/`inbox:new` children carrying their counts, and utilities `import`, `settings`, `trash`; update `crates/typr/tests/sidebar.rs` to assert the row order Dashboard → Trash for a new library and that no view, tag, All cards or Diagnostics row appears
- [x] 4.2 Draw the navigation group with `ui::tree` (sharing the `collapsed` state) and render a leading icon plus right-aligned count in both trees' `render_item`; utility rows get icons via `list_row`; test that folding Inbox hides its children but keeps its count, and that pressing Dashboard changes neither selection nor screen
- [x] 4.3 Restyle the search row as a rounded field with the search icon, `Search` and the keymap chord; pressing it focuses the library search; update the sidebar test that reads the chord from the keymap and add one that a press focuses search
- [x] 4.4 Make the Inbox the default selection: `Sidebar::new` selects `inbox`, and the four `"all"` sites in `shell.rs` (initial selection and Escape) use `inbox`; update `crates/typr/tests/shell.rs` so launch and Escape land on the Inbox
- [x] 4.5 Update `crates/typr/tests/a11y.rs` so a screen reader hears each row's label once with no icon name, and hears Dashboard as unavailable/coming later (`cargo test -p typr --test a11y` passes)

## 5. New deck row

- [x] 5.1 Add the `naming` state and the `New Deck` row/field to the decks group, with an `on_new_deck(String)` callback; in `shell.rs`, apply `CreateDeck { parent: None, name }`, reload and select `deck:<id>`; disable the row when the library is closed; test Enter with ` Spanish ` creates and selects `Spanish`, and Escape and blank Enter create nothing

## 6. Utility actions and Trash screen

- [x] 6.1 Route `import` to the existing import closure without changing the selection, and add `Screen::Trash` to `Screen`, its `PartialEq`, `screen_for` and the Escape handler; test in `shell.rs` that Import leaves the selection unchanged and that the Trash row opens the trash screen
- [x] 6.2 Build `screens/trash.rs`: trashed cards newest first with preview, deck name and Restore (`RestoreCard`), an empty state, and Empty trash with an inline count-naming confirm that applies `EmptyTrash`; register it in `screens::NAMES`; add `crates/typr/tests/trash.rs` covering ordering, restore, the empty state, confirm and cancel

## 7. Settings link to Diagnostics

- [x] 7.1 Add an `on_diagnostics` callback and a Diagnostics row to `screens/settings.rs`, wired in `shell.rs` to `Screen::Diagnostics`; test in `crates/typr/tests/settings.rs` that pressing it shows the diagnostics screen

## 8. Integration

- [x] 8.1 Run `cargo test --workspace` and `cargo clippy --workspace -- -D warnings`, and confirm both are clean
- [x] 8.2 Use the `run-typr` skill to screenshot a fresh library (`TYPR_DATA_DIR` pointed at an empty scratch folder) in light and dark, and compare the sidebar with the reference screenshot: rows, order, icons, counts and the Decks heading

## Workflow follow-up

- Archive the change with `openspec archive sidebar-inbox-and-default-decks` once it has been reviewed.
