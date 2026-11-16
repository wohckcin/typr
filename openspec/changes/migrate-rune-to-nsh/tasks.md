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
- [x] 4.3 Card editor and attachments (`pick_paths`, image tiles); verify `card-editor` scenarios
  - the attached image's reference is typed at the editor's caret through a `ComponentRef<Editor>` (`ui::source::insert_at_caret`), proven by `a_reference_is_typed_at_the_caret_not_appended`
  - leaving asks Save/Discard/Stay through the header's Back control, which works while the source holds the keyboard; Escape reaches the same question when the editor is not focused (the code editor keeps Escape for itself while focused)
- [x] 4.4 Study session and review canvas with keyboard grading via `on_key`; verify `study-session` and `review-scheduling` UI scenarios
- [x] 4.5 History; verify its flow tests
- [x] 4.6 Settings/preferences; verify `preferences` scenarios and persistence across a restart with a temp library
- [x] 4.7 Keymap panel and palette, with `Chord::from_event` rewritten for NSH key events; verify `keymap-panel` scenarios
- [x] 4.8 Embeds and markdown rendering per the spike's decision; verify `card-embeds` scenarios
- [x] 4.9 Diagnostics and import/export entry points; verify `diagnostics` and `transfer` UI scenarios
  - Diagnostics is a Utilities row: it states a stoppage, lists slow commands, and offers Import and Export through the system dialogs. `shell::export_to`/`import_from` hold the work, tested for a re-openable export, a refused overwrite and a foreign file named in the refusal.
- [x] 4.10 Run the Finishing checklist and accessibility checklist on each screen in light and dark with `nsh run`, keyboard-only; record per-group results in the PR
  - run in the window (computer use over `typr_nsh`, seeded library in `/tmp/typr-audit`), dark and light: library, editor (sides, hidden text), session and summary, settings, diagnostics; the per-group results are in `design.md`
  - the audit found and this work fixed: input text, placeholders and carets were invisible (no colour reached the field); settings' and diagnostics' section names were drawn as full-width bordered chips that read as empty fields; the library had no search field though the sidebar named its chord; tile names carried raw markdown; no row was selected at launch; sidebar rows had no role, hover or pressed state; Apply stretched the full width
  - the audit also found the library's keymap unwired, which this work now binds: Down/`j` and Up/`k` walk the grid with a visible ring, Enter edits the card in force, `s` and `n` start the due and new sessions, `c` makes a card, Backspace trashes one, and the session runs on Space/`j`/`f` to a summary with Remembered/Forgot counts, the due count and a way back
  - minimum window size: checked at 520×420 in the window (the tiler was disabled so the resize took); this found and fixed the grid counting columns from the window width instead of the space beside the sidebar, which squeezed two 140 pt tiles in where one 265 pt tile belongs
  - larger text: the platform pins the text size to 1.0 on macOS and the runner exposes no hook, so it was checked headlessly — at `font_scale = 1.5` the sidebar's text grows and every control stays reachable (`a_larger_text_size_reaches_every_screen_without_losing_a_control`)
  - screen reader / accessibility semantics: checked through the accessible tree a reader consumes (the same NSAccessibility data computer use reads): every control's role, name, state and the editor's live focus were read on the running window; VoiceOver's caption panel was exercised too. What was not captured is the spoken audio — that needs a human ear on the machine, and it does not change what the app exposes
  - the fold chevron, the one icon-only control, now names itself ("Expand/Collapse <deck>") and `the_fold_chevron_names_what_it_does_and_what_it_folds` proves it

## 5. Screenshot baselines

- [x] 5.1 Each screen's Rust test renders it in the light and the dark appearance, asserts the same text resolves to the same size and weight, and that its colour changes with the appearance; a screen with no such test fails by name
- [x] 5.2 Delete `tests/visual.rs` and `tests/baselines/` (the Rune scene table goes with the Rune binary in 6.1); CI drops the baseline record/compare steps (the rest moves to 6.3)
- [x] 5.3 State in `DESIGN.md` that appearance is verified by resolved styles, not captured images, and that no screenshot pipeline remains

## 6. Remove Rune and finish

- [x] 6.1 Remove `rune`, `rune-kit`, the root `[patch.crates-io]`, `vendor/`, `scripts/vendor-*`, the headless CLI arguments and `tests/visual.rs`; verify `cargo tree --workspace | rg -i rune` prints nothing
- [x] 6.2 Update the two typr-core comments that name Rune; verify `rg -i rune crates/typr-core` prints nothing
- [ ] 6.3 Update `.github/workflows/ci.yml` (GTK and xvfb packages, `cargo test --workspace --locked`, `nsh test --target linux` under `xvfb-run -a`) and verify CI passes on the branch
  - the workflow already installs libgtk-4-dev + xvfb, checks GTK ≥ 4, runs fmt, the token check, clippy `-D warnings -W cast_possible_truncation`, the backend's toolkit/log checks, build, `cargo test --workspace --locked` and `xvfb-run -a nsh test --target linux`
  - every step that can run on macOS now passes locally (clippy needed 124 fixes: mechanical closures by `--fix`, then path args, type aliases and bounded casts by hand); the two Linux-only pieces — `nsh test --target linux` and the CI run itself — need a push, which is not mine to make
- [x] 6.4 Update `README.md` (prerequisites, `cargo run`, `nsh run`, `nsh test`, layout table); verify each documented command runs
- [x] 6.5 Open a library written by the Rune build with the NSH build and grade a card; verify history appends unchanged
  - the Rune-era fixture (`crates/typr-core/tests/data/pre-split-library.sqlite3`) opened in the NSH app at `/tmp/typr-legacy`: the log reads `schema 3 -> 12, 9 migrations`
  - graded through the window with the keyboard (`s`, Space, `j`): the summary said "Remembered 1 · Forgot 0 · 1 due when you started"
  - the old review is untouched and the new one appended under the same key: `(1,'fwd','remembered',1768136400000)` then `(1,'fwd','remembered',1794811360464)`; the prompt's `reps` went 1 → 2 and its due date moved ahead

## Workflow follow-up

- Merge the branch into `main` after review and remove the worktree.
- Archive the change and verify `openspec/specs/project-structure` and `visual-language` reflect the deltas.
