# Proposal — settings screen

## Why

Every clone change so far added settings with no way to reach them: the new-card cap, the reverse switch,
the scheduler, the retention target and the FSRS weights all live in the `settings` table, and the only
way to change one is a test. Mochi has a settings screen with grouped preferences, and a clone that hides
its own switches is not a clone.

The app also has no way to see where its library lives, what schema it is at, or where its log went,
which is the first thing anyone asks when something looks wrong.

## What Changes

- A fifth screen: Settings, reachable from the library and from the session, grouped into Review,
  Appearance, and Library.
- Review: new cards per day, learning step, maximum interval, scheduler, retention, weights, reverse
  default.
- Appearance: dark or light, interface scale, preview font size.
- Library: the database path, its schema version, its size, the attachments folder, the log path, and
  the import and export actions.
- Every value is validated on entry and refused with its reason; a refused value leaves the stored one.
- Settings are stored in the database, so they travel with a library rather than with a machine.

## Capabilities

### New Capabilities

- `preferences`: what a setting is, how it is validated, where it lives, and what the screen must show.

### Modified Capabilities

None.

## Impact

- `crates/typr-core/src/store/mod.rs`: `Settings` grows from one field to the full set, with per-key
  validation and defaults; `Command::SetSetting` accepts a typed key rather than a free string.
- `crates/typr-core/src/store/db.rs`: reading and writing each key, refusing an invalid value.
- New `crates/typr/src/settings.rs`: the screen, its sections and its controls.
- `crates/typr/src/screen.rs`: `Screen::Settings`.
- `crates/typr/src/library.rs`: a control that opens it; appearance values reach the theme.
- No dependency changes.
