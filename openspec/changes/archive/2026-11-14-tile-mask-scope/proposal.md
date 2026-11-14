# Tile masking covers every span

## Why

`openspec/specs/card-grid/spec.md` says a tile draws hidden spans "exactly as a review face does before a
reveal". A review face hides only the group it is asking about, so that comparison now points at the wrong
scope: a card with `{{1::air}}` and `{{2::oxygen}}` is asked one group at a time, but its tile shows every
answer masked, because a grid is where an answer is easiest to read by accident. The code already does
this — `crates/typr/src/library/tile.rs` calls `typr_core::hidden::masked_all` — and the requirement text is
the only thing that disagrees.

## What Changes

- The tile requirement states that every hidden span is masked, and that a numbered span is labelled with
  its group so the grid shows how many prompts the card asks.
- No data, schema or scheduling change; the code and the wording are brought back into agreement.

## Impact

- Affected specs: `card-grid` (one modified requirement).
- Affected code: none — `typr_core::hidden::masked_all` and its callers in
  `crates/typr/src/library/tile.rs` and `crates/typr/src/editor.rs` already match the new wording, proved by
  `a_tile_hides_every_answer_a_hidden_text_card_holds` and
  `the_preview_masks_every_group_and_names_the_one_it_hides`.
