# Proposal — settings-appearance

## Why

`preferences` requires that a setting's change "takes effect without a restart or an edit to the file by hand",
and names an interface scale as an example (`openspec/specs/preferences/spec.md:55`). The library stores and
validates both affected keys — `Theme` (`crates/typr-core/src/store/settings.rs:35`) and `Scale` with
`MIN_SCALE = 0.80`, `MAX_SCALE = 1.50` (`:25-27`, validated by `in_scale` at `:410`) — but:

- the screen's Appearance group holds one control, "Show review counts" (`crates/typr/src/screens/settings.rs:213-217`);
  there is no theme control and no scale control;
- `grep -rn '\.theme\|\.scale\|Theme::' crates/typr/src` finds only the press scale in `ui/button.rs`; the app
  never reads either setting;
- appearance is owned by `nsh.json` (`"appearance": "system"`), so a reader who sets `theme=light` in the
  library gets no change at all.

The result is a setting that validates, round-trips through the store, and does nothing — and a visual review
that cannot be run in the appearance the app claims to support, because the window follows the OS.

## What Changes

- The Appearance group gains a control for the stored theme, and the app applies the stored theme at launch and
  on change, through the appearance the engine already exposes.
- `theme` becomes the setting the window follows; `nsh.json`'s `appearance` stops deciding it.
- `Theme` gains `System`, so the control offers System / Light / Dark.
- The interface scale leaves the promise: `preferences` stops naming it and `SettingKey::Scale` leaves the store,
  rather than shipping a setting nothing applies.
- A test proves the stored theme is the one the first frame resolves, and that a change repaints without a restart.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `preferences`: "Settings are reachable from the app" widens to name the Appearance group's controls and the
  guarantee that the stored appearance is the one the app draws in.

## Impact

- `crates/typr/src/screens/settings.rs`: the Appearance group.
- `crates/typr/src/shell.rs`, `crates/typr/src/bin/typr_nsh.rs`: applying the stored theme at launch.
- `crates/typr-core/src/store/settings.rs`: `Theme` gains `System`; `SettingKey::Scale`, `MIN_SCALE`, `MAX_SCALE`
  and `in_scale` are removed with the promise they served.
- `nsh.json`, `README.md`: the appearance key and what it now decides; the settings table loses its scale row.
- `tests/appearance.rs`: the light/dark pass becomes reachable from the app rather than only from a test config.
