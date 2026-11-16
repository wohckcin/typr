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

## Task 4.10 Finishing checklist, per group (window audit, dark and light)

- **Radius**: the cards, fields, chips, popups and buttons all take `tokens::radius`; nothing on a screen names a number (`scripts/check-tokens.sh`). Concentricity where a chip sits on a card reads right; no rectangle is a near-pill by accident; `rounded_full` only on the switch track and thumb.
- **Typography**: three sizes in use (`text_sm` for interface text, `text_xs` for metadata, `text_sm` monospace in the source); bold appears only on the selected sidebar row and the primary Save; secondary text is `muted-foreground`, measured #737373 on white (4.7:1) and #a3a3a3 on #0a0a0a.
- **Spacing and gaps**: every gap and padding is a step (`gap_1..4`, `p_3/p_4`, `px_3/py_1`), and nested panels return to their parent's inset; no doubled insets seen.
- **Alignment**: sidebar rows share one leading spine and keep their due chips in a trailing lane; tiles share their leading edge and their schedule line; settings' paths and their buttons sit on one baseline; the editor's source and preview split the width evenly.
- **Color and surfaces**: every colour is a token; one primary action per surface (Save in the editor, Apply in the review card); chips are quiet hairlines; the popup (the leave question, the clear question) is the only elevated surface.
- **Hover and pressed**: rows take a wash weaker than selection (`accent` at 60%) and press to the selection fill; buttons move to 85–90% opacity and ring the keyboard focus; tiles take the button's treatment through `tile_button`.
- **The `ui` layer**: every screen builds from `crate::ui`; the source check refuses an `nsh_base` control or a literal, and the gallery test shows each component in its states.
- **Component standards**: one height per size (buttons 32, fields 32, chips from the type scale), popups on one surface, focus rings outside the border.
- **Other states**: focus rings on every keyboard-reachable control (rows, tiles, tabs, toggles, fields, buttons); the fold control is the one icon-only button, and it names itself for a reader ("Expand Spanish" / "Collapse Spanish") through `ui::button::button_labelled`.
- **Motion and sound**: no motion on these screens beyond the switch thumb and the press settle; the app plays no sound.
- **Accessibility and keyboard (blocking)**: the main task completes with the keyboard alone — Down walks the grid with a ring, Enter edits, `s`/`n` start a session, Space reveals, `j`/`f` grade, Escape returns; every control is a role with a name (checked through the AX tree and `tests/a11y.rs`); nothing relies on colour alone (the selected row is filled *and* the tiles ring).
- **Platforms**: desktop only in this port; the web and mobile columns do not apply.
- **Content states**: empty (Diagnostics states "No slow commands recorded"; the grid states an empty deck with the new-card action), long content truncates in tiles and scrolls in the source. Larger text and a scripted minimum window were not exercised.

**Passed on the accessibility semantics a screen reader consumes, with one caveat named below.**

