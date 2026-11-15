# Tasks — visual parity

Depends on `settings-screen` (theme and scale tokens) and on every screen it restyles.

## 1. Tokens

- [x] 1.1 New `crates/typr/src/design.rs`: `space(u8) -> f32` on a 4 px scale, `type_body/type_title/type_caption` from the kit, `radius`, `elevation`, `focus_ring`, and `surface_card(children) -> El`.
  - **Verification**: `cargo test -p typr --lib design` asserts each token's value and that `surface_card` nests its children inside one bordered element.
- [ ] 1.2 A CI step refusing a spacing or font-size literal inside a screen file (`grep -nE '\.(px|gap|padding|w|h)\([0-9]'` over `crates/typr/src/{library,editor,study,history,settings}.rs`, with the token module excluded).
  - **Verification**: the step passes on the final tree and fails when a literal is reintroduced, checked by hand in a scratch worktree.

## 2. Card object

- [ ] 2.1 Library rows become cards through `surface_card`: name, preview line, metadata as secondary text, hover and focus states that change colour and shadow only.
  - **Verification**: `a_row_is_one_surface_with_its_metadata_inside` reads `runtime.elements()` and asserts one border ancestor contains name, preview and metadata; `hovering_a_row_moves_nothing` compares geometry before and after a pointer move.
- [ ] 2.2 Sidebar groups into Decks, Tags and Views with headings and counts, empty groups included.
  - **Verification**: `the_sidebar_groups_and_counts` asserts the three headings and their counts, and `an_empty_group_is_still_named`.

## 3. Review surface

- [ ] 3.1 `review_canvas`: dimmed background, centred card, reserved answer slot, hints below the card.
  - **Verification**: `revealing_does_not_move_the_card` compares the card's rect before and after a reveal; `the_hints_sit_under_the_card` compares their rects.
- [ ] 3.2 Empty states everywhere a list or queue can be empty, including the queue offering a cram session.
  - **Verification**: `an_empty_library_offers_a_card` and `an_empty_queue_offers_a_cram`.

## 4. Baselines

- [ ] 4.1 `crates/typr/tests/visual.rs`: render each scene through the tooling path, compare with `tests/baselines/<scene>.png` with a per-channel tolerance of 2 and a differing-pixel share of 0.5%, and fail on a missing baseline.
  - **Verification**: `every_scene_matches_its_baseline` passes on the final tree; `a_changed_token_fails_a_baseline` is demonstrated by changing one space token in a scratch worktree and seeing the test name the scene.
- [ ] 4.2 Baselines generated on the CI image and committed, with the machine and commit recorded in a README beside them.
  - **Verification**: the README names the image; the suite passes from a clean checkout.
- [ ] 4.3 CI runs the visual test in the scene step it already has.
  - **Verification**: `ci.yml` parses and the step names the test.

## 5. Close out

- [ ] 5.1 README documents the token set, how to regenerate a baseline, and how to add a scene.
  - **Verification**: the section exists and names both commands.
- [ ] 5.2 Full gate: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::cast_possible_truncation`, `cargo test --workspace --locked`, every scene snapping, baselines matching.
  - **Verification**: all report success.

## Workflow follow-up

- Archive after the changes before it. This is the last roadmap row; the remaining Mochi features that
  are out of scope are sync, deck sharing and the REST API, and the README says so.
