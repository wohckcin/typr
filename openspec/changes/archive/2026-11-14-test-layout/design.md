# Design — test layout

## Context

Measured in this tree: 25 source files holding 6795 lines, 24 test files holding 7651 lines — tests are 53%
of the code. Seventeen modules declare `#[cfg(test)] mod tests;`. The largest files are
`crates/typr/src/tests.rs` (2216), `crates/typr-core/src/store/db.rs` (957) with its five split test files
(2572), `crates/typr/src/library.rs` (877), `crates/typr/src/main.rs` (1077).

Sampled over the 978 crates vendored under `~/.cargo/registry`, 3812 files use inline `mod tests {` against
165 that declare a sibling `mod tests;`. Rune itself is mixed: 145 sibling, 127 inline.

A sibling file is still a child module, so `use super::*` reaches private items exactly as inline code does.
The two layouts differ in bookkeeping, not in privacy or reachability.

## Goals

- One subject per file, in both directions of the repository.
- No test file that belongs to nothing.

## Non-Goals

- Converting the tree to the ecosystem's inline default: `store`, `library` and `editor` would become single
  files of thousands of lines, which is worse for review.
- Changing any test's assertions, names or fixtures.
- Moving integration tests: `crates/typr-core/tests/` and `crates/typr/tests/` stay where Cargo looks.

## Decisions

### The size of the module decides

A sibling `tests.rs` earns its four boilerplate lines when the module is long enough that a reader wants the
code without the spec. Below that it is a second file to open for no gain. The threshold I applied is the
one the tree already suggests: modules under about 120 lines with test files as long as their code carry the
tests inline (`clock` 58/28, `day` 26/45, `shuffle` 40/41, `sides` 45/45); everything else keeps a sibling. `hidden` (279 with 140 test lines) and `screen` (19
lines with no test file at all) were each checked against that threshold: `hidden` keeps its sibling, and
`screen` has nothing to move.

### A binary's tests are split by subject, not by size

`crates/typr/src/tests.rs` is not too long so much as it is unscoped: it is declared from `main.rs`, so it
describes the whole app and nothing in particular. Splitting it into `session`, `library`, `editor` and
`diagnostics` behind the same `mod tests;` keeps `main.rs`'s declaration unchanged, keeps the shared fixtures
in one place, and makes `cargo test --bin typr session::` a useful filter.

### A child module may not take a screen's name

The four subjects are named `*_flow` because a child named `editor` or `library` shadows `crate::editor`
and `crate::library` inside itself, and every test that writes `library::Host::new_deck(&mut state)` or
`editor::Choice::Discard` then fails with `cannot find Choice in editor` and `trait import Host is
private`. The first attempt at the split failed exactly that way.

### Fixtures stay in the module root

The 15 helpers (`library_with_one_card`, `settle`, `wait_for`, `due_at`, `Stops`, …) move to `tests/mod.rs`
and the four child modules take them through `use super::*;`. Moving each helper to the only child that uses
it would be a second refactor with no reader benefit.

## Risks

- A child module that loses access to an import compiles once and then not again; the mitigation is that
  `tests/mod.rs` keeps every `use` the children need and the children glob-import it.
- Test namespaces change (`typr::tests::a_grade…` becomes `typr::tests::session_flow::a_grade…`), so a saved
  `cargo test <filter>` string may need the module prefix. No CI filter depends on it.
- Inlining four test files makes those modules longer; all four stay under 100 lines.
