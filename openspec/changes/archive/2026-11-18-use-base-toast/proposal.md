# Proposal — use-base-toast

## Why

The diagnostics screen states what an import or an export came to through `ui::overlay::toast`
(`crates/typr/src/ui/overlay.rs:80-97`, placed at `crates/typr/src/screens/diagnostics.rs:88`). That "toast" is a
hand-built `when(..)` over a plain view:

- nothing ever clears it: `grep -rn 'result.set(None)' crates` finds nothing, and `TOAST_MS`
  (`ui/overlay.rs:100`) has no reader, so the first result stays on screen for the rest of the run, beside every
  later action;
- it has no entrance or exit, no screen-reader announcement, no Escape and no swipe;
- `DESIGN.md:84` and `:102` say the result line is `nsh/base`'s `toast` with its own entrance and exit, which the
  code does not do. That breaks `native-elements`' "A hand-built element is a recorded decision": the element is
  hand-built and the record says it is not.

`nsh-base` 0.5.1 ships `toast_stack`, `toast` and `ToastManager` (`nsh-base-0.5.1/src/toast/`), with timeouts,
pause on hover, Escape, swipe and the announcement. The NSH Coding Guides ("Rendering and composition") say to
compose from the standard semantic component before building a custom surface, and the Design Guides ("Feedback
and overlays") name this case: a notification is asynchronous status that needs no decision.

## What Changes

- `ui::overlay::toast` becomes a styled `nsh_base::toast_stack` over a `ToastManager`, following the compiled
  example in `nsh-base-0.5.1/src/toast/mod.rs`; the screen pushes a result instead of setting a signal.
- A result leaves after its timeout (`TOAST_MS` becomes the timeout's token, or is deleted if the manager's own
  motion token is used).
- `DESIGN.md:84` and `:102` stay true, now because the code matches them.
- A headless test proves a result appears and then leaves.

## Capabilities

### Modified Capabilities

- `native-elements`: the result line is named as a base element, not a hand-built one.
- `diagnostics`: an import or export result is shown and then leaves.

## Impact

`crates/typr/src/ui/overlay.rs`, `crates/typr/src/screens/diagnostics.rs`, `crates/typr/src/shell.rs` (the
`result` signal becomes a push), `crates/typr/tests/` (one new case). No store or schema change.
