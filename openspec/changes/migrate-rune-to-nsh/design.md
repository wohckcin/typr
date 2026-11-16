# Design

## Context

See proposal.md for the reasons. What the current code shows:

- `crates/typr` imports Rune from 20+ modules. It uses `El`, `col`, `row`, `text`, `text_input`, `memo`, `ScrollSpec`, `TextEdit`, `Tx`, `Waker`, `Cx`, `AppSpec`, `Image`, `ImageCache`, `Key`, `KeyEvent`, `Mods`, `FileDialogOptions`, `dirs::AppDirs` and `tok`, plus `rune_kit::{Kit, Button, ButtonVariant, Dialog, Markdown, MarkdownStyle, CodeFrame, root}`. Tests use `rune::{Runtime, Env}`.
- Every screen is a `view(&State, &mut Cx) -> El` function that runs again on each state change. NSH runs a component body once and updates only its bindings.
- `crates/typr-core` has no engine dependency. Its only Rune mentions are two comments (`store/task/mod.rs:39`, `keymap/mod.rs:5`).
- `nsh 0.5.0` and Rust 1.99.1 are installed.

## Goals / Non-Goals

**Goals:**
- Every scenario in the screen capabilities passes on NSH on macOS and Linux.
- The app has its own `ui` layer and follows the NSH Design Guides: tokens, states, a motion inventory, and keyboard and accessibility support.
- Behaviour is tested with `Headless`, and appearance is tested with platform screenshot tests.

**Non-Goals:**
- Windows, iOS, Android, web or TUI targets.
- Any change to `typr-core` beyond comments and moving the key-event adapter.
- Visual parity with the Rune pictures to the pixel. The baselines are recorded fresh. The visual-language requirements set what must hold.

## Decisions

1. **Rewrite in place on a worktree branch.** Create `../typr-nsh` on branch `feat/migrate-rune-to-nsh` and replace `crates/typr`'s engine there. `main` keeps building on Rune until the merge. *Rejected:* a parallel `typr-nsh` crate, which keeps two UIs alive and means two edits for every typr-core change; rewriting on `main`, which leaves the app broken for weeks.

2. **The same module layout, mapped onto the recipe.** `src/lib.rs` holds `App` and the `#[native_module]`. `src/shell.rs` holds the sidebar plus the work area. `src/ui/` is the styled layer. The feature modules (`library`, `editor`, `study`, `history`, `settings`, `keymap`, `sidebar`, `embed`, `attachments`, `diagnostics`) keep their names and become pages and components. Tests stay beside their modules, as `project-structure` requires.

3. **State: Rune `State` becomes Copy stores of signals.** Each feature's state becomes a store that only its own methods write. It is provided once by `App` through `provide_context`, or held in a `global` where a listener or task needs it. Access to `typr-core` `Db` runs in `blocking(..)` and is applied in `spawn`. Library loads are `resource`s so that only the latest result counts. Transient drafts stay as signals inside their pages.

4. **Component mapping.**

   | Rune | NSH |
   |---|---|
   | `col`/`row`/`El` | `view()` / `.flex_row()` |
   | `memo` | `computed` |
   | `rune_kit::Button` + `ButtonVariant` | `ui::button(variant, size)` on `nsh_base::button` |
   | `rune_kit::Dialog` | `ui::dialog` on `dialog()`; confirmations use `alert_dialog()` |
   | `rune_kit::Markdown`, `CodeFrame` | `text_view().markdown(..)` with `render_code_block` / `render_custom` for embeds |
   | `TextEdit` / `text_input` | `ui::input` on `input()`; the card source uses `editor()` with the markdown language |
   | `ScrollSpec` | `scroll()`; the card grid and history use `virtual_list` |
   | `Image` / `ImageCache` | `image(path)` sized by the tile |
   | `Key`/`KeyEvent` chords | `on_key("mod+..")` bindings generated from the typr-core keymap; `Chord::from_event` is rewritten for NSH key events |
   | `FileDialogOptions` | `pick_paths` / `pick_save_path` |
   | `dirs::AppDirs` | `paths.rs` resolves the platform data directory itself, so the backend contract is unchanged |
   | `AppSpec` + `rune::run` | `nsh.json` + `platform::run_app` in `main.rs` |

5. **Tokens.** The theme JSON holds light and dark values for typr's semantic roles, with custom `cssVars` for roles the default theme lacks (for example a grade colour or a due state). `ui/tokens.rs` wraps every custom token in one function each, and `ui/motion.rs` holds `instant`/`fast`/`base`/`slow`/`slower`. `tests/tokens.rs` is rewritten to search screen modules for literals.

