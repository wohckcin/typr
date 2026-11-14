# Tasks — cramming

Depends on `custom-views` for `ViewSpec`.

## 1. Mode

- [x] 1.1 `Mode { New, Due, Cram }` on `Session`; `Session::new` keeps working for the two existing queues and a new `Session::cram(entries)` starts one.
  - **Verification**: `cargo test -p typr-core --lib session` gains `a_cram_session_starts_with_every_entry_queued` and every existing queue test passes unchanged.
- [x] 1.2 `State::submit_grade` branches once: in cram it applies the grade to the session and sends nothing.
  - **Verification**: `a_cram_grade_writes_nothing` asserts the library's prompt rows and `review_log` are byte-identical before and after three answers, and `crates/typr-core/src/store/tests/review.rs` publishes no event.

## 2. Selecting the cards

- [x] 2.1 `Db::cards_for_view(&ViewSpec) -> Result<Vec<CardSummary>, StoreError>` reuses the snapshot filter and returns studyable cards only in `Cram`, ignoring due dates and the cap.
  - **Verification**: `a_cram_selection_matches_the_list` compares the query with `Snapshot::cards_matching` for four specs; `a_cram_selection_ignores_due_dates_and_the_cap` schedules everything into next week and still returns them.
- [x] 2.2 Repeating: forgot requeues to the end, remembered removes, and the session finishes only when empty.
  - **Verification**: `one_card_three_passes` mirrors the spec scenario, and `a_cram_session_with_nothing_selected_does_not_open`.

## 3. Screen

- [ ] 3.1 The library offers "Cram" beside the two queues, using the current filter; an empty filter refuses with a message.
  - **Verification**: `cramming_a_tag_with_nothing_due_visits_exactly_its_cards` drives the key path and reads the rendered card.
- [ ] 3.2 The session screen names the mode and says the answers are not scheduled; the summary reports the count and offers "Save as view".
  - **Verification**: `a_cram_review_says_nothing_is_scheduled` and `saving_a_cram_session_keeps_its_filter`.
- [ ] 3.3 "Save as view" creates the view through `CreateView` with the session's own spec, once.
  - **Verification**: `the_summary_saves_the_drill_once` presses twice and finds one view.

## 4. Close out

- [ ] 4.1 A scene `session-cram` shows a cram session; README documents cramming and its lack of writes.
  - **Verification**: `--list-scenes` lists it and its `--snap` is non-blank.
- [ ] 4.2 Full gate: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::cast_possible_truncation`, `cargo test --workspace --locked`, every scene snapping.
  - **Verification**: all three report success.

## Workflow follow-up

- Archive after `custom-views`.
- Next roadmap row: `card-names-and-references`.
