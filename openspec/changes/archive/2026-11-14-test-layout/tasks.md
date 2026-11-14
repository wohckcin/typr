# Tasks — test layout

## 1. Split the binary's tests

- [x] 1.1 Turn `crates/typr/src/tests.rs` into `crates/typr/src/tests/mod.rs` plus `session_flow.rs`,
      `library_flow.rs`, `editor_flow.rs` and `diagnostics.rs`, leaving every fixture in `mod.rs`. The suffix is
      load-bearing: `tests::editor` and `tests::library` shadow `crate::editor` and `crate::library` from inside
      themselves.
  - **Verification**: `cargo test --bin typr` runs the same test names as before this change (counted, not
    eyeballed): the total is unchanged and `cargo test --bin typr session_flow::` selects only session tests.

## 2. Fold the small modules' tests in

- [x] 2.1 Delete `crates/typr-core/src/{clock,day,shuffle,sides}/tests.rs` and carry their tests inline behind
      `#[cfg(test)] mod tests { … }` in each module. `hidden` (279 lines, 140 test lines) and `screen` (19 lines,
      no test file) were checked against the same threshold: `hidden` keeps its sibling, `screen` had nothing to
      move.
  - **Verification**: `cargo test -p typr-core` still passes and `find crates/typr-core/src -name tests.rs`
    lists every remaining sibling with more code than test.

## 3. Prove nothing was lost

- [x] 3.1 Compare the test count before and after the move and run the whole gate.
  - **Verification**: `cargo test --workspace --locked` reports the same number of passing tests as the run
    taken before task 1.1, `cargo fmt --check` is clean, and
    `cargo clippy --workspace --all-targets --all-features -- -D warnings` reports nothing.

## Workflow follow-up

- [x] Archive this change once every task is ticked and the gate is green.
