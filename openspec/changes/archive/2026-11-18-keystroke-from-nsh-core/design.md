# Design — keystroke-from-nsh-core

## Context

Callers of the conversion: `shell.rs:508` (`keystroke_of_chord`), `screens/study.rs:127-157` and
`screens/editor/mod.rs:294` (`keystroke_of`), `screens/keymap.rs:152` (`keystrokes_of`, for the palette's
shortcut text). None reads a keystroke back into a chord.

## Decisions

1. **Keep `typr-core`'s `Chord`.** The core crate is tested without NSH and owns conflict checks and `dispatch`
   for the store; moving the table to `nsh_core::keymap::Keystroke` would make the core depend on the UI toolkit.
   The conversion stays at the boundary, in the app crate.
2. **One function maps `typr-core` to `nsh_core`.** `fn keystroke(chord: Chord) -> Option<nsh_core::keymap::Keystroke>`
   maps `Mods` to `Modifiers` and `Key` to `Key` (`Space` → `Key::Char(' ')`), and applies the shift lookup for a
   shifted character. `keystroke_of_chord` is `keystroke(chord).map(|k| k.to_string())`.
3. **Modifier order changes, meaning does not.** Today's text is `cmd+ctrl+alt+shift+k`; `Keystroke`'s is
   `ctrl+alt+shift+cmd+k`. `on_key` parses either order. The palette's shortcut text comes from the same function,
   so the panel and the bindings stay in step.
4. **`mod` is not used.** The table states Command explicitly (`Chord::command`), as today; the string says `cmd`.

## Risks

- A chord that has no `nsh_core` key (none today) returns `None` and is skipped, as now. The round-trip test
  covers every row of `bindings()`.
