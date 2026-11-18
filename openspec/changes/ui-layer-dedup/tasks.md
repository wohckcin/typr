# Tasks — ui-layer-dedup

## 1. Shell

- [x] 1.1 Add `write(command, republish)` in `crates/typr/src/shell.rs`; make `apply`, `quiet_apply` and the grade
      closure (`:586-607`) call it.
      Verification: `grep -c 'SLOW_COMMAND_MS' crates/typr/src/shell.rs` is 2 (the const and its one use);
      `cargo test -p typr` passes.
- [x] 1.2 Test "A slow grade is recorded": a grade slower than `SLOW_COMMAND_MS` shows on the diagnostics screen.
      Verification: the test fails on the code before 1.1 and passes after it.
- [x] 1.3 One `library_view()` closure, drawn by `Screen::Library` and by the editor's missing-card branch.
      Verification: `grep -c 'field_reachable("Search cards"' crates/typr/src/shell.rs` is 1.

## 2. ui layer

- [x] 2.1 Delete `SubmitField` (`ui/input.rs:91-139`); move the shared style into a private `styled(..)`.
      Verification: `grep -rn SubmitField crates` finds nothing.
- [x] 2.2 Fold `source` into `source_with` with an optional `on_change`; update its callers.
- [x] 2.3 `Keycap` and `Badge` share one body in `ui/status.rs`.
- [x] 2.4 Delete `skeleton`, `spinner_label`, `separator`, `hint`, `PRESS_MS`.
      Verification: `grep -rnw 'skeleton\|spinner_label\|separator\|hint\|PRESS_MS' crates/typr/src` finds no
      definition and no caller.
- [x] 2.5 Inline `tree::item`, `tree::Items` and `tokens::flush` at their callers (see design Risks for `flush`).

## 3. Proof

- [x] 3.1 `cargo clippy -p typr --all-targets` and `cargo test --workspace` pass with no test edited except imports.
- [x] 3.2 Screenshot the library, the editor and settings in light and dark; nothing moved.
      Verification: screenshots of the three screens, each appearance, from this tree and from `d469680` (before
      the change): all six pairs pixel-identical (0 differing pixels of 1423×2400).
