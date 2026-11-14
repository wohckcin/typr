# Tasks — tile masking

## 1. Wording

- [x] 1.1 Reword "A tile shows the first side of its card" in `openspec/specs/card-grid/spec.md` through this
      change's delta so it states that every hidden span is masked and every numbered span is labelled, with
      all three scenarios in place.
  - **Verification**: `openspec validate card-grid --type spec --strict` passes and the merged text contains
    "every hidden span SHALL be drawn masked".

## 2. Code already in place

- [x] 2.1 Confirm the tile masks every span and labels groups, and that no test pins the older
      first-group-only behaviour.
  - **Verification**: `cargo test -p typr --lib library::tile` passes, including
    `a_tile_hides_every_answer_a_hidden_text_card_holds`, and changing `masked_all` to `masked` in
    `crates/typr/src/library/tile.rs` makes that test fail.

## Workflow follow-up

- [x] Archive this change once the sync writes the reworded requirement into `openspec/specs/card-grid/spec.md`.
