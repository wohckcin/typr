# Tasks — card grid

## 1. The tile

- [x] 1.1 Add `crates/typr/src/library/tile.rs` with `pub fn tile<S: Host>(card: &CardSummary, width: f32) -> El` drawing the surface, the rendered first side and the footer strip. Verification: `cargo test -p typr --lib library::tile` passes `a_tile_draws_the_first_side_only`.
- [x] 1.1a Draw the strip and the footer to the measured anatomy: body 96 above a footer 26, the hidden-side
      label a full-width 22-point strip at the bottom of the body (9px/600, uppercase, 30% white,
      `rgba(0,0,0,0.1)`), the footer `rgba(0,0,0,0.09)` with `0 0 8px 8px` radius holding a 13px sentence-case
      word and a 10px/500 uppercase kicker.
      *Verification*: at a 997-point window a tile is 245 wide and at 700 it is 227, each exactly its share, the
      strip measures 22 and the footer 26; a
      rendered tile's strings read `New`, `0 days`, `1 hidden side`, with case as drawn.

- [x] 1.1b Draw tagged and untagged cards at their own heights and let a line take the tallest, rather than
      reserving space for a tag strip that is not there.
      *Verification*: a line holding one tagged and one untagged card reports a row height equal to the
      tallest tile, and the untagged tile stays shorter and draws no chip.

- [x] 1.1c Draw hidden text masked on a tile exactly as a review face draws it, and fail if the answer text
      reaches the tile.
      *Verification*: a card whose first side holds `{{insulin}}` renders a masked span and the tile's strings
      never contain `insulin`.

- [x] 1.2 Add `fn hidden_sides(card: &CardSummary) -> usize` (sides minus one, floored at zero) and render the badge only above zero. Verification: `a_tile_states_the_sides_it_hides` and `a_note_carries_no_hidden_side_badge`.
- [x] 1.0 Widen the snapshot: `CardSummary` gains `prompts: u32`, `unscheduled: u32` and `due_at: Option<i64>`, filled by the card query's left join on `studyable_prompt` (see design.md). No schema change. Verification: `a_card_with_one_due_prompt_snapshots_its_due_date` and `a_note_snapshots_no_prompts`; the existing snapshot tests still pass.
- [x] 1.4 Draw tag chips with `fn chips(tags: &[String], width: f32) -> (Vec<&String>, usize)` and the `+N` overflow. Verification: `the_chips_count_what_they_drop`.
- [x] 1.5 Draw the dashed placeholder tile for an empty deck, keyed `new-card`, wired to `Host::new_card`. Verification: `an_empty_deck_offers_a_tile_to_fill` finds the key and pressing it starts a draft.

## 2. The grid

- [x] 2.1 Add `fn columns_for(width: f32) -> usize` and `fn tile_rows(cards: &[CardSummary], columns: usize) -> usize` with the constants in one place. Verification: `two_tiles_fit_at_482_points` and `one_tile_fits_when_nothing_else_does`.
- [x] 2.2 Replace the row stack with a tile-row stack: `col().key("rows").growth(..).on_layout(..)` storing the width, then `virtual_rows(tile_rows(..), TILE_HEIGHT + GAP, ..)`. Verification: `a_long_deck_builds_only_the_tile_rows_on_screen` counts built tiles against `columns × visible_rows`.
- [x] 2.3 Keep selection, press and the actions menu working on a tile, and keep `Library::visible` the single source of order. Verification: `pressing_a_tile_opens_its_card` and `the_deck_actions_still_run_from_a_tile`.
- [x] 2.4 Register scenes `library-grid`, `library-grid-wide` and `library-grid-empty`. Verification: the three scenes snap non-empty and differ from each other in `cargo run -- --scene ... --snap`.

## 3. Consistency

- [x] 3.1 Update `changes/visual-parity/proposal.md` and its design where they describe the library as a list, so the two changes agree. Verification: `openspec validate visual-parity --strict` still passes.
- [x] 3.2 Run the gate: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::cast_possible_truncation`, `cargo test --workspace --locked`, and every scene snap. Verification: all four commands report success.

## Workflow follow-up

- Archive with `openspec archive card-grid --yes` once every task above is checked and the scene baselines for
  the three grid scenes exist in `visual-parity`.
