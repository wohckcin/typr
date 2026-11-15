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
- Queue rules (the cap, forgot cycling, cram not writing a schedule) are typr-core behaviour; typr-core's own tests cover them.
