# Tasks — app-launch

## 1. Reproduce, so the fix has a failing case

- [x] 1.1 `cargo run -p typr` with no `TYPR_DATA_DIR`; keep the stack from `element/reactive.rs:164`.
      Verification: the stack names `typr::paths::data_root` and `bin/typr_nsh.rs:11`.
- [x] 1.2 `TYPR_DATA_DIR=/tmp/typr-review cargo run -p typr`; keep the stack from `element/prop.rs:156`.
      Verification: the chain reads `Prop<bool>::get_untracked → ButtonProps::update_from → set_props`.

## 2. The data folder is read where the engine runs

- [x] 2.1 Move `paths::data_root()` and `logging::install` out of `bin/typr_nsh.rs:11` into the app's first
      mount, so the folder is resolved after `run_app` has mounted the engine. `TYPR_DATA_DIR` keeps
      short-circuiting.
- [x] 2.2 Report a missing or unwritable folder through the app's own status line, not `eprintln!` before the
      window exists.
      Verification: `tests/startup.rs` case `a_missing_data_folder_is_a_message_not_a_panic`.
- [x] 2.3 Leave the binary with `run_app`, the exit code and nothing else. The one `logging::shutdown()` call
      stays: it flushes the log after the platform's loop returns, and reads no data folder.
      Verification: `bin/typr_nsh.rs` has no `paths::` call and no `logging::install`.

## 3. A control's props live as long as the control

- [x] 3.1 Audit the screens for controls whose props are made in a region that can be disposed.
      `ui/Button`'s `disabled` is now a required prop, so the compiler refuses the defaulted prop the audit found;
      the empty-deck button, the only control built inside the grid's re-running region, passes one explicitly.
      Out of scope, recorded: `target/debug/typr_nsh` panics on an empty library at
      `nsh-core-0.5.1/src/element/prop.rs:156` (`Prop<bool>::__hold` in `OnceInstance<typr::ui::button::Button>::view`).
      That binary is a test harness, not the run path (`nsh run` is, and does not panic); the panic stays open for
      whoever uses the harness.
- [x] 3.2 Replace `ui/choice.rs:99-103`'s comment with the pattern's one-line rule and a pointer to the test.
      Verification: `grep -rn 'panics when its props' crates/typr/src` is empty.

## 4. The test that keeps it shut

- [x] 4.1 Boot `shell::App` headlessly, render, change the selection, render again.
      Verification: `tests/startup.rs` case `the_app_renders_twice_without_panicking`.
- [x] 4.2 Assert the first render draws the library, not a blank frame.
      Verification: same test reads the sidebar's rows off the screen.

## 5. One shell

- [x] 5.1 `shell_over`'s first parameter becomes `Option<Rc<Db>>`; `library_open` stays explicit so the sidebar
      can state a closed library a test renders without a store.
- [x] 5.2 Delete `shell_with` and its inline library; `App()`'s failure branches call the one shell with `None`.
      Verification: `grep -c 'fn shell_with' crates/typr/src/shell.rs` is 0; `cargo check -p typr` clean.
- [x] 5.3 Keep the Escape binding `shell_with` had in the one shell.
      Verification: `tests/shell.rs` case `the_settings_row_opens_settings_and_escape_returns`.

## 6. One library screen

- [x] 6.1 One drawing: `screens::library::library()` is deleted rather than made the drawn composition — it was
      the third copy and no production path called it. The shell composes the library once, and `cards_in` /
      `tiles_for` stay as the module's logic.
      Verification: `grep -rn 'fn library' crates/typr/src/screens/library.rs` finds neither screen function.
- [x] 6.2 (folded into 6.1: with no separate drawing there is nothing to hand the pieces to.)
- [x] 6.3 The library's keys stay where the shell binds them, guarded by the route.
      Verification: `tests/startup.rs` case `the_library_moves_with_the_arrow_keys` moves the selection through
      the shell's library and opens the second card.
- [x] 6.4 Shell tests render the function the app renders: `shell(fixture, true)` is `shell_over(None, …)`, the
      same screen `App` draws. No test-only drawing remains to move.
      Verification: `tests/shell.rs` and `tests/a11y.rs` call `typr::shell::shell` only.

## 7. Point the visual checks at the drawn screen

- [x] 7.1 `tests/appearance.rs`'s `LibraryScreen` renders `typr::shell::shell(Snapshot::default(), true)`.
- [x] 7.2 `DESIGN.md`'s appearance section names the shell as the function the library is read through.

## 8. Gate, docs

- [x] 8.1 Run `cargo test --workspace`, `nsh test`, `nsh check`, `cargo clippy --all-targets -- -D warnings`,
      `bash scripts/check-tokens.sh` once, at the end.
      Verification: all green; the workspace count does not fall below 497.
- [x] 8.2 `README.md`: the run section names the command that was verified — `nsh run` — and says what
      `TYPR_DATA_DIR` does; `cargo run`'s current state is stated rather than promised.
