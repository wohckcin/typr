# Tasks — reverse reviews

## 1. Schema and prompts

- [ ] 1.1 Migration V5: add `deck.reverse INTEGER` and `card.reverse INTEGER` (NULL inherits), insert `reverse_default` into `settings` with value `0`, and recreate `studyable_prompt` with the `deck_reverse` recursive CTE and the `p.key <> 'rev'` filter.
  - **Verification**: `a_library_from_before_reverse_gains_the_columns` rewinds a real file to `user_version = 4` and reopens it; `opening_reports_what_the_schema_did` counts the new step; `a_reverse_prompt_queues_only_where_reverse_is_on` covers card-on, deck-on, library-on and all-off.
- [ ] 1.2 `sync_prompt(card, sides)` inserts `rev` beside `fwd` for a card with two or more sides, still `ON CONFLICT DO NOTHING`.
  - **Verification**: `a_two_sided_card_holds_both_directions` sees keys `fwd` and `rev`; `a_note_holds_no_reverse_prompt` sees neither.
- [ ] 1.3 Commands `SetCardReverse { card, reverse: Option<bool> }`, `SetDeckReverse { deck, reverse: Option<bool> }` and `SetSetting { key: "reverse_default", … }` write the three levels.
  - **Verification**: `a_card_setting_beats_its_deck` and `a_deck_setting_reaches_nested_decks` create the tree, flip flags and read the queue back.

## 2. Scheduling independence

- [ ] 2.1 Nothing in `grade`, `reset_history`, `advance_clock` or the due ordering keys on anything but `(card, key)`, so independence is inherited; pin it with tests rather than new code.
  - **Verification**: `the_two_directions_keep_separate_schedules` grades `fwd` remembered and `rev` forgot in one library and asserts each prompt's own phase, interval and due day.
- [ ] 2.2 `Db::review_count` keeps counting non-learn records for the card across both prompts, and `History` lists which direction each record belongs to.
  - **Verification**: `history_names_the_direction_of_each_record` reads two records for one card and finds their keys in the rows.

## 3. Reviewing the reverse direction

- [ ] 3.1 `Session` builds `faces` from `sides`, reversed when `prompt` starts with `rev`, and exposes `direction() -> &str`.
  - **Verification**: `a_reverse_entry_shows_the_last_side_first` asserts the first face text and that one reveal surfaces the question; every existing sides test passes unchanged.
- [ ] 3.2 The session screen names the direction and the bindings row is unchanged.
  - **Verification**: `crates/typr/src/tests.rs::a_reverse_review_says_so` reads the rendered elements for "Reverse" on a `rev` entry and its absence on a `fwd` entry.

## 4. Controls

- [ ] 4.1 The library row shows a reverse marker and carries a toggle bound to `SetCardReverse`.
  - **Verification**: `the_row_toggles_reverse_for_one_card` turns it off and sees the `rev` prompt leave the due queue while a sibling card keeps its own.
- [ ] 4.2 A deck shows its three-state setting and offers on, off and inherit.
  - **Verification**: `a_deck_can_be_set_back_to_inherit` asserts the label changes and the queue follows the parent.
- [ ] 4.3 The editor header shows the card's own reverse state, including "inherit".
  - **Verification**: `the_editor_says_a_card_inherits_its_reverse_setting` opens a card with no own value and finds "Inherit".

## 5. Close out

- [ ] 5.1 Refresh the `library-5000` scene so it includes one reverse-enabled deck, and add `review-reverse` snapping a card asked from its answer side.
  - **Verification**: `--list-scenes` lists it and its `--snap` is non-blank.
- [ ] 5.2 Full gate: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::cast_possible_truncation`, `cargo test --workspace --locked`, every scene snapping.
  - **Verification**: all three report success.

## Workflow follow-up

- Archive this change after `cloze-deletions`, so the prompt-count wording lands in one order.
- Next roadmap row: `diagram-cards`.
