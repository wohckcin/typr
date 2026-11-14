# Tasks — deck views

## 1. Storage

- [ ] 1.1 Migration V12 adds `deck.view TEXT NULL` and `card.position INTEGER NOT NULL DEFAULT 0` with a per-deck index. It follows `fsrs-scheduling`'s V11; a change applied out of the roadmap order must renumber and restate which version it follows. Verification: `a_library_rewound_from_v11_gains_both_columns` rewinds a fixture from V12 to V11 with `PRAGMA user_version = 11`, reopens it, and reads both.
- [ ] 1.2 Add `pub enum ViewMode { Grid, List, Notebook, Column }` to `crates/typr-core/src/library_view.rs` with `parse`/`as_str`, carried by `Deck` in `Snapshot`. Verification: `a_deck_without_a_view_reads_as_grid`.
- [ ] 1.3 Add `Command::SetDeckView { deck, view }` and `Command::MoveCardTo { card, position }`, the first writing one column, the second renumbering a deck in one transaction. Verification: `setting_a_view_leaves_the_schedule_alone` and `moving_a_card_renumbers_its_deck`.

## 2. Views

- [ ] 2.1 Split `library::view` into `grid_view`, `list_view`, `notebook_view` and `column_view`, chosen by the deck's `ViewMode`; the list view keeps today's 44 px rows. Verification: `each_view_draws_the_same_cards`.
- [ ] 2.2 Notebook view draws full card contents with an edit affordance per card, keyed `notebook-{id}`. Verification: `the_notebook_view_draws_a_whole_card` asserts the second side appears in the view and the first does not in the grid.
- [ ] 2.3 Column view draws one card's contents at reading width with in-card find (next and previous match, and a no-match state). Verification: `the_column_view_finds_the_second_match`.
- [ ] 2.4 Add the view switcher to the deck header, keyed `view-grid`, `view-list`, `view-notebook`, `view-column`, calling `Host::set_view`. Verification: `switching_views_keeps_the_selection`.

## 3. Behaviour

- [ ] 3.1 Move the selection with the movement keys in every view, clamped at the ends, scrolling the selection into view. Verification: `the_selection_stops_at_the_first_and_last_card` and `moving_selection_scrolls_it_into_view`.
- [ ] 3.2 Route edit, preview and delete from the keyboard in every view, plus add-below and add-above in the notebook view. Verification: `every_view_edits_previews_and_deletes` and `the_notebook_view_adds_a_card_below`.
- [ ] 3.3 Reorder the selected card with the move keys through `Command::MoveCardTo`. Verification: `moving_a_card_up_survives_a_restart`.
- [ ] 3.4 Persist and restore the view per deck. Verification: `a_decks_view_survives_a_restart` and `one_decks_view_leaves_another_alone`.

## 4. Verification

- [ ] 4.1 Register scenes `deck-list`, `deck-notebook`, `deck-column` and confirm each snaps non-empty and differs from `deck-grid`. Verification: four `--snap` runs plus a size comparison.
- [ ] 4.2 Run the gate: fmt, clippy with `-D warnings -W clippy::cast_possible_truncation`, `cargo test --workspace --locked`, every scene. Verification: all four report success.

## Workflow follow-up

- Requires `card-grid` and `keymap-panel` first. Archive with `openspec archive deck-views --yes` after the gate.
