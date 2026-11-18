# Proposal — keystroke-from-nsh-core

## Why

The bindings table lives in `typr-core` (`crates/typr-core/src/keymap/mod.rs`) as the app's own `Chord`, `Key` and
`Mods`, which keeps the core crate free of NSH. The app crate then turns a `Chord` into the string `on_key` takes,
by hand, in `crates/typr/src/screens/keymap.rs`:

- `keystroke_of_chord` (`:128-150`) names every key (`"escape"`, `"left"`, ...) and `char_keystroke` (`:88-112`)
  joins modifiers with `+`. `nsh_core::keymap::Keystroke` already does both: its `Display` writes the canonical
  `ctrl+alt+shift+cmd+<key>` text (`nsh-core-0.5.1/src/keymap.rs:665`) with the same key names `on_key` parses.
- `chord_from_keystroke` (`:15-52`, about 40 lines) goes the other way, and its only callers are in
  `crates/typr/tests/keymap.rs`. Keys arrive through `on_key` handlers bound per chord, so production never parses
  a keystroke back into a chord.
- `shift_base` (`:55-86`) maps a shifted character to its key. It calls `Box::leak` for every capital letter.
  No binding uses a capital letter today, so it does not run yet; it leaks memory the day one is added.

`nsh-core` keeps its own shifted-character table private (`SHIFTED`, `keymap.rs:332`), and `Keystroke::parse`
refuses `?` in favour of `shift+/`. So the app keeps a small shift table, as a `const` slice with no leak; what
it stops writing by hand is the string itself.

## What Changes

- `keystroke_of_chord` builds an `nsh_core::keymap::Keystroke { modifiers, key }` and returns `to_string()`.
- `char_keystroke` and the per-key string arms are deleted; `shift_base` becomes a `const` lookup with no
  `Box::leak` (capital letters map to their lower case with Shift).
- `chord_from_keystroke` and its three test assertions are deleted.
- A test proves every chord in the table becomes a string `Keystroke::parse_with` accepts and that parses back to
  the same keystroke.

## Capabilities

### Modified Capabilities

- `keymap-panel`: the table's chords reach `on_key` as NSH's own keystroke text.

## Impact

`crates/typr/src/screens/keymap.rs`, `crates/typr/tests/keymap.rs`. `typr-core` is unchanged and gains no NSH
dependency. Net about -60 lines.
