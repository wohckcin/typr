# Design — use-base-toast

## Context

`shell.rs:270` owns `result: Signal<Option<SharedString>>`; `export` and `import` (`shell.rs:346-382`) set it, and
`diagnostics(..)` draws it through `ui::overlay::toast`. The base toast is headless storage (`ToastManager`), an
unstyled root (`toast`) and a stack that lays toasts out (`toast_stack`); the app's look goes in the `ui` layer.

## Decisions

1. **The manager lives in `ui::overlay`, behind one small API.** `ui::overlay` exposes `Toasts` (a `stored`
   `ToastManager<u64, SharedString>`, the visible list signal and a `ToastStackState`) with `push(text)` and the
   `toasts(&Toasts)` element. Screens never import `nsh_base`, which `tests/tokens.rs` enforces.
2. **One timeout.** `ToastOptions::timeout(Some(..))` with one duration. `TOAST_MS` is 360 ms today, which is a
   reveal duration, not a reading time; the timeout is a new constant sized for reading (about 4 s), and
   `TOAST_MS` is deleted.
3. **The lifecycle clock.** Follow the base example: `set_interval` advances the manager while toasts are
   mounted, paused while the stack is expanded. The interval is owned by the component, so it stops when the
   diagnostics screen leaves.
4. **Style every part.** The toast surface takes the popup surface already in `ui/overlay.rs` (`popup()`):
   popover fill, border, radius, `p_3`, `text_sm` in `foreground`. An unstyled part draws nothing (NSH skill,
   "Parts draw nothing until styled").

## Risks

- The stack is positioned by the app, not by `placement`, which only says which edge it grows from: rendered
  inline it drew as a card under the Import / Export buttons. `toasts(..)` therefore places it in an
  `overlay()` fixed to the window, bottom centre, which takes no press (`PointerEvents::None`) so the buttons
  under it stay usable; checked on the running app in light and dark alike.
- `set_interval` at 50 ms while a toast shows is negligible at one user and one screen.

## Out of scope

An app-wide toast host. Only the diagnostics screen reports a result today; a second caller moves the host up to
the shell.
