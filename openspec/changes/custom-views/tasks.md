# Tasks — custom views

Depends on nothing in flight; it can be implemented before the three changes ahead of it in the roadmap
if the user prefers.

## 1. Filter type

- [x] 1.1 New `ViewSpec`, `Sort` and `SortDir` in `crates/typr-core/src/store/view.rs`; `Snapshot::cards_matching(&self, spec: &ViewSpec) -> Vec<&CardSummary>`; `Sort::order(&self, cards, spec)` for the transient shuffle.
  - **Verification**: `cargo test -p typr-core --lib view` sorts by each key in both directions with cards that have no prompt last, and proves the shuffle only reorders.
- [x] 1.2 Migration V8: `card.pinned INTEGER NOT NULL DEFAULT 0` and `view(id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE COLLATE NOCASE, deck INTEGER, query TEXT NOT NULL, tags TEXT NOT NULL, mode TEXT NOT NULL, pinned_only INTEGER NOT NULL, sort_key TEXT NOT NULL, sort_dir TEXT NOT NULL)`.
  - **Verification**: `a_library_from_before_views_keeps_its_list` rewinds a real file to `user_version = 7` and reopens; `opening_reports_what_the_schema_did` counts the step.

## 2. Store

- [x] 2.1 Commands `CreateView { spec: ViewSpec, name: String }`, `RenameView { id, name }`, `DeleteView { id }`, `UpdateView { id, spec }`, `SetPinned { card, pinned }`; `Snapshot` gains `views: Vec<SavedView>`; a duplicate name is `StoreError::DuplicateName`.
  - **Verification**: `a_view_round_trips_through_the_store` and `two_views_cannot_share_a_name`.
- [ ] 2.2 A view whose deck has been deleted reports a missing scope and applies the whole library.
  - **Verification**: `a_view_whose_deck_is_gone_falls_back_to_every_card`.

## 3. Library

- [ ] 3.1 The sidebar lists views under the decks with rename and delete; selecting one sets the deck tree selection to none and applies the spec.
  - **Verification**: `selecting_a_view_applies_every_part_of_it` and `a_view_can_be_renamed_and_deleted`.
- [ ] 3.2 A sort control offers the six orders; a shuffle button is separate and does not save.
  - **Verification**: `the_list_is_ordered_by_the_chosen_key` and `a_shuffle_is_not_saved_with_the_view`.
- [ ] 3.3 The card row carries a pin toggle and the list shows pinned cards first when the view asks for only pinned.
  - **Verification**: `a_pinned_card_is_listed_without_a_deck` and `a_pin_survives_the_trash`.

## 4. Close out

- [ ] 4.1 A scene `library-views` seeds three views and one pinned card; README documents views and orders.
  - **Verification**: `--list-scenes` lists it and its `--snap` is non-blank.
- [ ] 4.2 Full gate: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::cast_possible_truncation`, `cargo test --workspace --locked`, every scene snapping.
  - **Verification**: all three report success.

## Workflow follow-up

- Archive after the changes before it.
- Next roadmap row: `cramming`, which reuses `ViewSpec` to build an ad-hoc session.
