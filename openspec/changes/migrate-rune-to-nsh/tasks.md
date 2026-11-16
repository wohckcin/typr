# Tasks

## 1. Branch and dependencies

- [x] 1.1 Create worktree `../typr-nsh` on branch `feat/migrate-rune-to-nsh` and verify `git worktree list` shows it
- [x] 1.2 Add `nsh-core`, `nsh-base`, `nsh-platform-macos`/`nsh-platform-linux` (per target) and dev-deps `nsh-engine`, `nsh-testing` (`default-features = false`) from one workspace table at the `nsh 0.5.0` version; verify `cargo check -p typr --all-targets --locked` passes with Rune still present
- [x] 1.3 Add `nsh.json` (`"rust": "crates/typr"` or per-crate, `"root": "App"`, id, appearance `system`) and verify `nsh check` passes
- [x] 1.4 Confirm the CI image provides GTK ≥ 4.14 (`pkg-config --modversion gtk4` in a dispatch run) and record the result in design.md

## 2. Spike: markdown and embeds on text_view

- [x] 2.1 Render a card with a heading, a code block, an inline attachment image and an embed through `text_view` on macOS with `nsh run`; record screenshots and whether the line box holds the inline image, and update design.md's embed decision accordingly
- [x] 2.2 Render the card source in `editor()` with the markdown language and check the `card-editor` behaviours it must support (typing, caret, selection, paste of an attachment); record the editor-vs-input decision in design.md

## 3. App foundations and ui layer

- [x] 3.1 Theme file with typr's semantic roles in light and dark, plus `ui/tokens.rs` and `ui/motion.rs`; verify a `Headless` test reads every custom token without a fallback
- [x] 3.2 `ui` core set: button/icon button (variants, sizes, hover/active/focus/disabled/loading), input, field, checkbox, switch, select, tabs/segmented, menu, popover, tooltip, dialog, alert helper, toast, list row, surface/card, badge, separator, empty, skeleton, spinner; verify a gallery page shows every variant and state and a `Headless` test presses each control by role
- [x] 3.3 `App` root, `main.rs` desktop runner and `shell.rs` (sidebar + work area) with stores provided by context; verify `cargo run -p typr` opens the window and a `Headless` test sees the sidebar headings
- [x] 3.4 Rewrite `tests/tokens.rs` to fail on spacing, font-size, radius or colour literals and on direct `nsh_base` control use in screen modules; verify it fails on a planted literal and passes after removing it
- [x] 3.5 Write the motion inventory for every screen in `DESIGN.md` before screen work starts

## 4. Screens (each task deletes its Rune module and ports its flow tests to Headless)

- [x] 4.1 Sidebar (Decks/Tags/Views, counts, filtering); verify `sidebar` scenarios via `Headless`
- [x] 4.2 Library and card grid/tiles on `virtual_list` with keyed rows; verify `card-library`, `card-grid`, `deck-views` scenarios including a recycling scroll test
- [ ] 4.3 Card editor and attachments (`pick_paths`, image tiles); verify `card-editor` scenarios
  - done: source/preview, image action and resolution, prompt caption, field table with a template, Save/Discard/Stay, draft restore, empty save refused, diagram pane with drag-to-rectangle, per-group covers, merge
  - remaining: the text↔diagram switch (a card cannot be created as a diagram, nor switched, from the editor); a way to remove a whole group (only one rectangle can be removed; the store has no group-removal command); proof that covers state their group without the button strip (their labels sit inside absolutely placed views and never appeared in the accessibility tree in any test)
- [x] 4.4 Study session and review canvas with keyboard grading via `on_key`; verify `study-session` and `review-scheduling` UI scenarios
- [x] 4.5 History; verify its flow tests
- [x] 4.6 Settings/preferences; verify `preferences` scenarios and persistence across a restart with a temp library
- [x] 4.7 Keymap panel and palette, with `Chord::from_event` rewritten for NSH key events; verify `keymap-panel` scenarios
- [x] 4.8 Embeds and markdown rendering per the spike's decision; verify `card-embeds` scenarios
- [ ] 4.9 Diagnostics and import/export entry points; verify `diagnostics` and `transfer` UI scenarios
- [ ] 4.10 Run the Finishing checklist and accessibility checklist on each screen in light and dark with `nsh run`, keyboard-only; record per-group results in the PR

## 5. Screenshot baselines

- [x] 5.1 Each screen's Rust test renders it in the light and the dark appearance, asserts the same text resolves to the same size and weight, and that its colour changes with the appearance; a screen with no such test fails by name
- [x] 5.2 Delete `tests/visual.rs` and `tests/baselines/` (the Rune scene table goes with the Rune binary in 6.1); CI drops the baseline record/compare steps (the rest moves to 6.3)
- [x] 5.3 State in `DESIGN.md` that appearance is verified by resolved styles, not captured images, and that no screenshot pipeline remains

## 6. Remove Rune and finish

- [x] 6.1 Remove `rune`, `rune-kit`, the root `[patch.crates-io]`, `vendor/`, `scripts/vendor-*`, the headless CLI arguments and `tests/visual.rs`; verify `cargo tree --workspace | rg -i rune` prints nothing
- [x] 6.2 Update the two typr-core comments that name Rune; verify `rg -i rune crates/typr-core` prints nothing
- [ ] 6.3 Update `.github/workflows/ci.yml` (GTK and xvfb packages, `cargo test --workspace --locked`, `nsh test --target linux` under `xvfb-run -a`) and verify CI passes on the branch
- [x] 6.4 Update `README.md` (prerequisites, `cargo run`, `nsh run`, `nsh test`, layout table); verify each documented command runs
- [ ] 6.5 Open a library written by the Rune build with the NSH build and grade a card; verify history appends unchanged

## Workflow follow-up

- Merge the branch into `main` after review and remove the worktree.
- Archive the change and verify `openspec/specs/project-structure` and `visual-language` reflect the deltas.
