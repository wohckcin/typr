# Design — app-launch

## Context

**The ambient read.** `crates/typr/src/bin/typr_nsh.rs:11` is the first statement of `main`:

```rust
match typr::paths::data_root() {          // bin/typr_nsh.rs:11
    Ok(root) => match typr::logging::install(…)
```

`data_root` (`crates/typr/src/paths.rs:26-33`) returns `TYPR_DATA_DIR` if set, and otherwise calls `data_dir()` —
`nsh_core::element::ambient::data_dir`, which reads a signal through `element::reactive::graph`
(`nsh-core-0.5.1/src/element/reactive.rs:164`). `run_app` is not called until later, so the runtime that signal
needs does not exist. That the panic depends on the environment is the proof of cause: with `TYPR_DATA_DIR` set it
does not occur, and the second panic does.

**The disposed prop.** With `TYPR_DATA_DIR=/tmp/typr-review`, the engine thread panics at
`nsh-core-0.5.1/src/element/prop.rs:156` (`a prop read or written after its owner was disposed`) through:

```
<Prop<bool>>::get_untracked → <Prop<bool>>::__update_from
<nsh_base::button::ButtonProps as Props>::update_from
<OnceInstance<nsh_base::button::Button> as AnyInstance>::set_props
```

A `Prop<bool>` a base `Button` holds was created in a scope that has since been disposed — the screen that built it
re-rendered. `ui/choice.rs:99-103` describes the same failure and works around it once.

**Why `nsh run` hides both.** The development host owns the mount and the rebuild cycle, so neither ordering is
exercised. The README's primary command is the one that breaks.

**The two shells.** `shell_over` (`shell.rs:184`) takes `Rc<Db>`, creates the route, the sidebar, the query, the
selection and nine library key bindings (`.448-530`), and draws the library inline. `shell_with` (`:115`) takes no
store, creates the same signals (`:124-140`), binds only Escape (`:146`), and draws its own copy of the field and
grid (`:160-180`). `screens/library.rs::library()` (`:40`) composes the same three elements with two signals of its
own; its only callers are `tests/appearance.rs:130` and `tests/library.rs:46`. `App()` (`:817`) chooses between the
two shells and the tests take neither branch.

## Goals

- `cargo run -p typr` draws the library, and so does `nsh run`, with no `TYPR_DATA_DIR`.
- A missing or unwritable data folder is a message, not a panic and not a blank window.
- One function draws the library; the appearance pass renders that function.
- The two rules are held by tests that fail on the same panic or the missing drawing, not by comments.
- No new dependency and no new abstraction: calls move, ownership narrows, a copy is deleted.

## Non-Goals

- Changing `nsh.json`, the app id, or the platform hosts.
- A general component-ownership framework: `ui/choice.rs`'s pattern becomes the rule, nothing generalises it.
- Windows: the binary has `macos` and `linux` arms and this change adds no third.
- Changing what a closed library shows: the empty snapshot's grid stays, because `:130`'s reason (a blank work
  area leaves the reader guessing) still holds.

## Decisions

1. **Resolve the log and the data folder inside the first mount.** `paths::data_root` keeps its signature and is
   called by the app root after the engine exists; the log is installed there too. The binary keeps only `run_app`
   and the exit code. Rejected: an "is there a runtime?" check around the early call — two ways to read one fact,
   with the panic one call away.
2. **A control's props are built by the component that owns them.** `ui/button.rs` already constructs a `Button`
   inside a `#[component]`; the failure comes from a screen passing a closed-over `Prop` into a control built
   elsewhere. Every `nsh_base` control a screen builds is audited and the prop creation moves into the owning
   component; `ui/choice.rs:99-103`'s comment becomes a pointer to the test.
3. **The test boots the real root.** A headless test constructs `shell::App`, renders, changes a state, renders
   again, and fails on `prop read or written after its owner was disposed` and on `no reactive runtime`.
4. **`shell_over` absorbs `shell_with`.** Its first parameter becomes `Option<Rc<Db>>`; `library_open` derives from
   it, and only actions that need the store branch. Rejected: keeping `shell_with` and adding the missing bindings —
   a second implementation of the same key handling, which is how the two drifted.
5. **`screens::library::library()` is deleted, not adopted.** It was the third copy and no production path called
   it, so deleting it removes the drawing rather than promoting it; the shell's library and its `cards_in` /
   `tiles_for` logic keep one screen, and `tests/appearance.rs` and `tests/library.rs` render `shell(..)`, which
   needs no store. Implementation showed the promotion would have kept three state sets alive for one screen.
6. **Keys travel with the drawing.** The library's bindings are created where the library is composed, guarded by
   the route as `in_library` does today (`:444`), so no second drawing can silently lack them. A maintenance
   guarantee, not a new capability: the reachable closed state has no cards (`:830`, `:836`), so nothing is gained
   by navigating it.
7. **Shell tests render states `App()` can produce.** `tests/shell.rs:42` and `:85` build `shell(fixture, true)` —
   cards through the copy that binds only Escape, a combination production never reaches. The fixture case moves to
   the store-fed shell or is dropped; no test keeps an unreachable state.
8. **The README states the command the test uses.** If `cargo run` is documented, the test exercises its code, not
   only `nsh run`'s.
