# Design — module folders

## Context

Rust accepts both `foo.rs` with `foo/` beside it (its 2018 layout) and `foo/mod.rs` with `foo/`. The tree
already used both: `crates/typr-core/src/store/mod.rs` and `crates/typr-core/src/store/tests/`, against
`crates/typr-core/src/hidden.rs` and `crates/typr-core/src/hidden/tests.rs`.

Eleven modules carried a folder holding nothing but `tests.rs`, and `crates/typr/src/screen/` was an empty
directory left over from a split.

## Goals

- One shape: a module with a folder has its root inside it.
- No directory that exists only to hold tests.

## Non-Goals

- Converting `crates/typr/src/store`-style folders that already hold several subjects: they are unchanged.
- Renaming files, editing content, or moving `tests/` integration directories that Cargo owns.

## Decisions

### The root moves, the tests stay

`foo.rs` becomes `foo/mod.rs`; `foo/tests.rs` keeps its path and its `#[cfg(test)] mod tests;` declaration.
That keeps every `mod` statement, `use` path and `#[path]`-free declaration working, so the whole change is
eleven `git mv`s plus one `rmdir`.

### `mod.rs` is the price of the rule

The 2018 style guide prefers `foo.rs` beside `foo/`. That preference assumes the folder holds submodules a
reader needs anyway; here the folder usually holds tests, so a folder whose root is elsewhere reads as two
half-modules. Consistency inside this tree matters more than matching the guide, and the choice is recorded
here so a future reader does not "fix" it back.

## Risks

- Reviewers who prefer `foo.rs` + `foo/` will read the move as churn; the spec requirement states the rule,
  which is the answer.
- A future submodule added to one of these modules now lands in a folder already named for it, which is the
  intended outcome rather than a risk.
