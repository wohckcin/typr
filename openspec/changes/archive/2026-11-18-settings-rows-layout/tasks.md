# Tasks — settings-rows-layout

## 1. The `ui` layer

- [x] 1.1 `ui/segmented.rs`: a `segmented` control over `Vec<(label, value)>`, the value in force and an `on_change`;
      each segment a button named by its label, the chosen one `selected` with the card fill and border.
      Verification: `tests/segmented.rs` cases `pressing_a_segment_reports_its_value` and
      `the_chosen_segment_is_the_selected_one` (read through `accessibility_state`).
- [x] 1.2 `ui/rows.rs`: `rows` (the card: border-coloured background, 1 pt gap, rounded clip) and
      `setting_row(label, control)`.
      Verification: `tests/rows.rs` case `a_row_has_its_label_before_its_control` (the label's x is less than the
      control's); `bash scripts/check-tokens.sh` finds no literal outside `ui/`.

## 2. The settings screen

- [x] 2.1 Rebuild `screens/settings.rs` as the header over one `rows` card, in the order Review, Appearance, Library:
      New cards a day (field and Apply), Theme (segmented), Show review counts (segmented Off / On), the three
      paths (value and reveal button), Schema and size (value rows), Import, Export.
      Verification: `cargo test -p typr --test settings --test a11y --test shell` pass with no assertion edited.
- [x] 2.2 The header: the breadcrumb "Settings / General" and the "Find a setting" field, `Values::query` and
      `row_matches(label, query)`.
      Verification: a unit test of `row_matches` (case, substring, empty query); `tests/settings.rs` cases
      `typing_a_name_hides_the_other_rows`, `a_query_nothing_matches_says_so` and `clearing_the_query_restores_the_rows`.
- [x] 2.3 Remove the screen's use of `list_row`, `toggle` and `section`; leave `ui::toggle` in place.
      Verification: `grep -n 'toggle()\|list_row()\|fn section' crates/typr/src/screens/settings.rs` finds nothing;
      `cargo test -p typr --test toggle` still passes.

## 3. Proof

- [x] 3.1 Screenshots of the settings screen at 1920 × 1080 and 390 pt, light and dark, beside the reference: rows
      share a height, labels start on one spine, controls end on one spine, the hairlines are single, nothing
      clips, the focus ring is whole.
      Verification: each group of the Finishing checklist reported passed or as what was fixed, in `DESIGN.md`'s
      appearance review table.
- [x] 3.2 `cargo test --workspace`, `nsh test`, `nsh check`, `cargo clippy --all-targets -- -D warnings` and
      `bash scripts/check-tokens.sh`, once, at the end.
      Verification: all green.
