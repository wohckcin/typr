# Tasks — hidden text

## 1. Parse hidden text

- [ ] 1.1 Add `Shape` and `shape(markdown)` to `crates/typr-core/src/hidden.rs`: `Sides(Vec<String>)` when there is no hidden text (delegating to `sides::split`), `Hidden { faces, groups }` otherwise, ignoring fences and inline code and leaving an unclosed brace as text.
  - **Verification**: `cargo test -p typr-core --lib hidden` covers one unnumbered span, two numbered groups, mixed spans, a span inside a fence, a span inside backticks, an unclosed brace, and a card with both a separator and a span (which reports one face and no sides).
- [ ] 1.2 Render the masked face: `fn masked<'a>(text: &'a str, shape: &Shape, key: &str) -> String` and `fn shown(...)`, replacing each span of the active group with `[ … ]` and leaving other groups visible.
  - **Verification**: a test asserts `cloze:1` masks only group 1, `cloze:0` masks every unnumbered span, and `shown` reproduces the original text byte for byte.

## 2. Store the shape

- [ ] 2.1 Migration V4 in `crates/typr-core/src/store/schema.rs`: add `card.has_cloze INTEGER NOT NULL DEFAULT 0`, backfill it from `markdown` containing `{{`, and recreate `studyable_prompt` by copying the body migration V3 left, verbatim, and widening only its last predicate to `AND (c.sides >= 2 OR c.has_cloze = 1)`.
  - **Verification**: `a_library_from_before_hidden_text_gains_the_column` opens a file rewound to `user_version = 3`, checks the column exists and the view reports the same rows; `opening_reports_what_the_schema_did` still counts the new step; `the_widened_view_still_hides_archived_and_trashed_cards` proves the archived, trashed and archived-ancestor clauses survive the recreation.
- [ ] 2.2 Replace `sync_prompt(card, sides)` with `sync_prompts(card, shape)`: insert `fwd` for two or more sides, `cloze:<n>` per numbered group and `cloze:0` for unnumbered spans, never delete, and set `has_cloze`. Call it from `CreateCard` and `SaveCard`.
  - **Verification**: `a_hidden_text_card_gets_one_prompt_per_group` creates a card with two groups and sees `cloze:1`, `cloze:2`; `a_card_that_stops_hiding_text_keeps_its_history` edits the spans away and finds the prompts still present with their records, while the card leaves the queue.
- [ ] 2.3 Keep the new prompt count honest in the queues: `new_prompts` and `due_prompts` already select from the view, so a one-face hidden-text card appears, and the cap still counts cards rather than prompts.
  - **Verification**: `a_one_line_hidden_text_card_is_in_the_new_queue` and `two_groups_of_one_card_count_once_against_the_cap`.

## 3. Review the mask

- [ ] 3.1 `Entry` becomes `{ card, prompt, faces: Vec<Face>, overdue }` with `Face { hidden, shown }`; `Session` reads `hidden` until revealed and `shown` after.
  - **Verification**: `crates/typr-core/src/session/tests.rs` keeps every existing sides test passing unchanged, and `a_hidden_text_face_unmasks_on_one_reveal` grades after a single reveal.
- [ ] 3.2 `State::session_entries` builds entries from the store's prompts and the card's shape, so a `cloze:<n>` prompt yields one face and a two-sided card yields two.
  - **Verification**: `crates/typr/src/tests.rs::a_hidden_text_prompt_shows_a_masked_sentence` opens a session on a cloze card, finds the mask in the rendered text, presses Space once and finds the answer with the grade buttons on screen.

## 4. Show it in the editor

- [ ] 4.1 The preview masks the card's first group and the sides caption becomes a prompt count: "2 prompts" for two groups, "1 prompt" for a single group, and no sides caption at all on such a card.
  - **Verification**: `the_preview_masks_hidden_text_and_counts_the_prompts` types a two-group card and reads the caption and the masked text out of the rendered elements.
- [ ] 4.2 A card holding hidden text renders one pane, not source-and-sides wording that claims two sides.
  - **Verification**: `a_hidden_text_card_is_not_described_as_two_sides` fails if the caption says "2 sides".

## 5. Close out

- [ ] 5.1 Document the syntax in `README.md` beside the separator, and add a scene `review-cloze` showing a masked card.
  - **Verification**: `cargo run -q -p typr -- --list-scenes` lists `review-cloze`, and its `--snap` is non-blank.
- [ ] 5.2 Full gate: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::cast_possible_truncation`, `cargo test --workspace --locked`, and every scene snapping.
  - **Verification**: all three commands report success, and the ten existing scenes still snap.

## Workflow follow-up

- Archive this change.
- Next roadmap row: `review-reverse`.
