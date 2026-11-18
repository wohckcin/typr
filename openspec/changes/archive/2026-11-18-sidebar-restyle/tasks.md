# Tasks — sidebar-restyle

## 1. The success token

- [x] 1.1 Add `success` to `crates/typr/theme.json` (light and dark) and `ui/tokens::success()`.
      Verification: `cargo test -p typr --test tokens` passes; a unit test reads `success()` in both appearances
      and the two differ.
- [x] 1.2 A `DOT` size constant in `ui/size.rs` for the state dot.
      Verification: `bash scripts/check-tokens.sh` finds no size literal in `screens/sidebar.rs`.

## 2. The footer

- [x] 2.1 Make the column a flex column: search row, the groups' region with `flex_1`, then `state_row(state)` with a
      hairline above it; remove `library_state` from under the search row.
      Verification: `tests/sidebar.rs` case `the_state_row_is_below_every_group` (the state text's y is greater than
      the y of the last row of the last group) and `the_groups_follow_the_search_row` (no state text between the
      search row and the first heading).
- [x] 2.2 The dot: a circle coloured `success()` when the library is open and `destructive()` when it is closed,
      beside the unchanged state text; no accessible name on the dot.
      Verification: `tests/sidebar.rs` cases `an_open_library_has_a_success_dot` and
      `a_closed_library_has_a_danger_dot` read the dot's `background_color` through `view_style`.
- [x] 2.3 Existing assertions untouched.
      Verification: `cargo test -p typr --test sidebar --test shell --test startup --test a11y` and `nsh test`
      pass with no assertion edited.

## 3. Proof

- [x] 3.1 Screenshots of the sidebar at 1920 × 1080 and 390 × 600, light and dark, beside the reference: the footer
      is at the bottom, the hairline is single, the dot is visible against both backgrounds, nothing is pushed out
      of a short column.
      Verification: each group of the Finishing checklist reported in `DESIGN.md`'s appearance review table.
- [x] 3.2 `cargo test --workspace`, `nsh test`, `nsh check`, `cargo clippy --all-targets -- -D warnings` and
      `bash scripts/check-tokens.sh`, once, at the end.
      Verification: all green.