6. **Testing.** The flow tests in `src/tests/*` keep their subjects and become `Headless` tests that select by role and label, with `MemoryStorage` and a temporary library path. `tests/visual.rs` and the scenes become `screens/*.test.tsx` that open each screen in light and dark and compare its `app.screenshot(..)` with `tests/baselines/<platform>/`. Seeded data comes from a fixture library opened through an environment variable the app already supports.

## Risks / Trade-offs

- [Inline attachments and embeds inside markdown lines. An earlier spike measured them only on stub text metrics.] → The first page task is a spike that renders a card with an inline image and a code embed through `text_view` on macOS and records the result. If the line box does not hold the attachment, embeds render as blocks, and that is noted against `card-embeds`.
- [`editor()` is a code editor and may not match the card editor's markdown behaviours, such as preview and caret placement.] → The `card-editor` scenarios are ported first among the screens. If they need it, the editor falls back to a multiline `input()`.
- [Screenshots exist only in TypeScript tests on a real target, and font rasterisation differs per runner.] → Each platform gets its own baseline set, recorded on that platform; CI records Linux through a dispatch, as it does today.
- [A big-bang branch drifts from `main`.] → typr-core changes merge into the branch every week. UI feature work on `main` stops while the port is open.
- [Linux needs GTK 4.14+ on the CI image.] → Ubuntu 26.04 (resolute), the runner series, publishes `gtk4` 4.22.4, so the floor is met by the distribution. The image must still have `libgtk-4-dev` installed, which the CI task adds; the first CI run on the branch confirms it on the runner itself.

## Migration Plan

1. Create the worktree branch and add the NSH dependencies next to Rune, so the branch builds the whole time. Each Rune screen module is deleted in the same task that lands its NSH replacement.
2. Build the `ui` layer and its gallery page, then the shell, then the screens in dependency order: library, editor, study, history, settings, keymap, embeds, diagnostics.
3. Remove Rune, `vendor/`, the patches and the headless arguments. Update CI and docs. Merge when every capability scenario passes.

Rollback: before the merge, delete the branch. After it, revert the merge commit. The library format is unchanged, so user data opens with either build.

## Spike results (2026-11-15)

- `text_view` renders a card as typr writes it: ATX headings with bold inside, task lists with drawn checkboxes, tables with dividers, fenced code with syntax colours.
- An inline image sits **inside the line box**: `Text before ![chart](/tmp/typr-spike.png) text after` draws the image at about three quarters of a line and wraps the sentence around it. The open question in Risks is closed in favour of the current look.
- `render_code_block` is called once per fenced block with the info string in `language` and the body in `code`: fences written as ` ```typr-embed ` and ` ```typr-embed-missing ` both arrived, so an embed can be drawn as the app's own view, and the two info strings stay distinguishable.
- A fence with no body (` ```typr-embed ` followed straight by the closing fence) does not parse as a code block: its info string is drawn as text. typr-core always writes the card's name as the body (`markdown_ext::expand`), so this only matters if that ever changes.

- `editor()` holds the card's markdown source: `language("markdown")` gives it syntax colours and line numbers, and it is controlled — `value(|| Some(source.get()))` with `on_change(&SharedString)` wrote the whole text back (a two-character insertion left the source at 77 chars, one event). Typing and the caret work as the card editor needs.
- Not exercised by the spike: selection ranges and pasting an image file into the editor. Both are card-editor behaviours that task 4.3 checks before the port of that screen is called done; if either fails, the editor falls back to a multiline `input()`.
- The window exposes the editor to accessibility as `textarea`, so the screenshot and flow tests can reach it by role.

## Task 3.2 decisions

- **Toast** is a plain view bound to a message signal (`ui::overlay::toast`), not `nsh_base`'s toast stack. Rune's equivalent was a single line, and the screens need one message at a time; the stack is revisited if a screen needs queued notifications.
- **Spinner** is a text label (`ui::status::spinner_label`), not an animated control. Rune drew none either; a busy control shows its state through `busy_button`, which disables it and reads "Working…".
- **Tooltip** renders through the element's `tooltip` hint and has no press test: a hint is not pressable. It is checked when the first screen that uses it is ported.

## Task 3.3 layout

- `src/shell.rs` holds the `App` root and the sidebar. The work area is a placeholder until the screens are ported.
- `src/bin/typr_nsh.rs` is the NSH desktop runner. `default-run = "typr_nsh"` makes `cargo run -p typr` open it.
- `src/main.rs` stays as the Rune binary until task 6.1 removes it.

## Task 3.4 token check

- `scripts/check-tokens.sh` checks NSH screens only, starting with `src/shell.rs`. The Rune screen modules are not checked because they are deleted as each screen is ported (tasks 4.x); each port adds its file to the list.
- The `ui` layer is exempt, since it is where the values are named.

## Sidebar selected fill

- `ui::tokens::row_fill(selected)` decides the fill; a unit test checks that the selected and unselected fills differ and that the selected one is not transparent. Pixel confirmation is a 5.1 fixture: a sidebar capture with a selected deck, light and dark.

## Task 4.4 study session

- The screen takes a `typr_core::session::Session`, so due, new, reverse and cram are chosen by the caller. Reveal, grades, Back, Shuffle and Leave come from the review keymap table.
- A refused grade shows "The grade was not saved. Grade again to retry." and leaves the card answerable.
- Queue rules are typr-core behaviour. Named typr-core tests for them: `session/tests.rs` (`a_forgotten_new_card_returns_last_in_the_queue`, `a_forgotten_due_card_returns_when_its_relearning_step_lands`, `quitting_five_of_twelve_leaves_seven_to_answer`, `a_reshuffle_reorders_only_within_one_due_day`, `a_prompt_arriving_mid_session_joins_the_end`, `a_refused_grade_frees_the_card_again`, `a_reverse_entry_shows_the_last_side_first`, `a_cram_session_starts_with_every_entry_queued`); `store/tests/review.rs` (`the_new_queue_stops_at_the_cap_and_resumes_the_next_day`); `store/tests/cards.rs` (`a_queue_can_be_scoped_to_one_deck`); `store/tests/views.rs` (`a_cram_selection_ignores_due_dates_and_the_cap`).
- Cram session behaviour is covered by named tests: `session/tests.rs` (`a_cram_session_writes_no_schedule`, `a_cram_session_repeats_what_was_forgotten`, `one_card_three_passes`, `a_cram_session_starts_with_every_entry_queued`) and `store/tests/views.rs` (`a_cram_sessions_filter_can_be_kept_as_a_view_and_never_on_its_own`, `a_cram_selection_ignores_due_dates_and_the_cap`, `a_cram_selection_matches_the_list`).

## Task 4.8 embeds

- `screens/embed.rs` rewrites embeds with typr-core's `markdown_ext::expand`, then draws each `typr-embed` fence with the body, `typr-embed-missing` with "No card named …", and a self-embed with a marker.
- A nested embed (an embed inside an embedded body) draws "“Name” embeds another card" instead of the body or the inner card; `tests/embed.rs` checks both are absent.

## Task 4.5 history

- `screens/history.rs` lists every record newest first with phase, grade, time and due date; `review_count` excludes learning-phase records. `tests/history.rs` verifies count, order and the empty state.


## Task 4.6 settings/preferences

- The NSH screen accepts and refuses values (limit named, previous kept), reports the library's file (`LibraryInfo::of`): database path, schema version, size, attachments folder and log path, each with a "Show … in the file manager" control that calls `reveal_path`, plus Import and Export.
- Reachable from the library: the shell's Settings row puts the screen in the work area, and Escape returns to the library (`tests/shell.rs`). Reachable from a session: the study screen binds the Settings chord from the table to its caller's `on_settings` (`tests/study.rs`).
- `tests/settings.rs` covers the refusal, the paths and controls, Import/Export, and `an_accepted_cap_persists_across_a_restart` against a temp `typr_core::store::Db`; core settings tests cover validation, corrupt values and round-trips.


## Task 4.7 keymap panel

- `chord_from_keystroke` maps NSH keystroke strings to typr-core `Chord`; `keystroke_of` turns a table chord into the keystroke `on_key` takes, writing Shifted characters as the key they are typed on (`?` is `shift+/`). `keymap_panel` opens and closes on the surface's shortcut chord and its list narrows as the user types; `palette` narrows the surface's commands and Enter (the field's submit) runs the first left. A command the surface does not bind is never offered.
- `tests/keymap.rs` verifies the mapping, the shifted form, the listing, the open/close toggle, filtering, and the palette's Enter run. The palette indicator is drawn before the row list: a row list of cards following it is drawn into its own subtree.

## Task 4.9 diagnostics

- `screens/diagnostics.rs` states a stoppage once, lists slow commands or an empty state, and runs import/export callbacks with a result line. `tests/diagnostics.rs` verifies all of these. The spec's file-log, schema-facts and quit-recording requirements are typr-core/logging behaviour with their own code paths.


## Task 6.2

- `crates/typr-core` no longer names Rune (comments updated in `store/task/mod.rs`, `store/tests/protocol.rs`, `keymap/mod.rs`, `Cargo.toml`). Rune still remains in `crates/typr`, `vendor/`, root patches and CI until 6.1/6.3.


## Task 4.3 card editor and attachments (partial)

- `screens/editor.rs` gains `attach_image` (copies a chosen picture into the attachments root, refuses one that cannot be copied), `image_reference` and `image_references`. The editor takes `on_attach`, offers an "Attach image" action through `pick_paths`, appends the reference to the draft and shows the copy error. `tests/editor.rs` covers the copy/refusal and the reference text.
- Not done: image occlusion (diagram rectangles), templates and fields in the editor, and recovery of an unsaved draft. 4.3 stays unticked.

- `editor_with_draft` opens on an unsaved draft, marks it "Unsaved", and reports each edit so the caller can write the draft down (`tests/editor.rs` checks the restore and the badge). Headless cannot type into the source editor (it exposes no input role), so the typing report is wired but not driven by a test.

## Task 5.1 appearance in both themes

- `tests/appearance.rs` renders every screen in `ColorScheme::Light` and `Dark` and reads the resolved style with `screen().span_style`: font size (bits) and weight must be equal, and the colour must equal a reference text painted with the same token (`tokens::foreground` or `tokens::muted_foreground`), so it is the theme's colour for that role and appearance, not merely a different one.
- `screens::NAMES` and the test's own list must agree. Both are hand-maintained: a module added without touching either is not caught, and the guard fires only when a name is added to `NAMES` without its check. Rust cannot enumerate modules, so there is no drift-free form; adding a screen means adding its name and its check together.
- No images: `Headless` has no screenshot at all (the method lives on a private driver and returns "a screenshot needs a real platform"), and `nsh test --target` cannot run until Rune is removed (it resolves `.nsh/rust/*` against Rune's root-only patches).

## Task 6.1 Rune removed

- `crates/typr` keeps `screens/`, `ui/`, `shell.rs`, `paths.rs` and the desktop runner; the Rune modules (`main.rs`, `editor/`, `library/`, `sidebar/`, `sidebar.rs`, `design/`, `attachments/`, `history/`, `keymap/`, `logging/`, `markdown/`, `study/`, `screen.rs`, `settings.rs`, `day_offset.rs`, `src/tests/`) and `vendor/` are gone.
- `paths.rs` uses `nsh_core::prelude::data_dir()` with `TYPR_DATA_DIR` overriding it, replacing `rune::dirs::AppDirs`.
- `cargo tree --workspace | grep -ci rune` is 0 and `Cargo.lock` names no rune package; the workspace suite (308 core + screen tests) passes.

## Task 6.4 README

- `README.md` now documents the Native Shell app: two-crate layout, prerequisites (Rust pin, `nsh` CLI, GTK 4.14 on Linux), `cargo run -p typr`, `nsh run`, `cargo test`, `nsh test`, `nsh check`, card syntax, scheduling keys and the data folder, with `TYPR_DATA_DIR` as the override.
- Verified: `cargo build -p typr`, `nsh check .`, `nsh test .` (96 passed), `nsh run --target tui` for 25 s with no panic. `cargo run -p typr` starts; the `owner was disposed` panic seen only appears when the process is killed by a timeout.
- `nsh.json` needs both `"root": "App"` and `"rust": "crates/typr"`, so `crates/typr/src/lib.rs` carries a `#[native_module("typr/native")]` (registering nothing) — without it `nsh check`/`nsh test` fail on the generated describe crate.

## Diagnostics after 6.1 (4.9 un-ticked)

- The logger is ported: `logging/` (restored, Rune-free) installs a bounded file under `data_dir/logs`, rotates at 1 MiB, takes its level from `TYPR_LOG` (default `warn`), formats each record and hands it to a writer thread so the frame thread never touches the file, and flushes at quit. `bin/typr_nsh.rs` installs it; `paths::open_from_environment` records the library path and its schema facts at info; `shell::App` logs a failed open or snapshot with `logging::describe`'s cause chain.
- 4.9 is un-ticked because two `diagnostics` scenarios still have no implementation: "A slow command is recorded with its cost" and "A worker that ended abnormally is recorded when the application quits". Both need the store worker (`typr_core::store::Handle` + `WorkerStatus`) wired into the app, which is part of connecting the screens to the library. The log file, its rotation, the level, the startup facts, the cause chain and the never-stalls guarantee are in place.
