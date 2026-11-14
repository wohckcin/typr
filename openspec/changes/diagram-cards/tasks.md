# Tasks — diagram cards

Depends on `cloze-deletions` for the idea of several prompts per card and for the face type this change
turns into an enum.

## 1. Store a diagram

- [x] 1.1 Migration V6: add `card.kind TEXT NOT NULL DEFAULT 'text'`, `card.image TEXT`, `card.caption TEXT`, create `mask(card_id INTEGER NOT NULL, group_no INTEGER NOT NULL, x REAL NOT NULL, y REAL NOT NULL, w REAL NOT NULL, h REAL NOT NULL, PRIMARY KEY (card_id, group_no, x, y, w, h))` with a foreign key and a `mask_group` index, and extend `studyable_prompt` by copying the body migration V5 left, verbatim, and adding `OR EXISTS (SELECT 1 FROM mask m WHERE m.card_id = c.id)` to its last predicate, keeping the `c.has_cloze` clause and the `rev` disjunct.
  - **Verification**: `a_library_from_before_diagrams_reads_as_text` rewinds a real file to `user_version = 5`, reopens, and finds every card `kind = 'text'` with its queue unchanged; `opening_reports_what_the_schema_did` counts the step. `a_one_sided_diagram_card_queues_while_a_one_sided_text_card_does_not` and `a_reverse_prompt_still_queues_only_where_reverse_is_on` prove the earlier clauses survive.
- [ ] 1.2 Commands `CreateDiagram { deck, image, caption }`, `SetDiagram { card, image, caption }`, `AddMask { card, group, rect }`, `SetMaskGroup { card, rect, group }`, `RemoveMask { card, rect }` and `MergeMasks { card, rects, group }`, plus `mask_groups(card) -> Vec<u32>` and `masks(card) -> Vec<MaskRow>` on the store trait.
  - **Verification**: `a_diagram_card_holds_the_masks_it_was_given` and `merging_two_rectangles_leaves_one_group` in `crates/typr-core/src/store/tests/cards.rs`.
- [ ] 1.3 `sync_prompts` inserts `mask:<n>` for each group of a diagram card and leaves text behaviour untouched.
  - **Verification**: `a_diagram_card_gets_one_prompt_per_group` and `a_diagram_without_masks_has_no_prompt`.

## 2. Review a masked image

- [ ] 2.1 `Face` becomes an enum with a text and an image variant; `Session` builds image faces from the mask group, counting one face per group.
  - **Verification**: `a_mask_group_is_one_face` grades after a single reveal; all sides and cloze tests still pass.
- [ ] 2.2 The review screen draws the image with the group's rectangles covered and uncovers them on reveal, showing the caption when present.
  - **Verification**: `crates/typr/src/tests.rs::a_diagram_review_covers_only_its_own_group` reads the rendered elements for one cover per rectangle of the reviewed group and none for the other; `a_diagram_review_shows_its_caption` finds the caption text.

## 3. Author a diagram

- [ ] 3.1 The editor offers "Diagram" for a card and copies the chosen image into the attachments folder through the native file dialog, reporting a copy failure on screen.
  - **Verification**: `choosing_an_image_copies_it_and_stamps_the_card` and `an_image_that_cannot_be_copied_leaves_the_card_alone`.
- [ ] 3.2 Dragging across the image adds a rectangle, stored as fractions of the measured pane, ignoring a drag smaller than one percent.
  - **Verification**: `a_drag_stores_a_fractional_rectangle` drives `rune::Runtime` pointer events across a 400-point pane and asserts the stored `w` is about 0.25; `a_click_adds_nothing` presses and releases in place.
- [ ] 3.3 Rectangles are assigned to groups, merged, and removed, with the group of each drawn on the image.
  - **Verification**: `merging_selects_one_group_for_both` and `removing_a_group_leaves_its_records` (the second reads history after removal).
- [ ] 3.4 Keyboard fallback: a key adds a rectangle at the centre and the arrow keys resize it before the group is chosen.
  - **Verification**: `a_mask_can_be_made_without_a_pointer`.

## 4. Close out

- [ ] 4.1 README gains the diagram card type, and a scene `review-diagram` renders a card with two masked groups.
  - **Verification**: `--list-scenes` lists it and its `--snap` is non-blank.
- [ ] 4.2 Full gate: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::cast_possible_truncation`, `cargo test --workspace --locked`, every scene snapping.
  - **Verification**: all three report success.

## Workflow follow-up

- Archive after `cloze-deletions` and `review-reverse`.
- Next roadmap row: `card-templates`.
