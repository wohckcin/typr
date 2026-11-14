# Proposal — keymap, shortcut panel and command palette

## Why

The app is keyboard-driven and says so nowhere. Keys live inside three `match` arms — `State::on_key` in
`crates/typr/src/main.rs`, `study::key`, and a handful of `if` chains — so the set of bindings cannot be listed,
cannot be checked for conflicts, and cannot be shown next to the action that uses it. The session screen prints
its own hand-written row of hints, which is a fourth copy of the same facts.

Mochi treats the keyboard as an interface: every action carries its keycap, `?` opens a panel of bindings
grouped by surface with a search box, and `Cmd+K` opens a palette that runs the same commands. Measured on the
running app: the panel's sections are "Editor", "Deck page", "List view (select mode)", "Notebook view",
"Notebook view (edit mode)", "Column view", "Review page" and "Miscellaneous"; the sidebar's search row is
labelled `Search` with a `Cmd K` keycap.

## What Changes

- One table declares every command, its chord and the surface it applies to; dispatch, keycap hints and the panel
  all read that table.
- `?` opens a searchable panel of bindings grouped by surface.
- Actions draw their keycap next to their label, on buttons and in the session's binding row.
- `Cmd+K` opens a command palette over the commands valid on the current surface; typing filters, Enter runs.
- A test rejects two bindings for the same chord on the same surface.

## Capabilities

### New Capabilities

- `keymap`: the single table, the panel, the palette, and the rule that no binding lives outside the table.

### Modified Capabilities

None. `study-session`'s "The session is fully keyboard-operable" keeps requiring that bindings are displayed;
this change decides where they come from.

## Impact

- New `crates/typr-core/src/keymap.rs`: `Command`, `Surface`, `Binding`, `bindings()`, `chord_of`, and the
  conflict check as a plain function the tests call.
- `crates/typr/src/keymap/panel.rs` and `crates/typr/src/keymap/palette.rs`: the two surfaces that render it.
- `crates/typr/src/main.rs`, `study.rs`: the three `match` arms become `keymap::dispatch` plus one executor per
  surface.
- `crates/typr/src/library.rs`, `editor.rs`, `study.rs`: buttons gain a keycap derived from the same table.
- No schema change; no behaviour change to scheduling or storage.

## Dependencies

`deck-views` asks this change for its bindings, and `card-grid` draws tiles that carry these keycaps.
