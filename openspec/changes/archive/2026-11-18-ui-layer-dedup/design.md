# Design — ui-layer-dedup

## Decisions

1. **`write(command, republish: bool) -> Result<Outcome, StoreError>`.** It times the command, records a slow one,
   sets `stopped` and logs on error, and re-reads the snapshot when `republish` is true. `apply` is
   `write(c, true)`, `quiet_apply` is `write(c, false)`, and the grade path maps `Err` to `GradeRefused`. Keeping
   the two names as thin closures is fine; the logic exists once.
2. **The grade path records slow grades too.** Today it skips the timing. Routing it through `write` makes a slow
   grade show on the diagnostics screen, which is what "A slow command is recorded with its cost" already says.
3. **`library_view()` is a local closure in `shell_over`**, capturing `query`, `search_handle`, `tiles`,
   `selection`, `on_new` and `open`, with the gap value of the `Screen::Library` branch (`gap_3`). The editor's
   fallback had `gap_2`; the difference was accidental.
4. **One field style, private.** `fn styled(input: Input, size: Size) -> Input` in `ui/input.rs`. Not public: screens
   still place `field()` / `field_reachable`, so no screen styles a control.
5. **No new abstraction for button press states.** `Button`, `Tile` and `ListRow` repeat the focus ring and the
   press branch; that is left for `motion-coverage` or a later change, because their fills differ on purpose.

## Risks

- Inlining `tree::item` changes `sidebar.rs:150`'s import only. Inlining `tokens::flush` adds a literal
  `px(0.0)` to `screens/editor/diagram.rs:202`; check `tests/tokens.rs` does not flag a zero inset, and keep
  `flush` if it does.
