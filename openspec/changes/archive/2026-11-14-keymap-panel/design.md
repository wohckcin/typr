# Design — keymap, panel, palette

## Context

Today the bindings are spread over `State::on_key` in `crates/typr/src/main.rs` (library `s`/`n`/`h`, editor
`Escape`, save `s`), `crates/typr/src/study.rs::key` (Space/J/F/Left/Right/Esc/R) and a hand-written
`BINDINGS` row that prints them. Rune delivers key events to a focused element and to the app's `on_key`, and
`Key` has no `Space`/`ArrowRight` variants: space arrives as `Key::Text(" ")`, arrows as `Key::Left`/`Key::Right`.

Mochi's panel is the model: sections per surface, each a list of label-and-chord rows, with a filter box, and a
`Cmd+K` palette that runs the same commands.

## Goals

- One declaration site; dispatch, hints and the panel all derive from it.
- A conflict is a failing test, not a runtime surprise.
- The palette and the keyboard offer the same commands, so neither can drift.

## Non-Goals

- No user-remappable keys in this change.
- No chord sequences (prefix keys) and no modal editing.
- No audio bindings; the app has no audio.

## Decisions

### The table is data in `typr-core`, not code in the views

`pub enum Surface { Library, Editor, Session, Summary }`, `pub enum Command { NewCard, EditCard, ArchiveCard,
Reveal, GradeRemembered, GradeForgot, Shuffle, ToggleHistory, Leave, Save, ShowShortcuts, ShowPalette, … }` and
`pub struct Binding { pub command: Command, pub surface: Surface, pub chord: Chord, pub label: &'static str }`
with `pub fn bindings() -> &'static [Binding]`. It lives in core because it is pure data with no toolkit types,
so the conflict check is an ordinary test and `deck-views` can add entries without touching views.

### `Chord` is our own type, not `rune::Key`

`Chord::Key(Key)` and `Chord::Modified { key, mods }` with `fn from_event(&KeyEvent) -> Chord` implemented in the
desktop crate, where `rune::Key` is available. Keeping `rune::Key` out of core preserves
`project-structure`'s rule that core names no toolkit crate, and it gives the panel something printable without
converting back from the toolkit.

### Dispatch returns a command; surfaces execute it

`fn dispatch(surface: Surface, chord: Chord) -> Option<Command>` replaces the three `match` arms. Each surface
keeps one `fn run(&mut self, command: Command) -> bool` because the execution needs state; the read-only screens
(library, summary) pass a command to `State`, the session passes it to `Session`. A `match` inside `run` is
unavoidable and fine: what the table removes is the duplication of *which key means what*.

### The panel is one list built from the table, filtered by a `TextEdit`

`crates/typr/src/keymap/panel.rs` builds sections by grouping `bindings()` on `surface`, renders each row as a
label with `rune_kit::kbd` chips, and filters with the existing folded matcher over `label`. It is a
`rune_kit::Dialog` with `.fit()`, opened by `ShowShortcuts` and closed by the same command, so no new modal
machinery is introduced.

### The palette reuses the same commands and the same filter

`crates/typr/src/keymap/palette.rs` lists `bindings().iter().filter(|b| b.surface == current)`, shows the label
and the chord, keeps a highlighted index, and runs the chosen command through the same `run` the keyboard uses.
A separate command registry was rejected: two registries is exactly the drift this change exists to remove.

### The conflict check is a function the test calls, not a build script

`pub fn conflicts(bindings: &[Binding]) -> Vec<(Surface, Chord, Command, Command)>` returns pairs that collide.
A `const` assertion would be nicer but `Chord` cannot be compared in a const context; a test that calls
`conflicts(&bindings())` and asserts it is empty is the honest version.

## Risks / Trade-offs

- **A table plus an executor per surface is more indirection than a `match`.** Accepted for one reason: the panel
  and the palette cannot exist without a single source, and the drift they prevent is already a bug here (the
  session prints hints that duplicate its own `match`).
- **Keycap hints cost width** on a narrow library header. Mitigated by drawing them only on hover at widths below
  the tile minimum, a rule the tile already computes.
- **`Key::Text` matching is case-sensitive**, so a chord table must record lowercase letters and let dispatch
  fold case; a test pins that both `s` and `S` saved.

## Migration Plan

No storage change. The three `match` arms are deleted in the same commit that adds the table, so there is never a
moment with two sources.

## Open Questions

- Whether the palette should also list commands from other surfaces, disabled rather than hidden.