- **Minimum window**: checked at 520×420 with the tiler off. It exposed a real bug — `grid_columns` counted from the window width, so two 140 pt tiles were squeezed beside a 220 pt sidebar. The grid now counts from the space beside the sidebar (`content_width`), so a narrow window shows one full-width tile. Screenshot at 520×420 confirms it.
- **Larger text**: macOS NSH pins `font_scale` to 1.0 and `run_app` exposes no hook to set it, so the window cannot show larger text. Checked headlessly at 1.5: the sidebar's text grows and every control stays reachable (`tests/a11y.rs`, `a_larger_text_size_reaches_every_screen_without_losing_a_control`).
- **Screen reader**: checked through the accessibility tree a reader consumes — the same NSAccessibility data computer use reads, on the running window: every control exposes a role and a name (`button "All cards" (selected)`, `textfield "Search cards"`, `button "Back"`, `button "Save"`, the fold's `Collapse Spanish`), the editor's focus is reported live, and the tree tracks state changes (selected, focused, disabled). VoiceOver was also launched and its caption panel toggled, and it enumerated the app's window. Caveat: the spoken audio itself was not captured — confirming *what a voice says* needs a human ear; the semantics it speaks from are verified.

Fixed from this audit: input text/placeholder/caret colour (none reached the field before), section names drawn as chips, the missing library search field, raw markdown in tile names, no default selection, rows without role/hover/pressed/focus, `Apply` at full width, and the library's keymap (selection, Enter, `s`/`n`, `c`, Backspace) with the session, its grades and a summary behind them.

## Task 4.9 diagnostics and transfer entry points

- The sidebar's Utilities now holds Diagnostics beside Settings. `shell_over` mounts `diagnostics` with three signals it owns: `stopped` (set at launch when the library did not open, and by every refused command), `slow` (a command that took `SLOW_COMMAND_MS` = 100 ms or more, by name and cost) and `result`.
- Export asks `pick_save_path` and runs `Command::ExportLibrary`; import asks `pick_paths` and runs `Command::ImportArchive` into the library's attachments folder. Both report through `shell::export_to` / `shell::import_from`, which return the line the screen shows, so the work is testable apart from the dialogs.
- Tests: `tests/shell.rs` opens Diagnostics from the row and sees both entry points; `export_to` writes a file that reopens as a library with its cards, refuses to overwrite an earlier export, and `import_from` names a foreign file and writes nothing. The `diagnostics` screen's own scenarios stay in `tests/diagnostics.rs`; the transfer round trip's details stay in typr-core's `tests/transfer.rs` (19).

## Task 4.9 diagnostics (first pass, kept)

- `screens/diagnostics.rs` states a stoppage once, lists slow commands or an empty state, and runs import/export callbacks with a result line. `tests/diagnostics.rs` verifies all of these. The spec's file-log, schema-facts and quit-recording requirements are typr-core/logging behaviour with their own code paths.


## Task 6.2

- `crates/typr-core` no longer names Rune (comments updated in `store/task/mod.rs`, `store/tests/protocol.rs`, `keymap/mod.rs`, `Cargo.toml`). Rune still remains in `crates/typr`, `vendor/`, root patches and CI until 6.1/6.3.


## Task 4.3 card editor and attachments (partial)

- `screens/editor.rs` gains `attach_image` (copies a chosen picture into the attachments root, refuses one that cannot be copied), `image_reference` and `image_references`. The editor takes `on_attach`, offers an "Attach image" action through `pick_paths`, appends the reference to the draft and shows the copy error. `tests/editor.rs` covers the copy/refusal and the reference text.
- Not done: image occlusion (diagram rectangles), templates and fields in the editor, and recovery of an unsaved draft. 4.3 stays unticked.

- `editor_with_draft` opens on an unsaved draft, marks it "Unsaved", and reports each edit so the caller can write the draft down.
- Typing is testable: `editor()` exposes no input role by default, but `ui::source` now sets `accessibility_label("Card source")`, so `type_text("label:Card source", ..)` reaches it. `tests/editor.rs` types into the draft and asserts both that the report carries the new text and that the preview draws it. `nsh test --target` is not a route for this: `open()` requires an `App.tsx`, which a Rust-root app does not have.

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

## Task 4.3 progress (still open)

- Images: `resolve_images` rewrites `![alt](name)` to the absolute path under the attachments folder, leaves an absolute or `..` reference as written (the renderer then draws its placeholder), and the preview applies it. `prompt_count` counts hidden-text groups or a two-sided card and shows on the editor caption.
- Templates: `template_preview` renders a card from `typr_core::template::render_card`; `field_table` lists a value per field with computed rows read-only, and reports edits.
- Leaving: `leaving_choice(dirty)` asks only when there is something to lose; `leave_decision` offers Save, Discard and Stay and reports the answer.
- Occlusion: `rect_from_drag` turns a drag from the picture's middle into a fraction rectangle, refuses anything under one percent on either axis (a click adds none), and `cover_box` places a cover at any pane size. Tests cover both.
- Still missing before 4.3 can be ticked: the diagram pane that draws the covers and wires `rect_from_drag` to `AddMask`/`MergeMasks`/`RemoveMask`, the text↔diagram switch, and the editor's connection to the store (the shell has no editor route yet).

### What the editor mounts, and what it does not

- Mounted in `editor_full`: the source, the preview (from `template_preview` when a template is in force, else the sides, with `resolve_images` applied), the `field_table` only when a template is present, the Saved/Unsaved and prompt badges, Save, Attach image, and `leave_decision` on the Leave chord (`escape`), which asks only when the draft differs from what was saved. Editor-level tests: `a_templated_card_shows_its_field_table_and_previews_the_template`, `a_card_without_a_template_shows_no_field_table`, `leaving_a_dirty_editor_asks_and_stay_keeps_it`, `leaving_a_dirty_editor_can_discard`, `leaving_a_dirty_editor_can_save_first`, `leaving_a_clean_editor_does_not_ask`.
- NOT mounted: the occlusion pane. `rect_from_drag` and `cover_box` are tested only as functions; nothing draws covers, takes a drag, or calls `AddMask`/`MergeMasks`/`RemoveMask`, and the text↔diagram switch is not built.
- NOT wired: no caller supplies the store callbacks, so `on_save`/`on_attach`/`on_draft`/`on_leave` are exercised only by tests; the shell still has no editor route.

## Task 4.3: what is built, and what is not

- The editor mounts the source and preview, the image action, the field table when a template is in force, the prompt caption, the Save/Discard/Stay question, and the diagram pane (picture, caption, covers placed by `cover_box`, drag through `rect_from_corners`, a group strip whose pick feeds `Merge group` and `Remove`).
- The shell routes: the sidebar selection drives a grid whose tiles open `Route::Editor(id)`; Escape returns. Saving writes `SaveCard` and clears `SetDraft`; each keystroke writes `SetDraft` without re-publishing the snapshot, so typing does not rebuild the editor; masks write `AddMask`, `MergeMasks`, `RemoveMask`. Attach copies through `editor::attach_image` into the library's attachments folder.
- Tests that prove it: editor suite (28) covers the panes and answers; the shell suite opens a card from its tile, returns on Escape, and drags a rectangle on a diagram card then reads the row back from the library file (6).
- Closed since the first pass:
  - The text↔diagram switch. An empty text card offers "Make it a diagram": the picker chooses a file, `attach_image` copies it, and `SetDiagram` (which refuses a card holding text) makes it a diagram. A diagram offers "Clear the diagram" behind a question; `ClearDiagram` deletes the picture reference and every rectangle, retires the prompts and leaves the history, turning the card into empty text. Tests: `an_empty_card_offers_the_diagram_switch_and_a_written_one_does_not`, `the_diagram_clears_only_after_the_question`, and the shell's `clearing_a_diagram_turns_the_card_into_text_in_the_library`, which reads the stored kind back.
  - Removing a whole group. `Command::RemoveMaskGroup { card, group }` deletes every rectangle of the group and syncs prompts; `a_cover_says_which_group_it_belongs_to` and the pane's `Remove group` button (reporting the picked group) cover it, and the core test `removing_a_group_takes_every_rectangle_in_it_and_keeps_the_history` proves the reviews stay readable.
  - Covers stating their group. Each cover is `accessible` with the label `rectangle in group N`, so it enters the accessibility tree; `a_cover_says_which_group_it_belongs_to` counts both covers by that label.
- The cursor clause is met: `ui::source::source_handle()` makes a `ComponentRef<Editor>`, `source_with` puts it on the editor, and `insert_at_caret` types the reference at the caret (falling back to appending only while the editor is not mounted). `a_reference_is_typed_at_the_caret_not_appended` presses a button that inserts into a source opened with its caret at 0 and sees `!xyz`.
- 4.3 is ticked. Two behaviours worth knowing:
  - While the code editor holds the keyboard it consumes Escape itself, so the header's Back control is the path that always asks; Escape asks too whenever the editor is not focused. `the_way_back_asks_while_the_source_holds_the_keyboard` proves the header path. The open question is dismissed by the platform's back press (`Headless::back`, which is what the platforms send); `escape_dismisses_the_leave_question_without_leaving` proves that, and nothing in the screen binds Escape while the question shows.
  - Discard clears the stored draft (`Actions::discard` → `SetDraft { text: None }`), and `a_draft_survives_a_restart_until_it_is_discarded` types, stays, relaunches to see `Unsaved`, discards, and relaunches to see the saved card.

## Task 6.3 and 6.5

- **6.5**: the pre-split fixture — a library written before Rune was removed — was copied to `/tmp/typr-legacy` and opened by the NSH binary. The log line `library /tmp/typr-legacy/typr.sqlite3 schema 3 -> 12, 9 migrations` is the migration running on a Rune-era file; the session started with `s`, revealed with Space, graded with `j`, and the database kept the old review row and appended the new one under the same `fwd` key, with `reps` 1 → 2.
- **6.3**: `ci.yml` was already updated for GTK and xvfb. Making its clippy step pass took 124 findings: `cargo clippy --fix` handled the mechanical ones (97 redundant closures, identity maps, collapsible `if`s), and by hand: `&PathBuf` → `&Path` in `paths.rs`, type aliases for the boxed-closure shapes (`DraftReport`, `MergeMasks`, `Commands`), a saturating column count, and `try_from` in place of two truncating casts. Every step that runs on macOS passes locally; `nsh test --target linux` and the CI run need a push.

## Task 6.3: what CI needed beyond the workflow's first draft

- **The CLI is not on the runner.** nsh.dev's install script puts a release in `~/.nsh` and links `~/.local/bin/nsh`; the step pins `NSH_VERSION=v0.5.1` and `~/.nsh` is cached by version. Without it `nsh test` was a command-not-found.
- **The Linux host links GStreamer.** `nsh-platform-linux` builds `gstreamer-sys`, so clippy stopped at its pkg-config probe. `libgstreamer1.0-dev` and `libgstreamer-plugins-base1.0-dev` join `libgtk-4-dev`; the step checks both `gtk4` and `gstreamer-1.0`.
- **The runner's exit status is not `std::process::ExitCode` on Linux.** The GTK host returns `glib::ExitCode`, so `main` failed to compile there with E0308. A cfg'd `finish()` converts it.
- **The pinned version moved to 0.5.1**, the release the installed CLI reports and CI installs, so the app and its CLI cannot drift.
- **`nsh test --target linux` cannot pass here and is not needed.** It failed with `error no test files in .: tests are *.test.tsx / *.test.ts files importing nsh/test, and the crates' Rust tests run without --target`. This app has no TSX tests, so the crates' Rust tests — already run on the Linux runner by `cargo test --workspace --locked` — are the whole of what it would have executed. The docs also say headless tests need no display, so the step and xvfb were removed together; GTK and GStreamer stay because building `nsh-platform-linux` needs their dev packages.
