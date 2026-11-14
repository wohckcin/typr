# Tasks — test layout

## 1. Split the binary's tests

- [x] 1.1 Turn `crates/typr/src/tests.rs` into `crates/typr/src/tests/mod.rs` plus
      `session.rs`, `library.rs`, `editor.rs` and `diagnostics.rs`, leaving every fixture in `mod.rs`.
  - **Verification**: `cargo test --bin typr` runs the same test names as before this change (counted, not
    eyeballed): the total is unchanged and `cargo test --bin typr session::` selects only session tests.

## 2. Fold the small modules' tests in

- [x] 2.1 Delete `crates/typr-core/src/{clock,day,shuffle,sides}/tests.rs` and carry their tests inline behind
      `#[cfg(test)] mod tests { … }` in each module.
  - **Verification**: `cargo test -p typr-core` still passes and `find crates/typr-core/src -name tests.rs`
    lists every remaining sibling with more code than test.

## 3. Prove nothing was lost

- [x] 3.1 Compare the test count before and after the move and run the whole gate.
  - **Verification**: `cargo test --workspace --locked` reports the same number of passing tests as the run
    taken before task 1.1, `cargo fmt --check` is clean, and
    `cargo clippy --workspace --all-targets --all-features -- -D warnings` reports nothing.

## Workflow follow-up

- [x] Archive this change once every task is ticked and the gate is green.
