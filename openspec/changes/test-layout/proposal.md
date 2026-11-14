# Test layout follows the size of what it tests

## Why

Every test in the repository sits in a sibling `tests.rs`, a convention taken from Rune's own `AGENTS.md`.
It pays off where the module is big — `crates/typr-core/src/store/db.rs` stays 957 lines instead of about
3.5k — and costs four boilerplate lines where the module is small: `crates/typr-core/src/shuffle.rs` is 40
lines and its spec file is 41.

The cost that actually hurts is a third case: `crates/typr/src/tests.rs` holds 2216 lines across 41 tests
and 15 helpers, declared from `crates/typr/src/main.rs` as a crate-root `mod tests;`. It belongs to no
module, so every app-level behaviour lands in it — saving a card, the archived view, the session summary,
the quit record. It is the largest file in the repository and it is a folder pretending to be a module.

## What Changes

- App-level tests split by subject into `crates/typr/src/tests/{session,library,editor,diagnostics}.rs`
  behind the same `#[cfg(test)] mod tests;`, with the shared fixtures left in `crates/typr/src/tests/mod.rs`.
- The four small modules of `typr-core` — `clock`, `day`, `shuffle`, `sides` — move their tests inline into
  the module they test, and their `tests.rs` siblings are deleted.
- Sibling files stay for the modules that earn them: `store`, `session`, `hidden`, `library`, `editor`,
  `tile`, `logging`, `attachments`, `study`, `history`.
- No test is deleted, renamed or weakened: the same 244 tests run before and after.

## Impact

- Affected specs: `project-structure` (one added requirement stating where a module's tests live).
- Affected code: `crates/typr/src/tests.rs` becomes a folder; four `tests.rs` files in `typr-core` are folded
  into their modules; no production line changes.
