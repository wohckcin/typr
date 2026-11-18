# Tasks — use-base-toast

## 1. The base toast in the ui layer

- [x] 1.1 Replace `ui::overlay::toast` (`crates/typr/src/ui/overlay.rs:80-97`) with `Toasts` + `toasts(..)` built
      on `nsh_base::{toast_stack, toast, ToastManager, ToastOptions, ToastStackState}`, as in
      `nsh-base-0.5.1/src/toast/mod.rs`'s example; style the surface with `popup()`.
      Verification: `cargo check -p typr --all-targets`.
- [x] 1.2 Delete `TOAST_MS` (`ui/overlay.rs:100`); add the reading timeout as one constant beside it.
      Verification: `grep -rnw TOAST_MS crates` finds nothing.

## 2. The screen pushes a result

- [x] 2.1 `shell.rs`: `export` and `import` push their line to `Toasts` instead of `result.set(Some(..))`;
      `diagnostics(..)` takes `Toasts`.
      Verification: `grep -n 'result' crates/typr/src/shell.rs` names no `Signal<Option<SharedString>>`.

## 3. Proof

- [x] 3.1 Headless test `a_result_appears_then_leaves` in `crates/typr/tests/diagnostics.rs`: push a result,
      `wait_for` its text, advance past the timeout, assert it is gone.
      Verification: the test fails on today's code (the line never leaves) and passes after 2.1.
- [x] 3.2 Screenshot the diagnostics screen with a result showing, light and dark; the toast does not cover the
      buttons. Report the Finishing checklist groups for this surface.
      Verification: the running app, both appearances — the toast floats at the bottom centre, clear of Import
      and Export (a real pointer press on Export still opens its dialog through the overlay), announced as a
      group in the accessibility tree, and Escape dismisses it before its timeout.
- [x] 3.3 `DESIGN.md:84` and `:102` still read true; adjust wording if the timeout or placement differ.
