# Proposal — app-launch

## Why

`cargo run -p typr`, the first command in the README's "Build and run", opens a blank window and panics — twice,
in sequence:

1. `crates/typr/src/bin/typr_nsh.rs:11` calls `typr::paths::data_root()` before the platform mounts the engine.
   Without `TYPR_DATA_DIR` that falls through to `nsh_core::element::ambient::data_dir()`
   (`crates/typr/src/paths.rs:33`), which reads a signal with no reactive runtime: `panicked at
   nsh-core-0.5.1/src/element/reactive.rs:164:9: no reactive runtime: signals are used on the engine thread, while
   components run`.
2. With the first panic bypassed (`paths.rs:26` short-circuits on `TYPR_DATA_DIR`), the engine thread panics at
   `nsh-core-0.5.1/src/element/prop.rs:156: a prop read or written after its owner was disposed`, reached through
   `nsh_base::button::ButtonProps::update_from` from `OnceInstance<Button>::set_props`. A `Prop<bool>` handed to a
   base `Button` outlives the scope that built it; `crates/typr/src/ui/choice.rs:99-103` documents the same hazard
   in a comment and avoids it in that one component.

Both reproduce on every attempt (3/3 and 2/2). `nsh run` renders because the development host mounts the engine
first.

Behind the same root, the library is drawn three times:

| Drawing | Where | Drawn by the app? |
|---|---|---|
| inline sidebar + search + grid | `crates/typr/src/shell.rs:184-300` (`shell_over`) | yes, whenever the library opens |
| inline copy | `shell.rs:115-183` (`shell_with`) | yes, when it does not |
| `screens::library::library()` | `crates/typr/src/screens/library.rs:40-77` | **no** — only `tests/appearance.rs:130` and `tests/library.rs:46` call it |

So `DESIGN.md`'s light/dark proof asserts against a function no production path draws; the shell the app reaches
with no library binds only Escape (`shell.rs:146`) while `shell_over` binds nine library commands (`:448-530`); and
`tests/shell.rs:42` and `:85` render `shell(fixture, true)`, which resolves to `shell_with(fixture, true, …)` — cards
in the copy that has no keys, a combination `App()` never produces (`:830`, `:836` pass `Snapshot::default()` and
`false`).

## What Changes

- The binary resolves the data folder and installs the log from inside the app, after the engine exists.
- Every screen that hands a base control a prop creates it in the component that owns the control.
- A test boots the app root twice and fails on either panic.
- One shell: `shell_over` takes `Option<Rc<Db>>` and draws the library for both cases; `shell_with` and its copy go.
- One library screen: the shell draws `screens::library::library()`, so the tests' subject is the app's subject.
- The library's keys are bound where the library is composed, so no second drawing can lack them.
- Shell tests render only argument pairs `App()` can pass; the README's run command is the one the test exercises.

## Capabilities

### New Capabilities

- `app-startup`: the app draws its first screen from each documented run path, reads the facts the engine owns
  where the engine runs, and keeps every control's props alive as long as the control.
- `shell`: the app root — which screen the work area shows, how an open and a closed library differ, and that a
  screen exists once.

### Modified Capabilities

None.

## Impact

- `crates/typr/src/bin/typr_nsh.rs`, `paths.rs`, `logging/mod.rs`: who resolves the data folder, and when.
- `crates/typr/src/screens/*`, `ui/button.rs`: which scope owns a base control's props.
- `crates/typr/src/shell.rs`: ~70 lines deleted, one parameter added; `shell_with` absorbed.
- `crates/typr/src/screens/library.rs`: becomes the library the shell draws rather than a test fixture with the
  same name.
- `crates/typr/tests/`: a startup test, a closed-library-free shell test set, the appearance pass on the drawn
  screen. `README.md`: the run instructions the test proves.
