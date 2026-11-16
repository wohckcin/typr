# Proposal

## Why

A review of the app against the official NSH coding guides and design guides found six places where the
app breaks a rule the guides call a must, and one where it can act on the wrong object. The worst is the
card grid's selection: it is stored as a list position, so typing in the search field moves the highlight
to a different card and Enter or Delete then acts on that card. The rest are contract and visual gaps:
component events carry bare values instead of named objects, public structs cannot gain a field without
breaking callers, controls have no size scale, a button has no hover state, `DESIGN.md` documents motion
the app does not have, and copy on three dialog-opening commands lacks the ellipsis the guides require.

## What Changes

- The library's selection becomes a card identity, not a list position. Typing a filter, reordering or
  trashing a card never changes which card is in force, and arrow keys, Home and End move the selection by
  identity.
- The editor's component events carry one named object each, with act names in place of
  `DiagramFromImage` and `RemoveMaskGroup`.
- Every public struct with public fields gains `#[non_exhaustive]`, with `Default` or a constructor for the
  callers that still build it, so a later field is an addition rather than a breaking change. `Values` keeps
  its constructor only, because its signals belong to the caller's owner.
- The app's styled controls gain one `size` prop that sets height, horizontal padding, text size and
  radius together, and control frames use a minimum height rather than a fixed one so larger text does not
  clip.
- `ui`'s button, tile and list row gain hover and press states drawn from the motion tokens, with
  `reduce_motion()` honoured.
- `DESIGN.md`'s motion inventory is reconciled with the code: the sidebar's current-item selection slides,
  and lines that stay unbuilt say so with a reason.
- Popups paint with the theme's `popover` surface, the settings daily-cap field gets a visible label, and
  the three commands that open a dialog gain an ellipsis.
- The two seams the same review left open are closed while the shell is being changed: the store writer becomes
  `apply` and `quiet_apply`, the study screen reports a refused grade as an event rather than a `bool`, and
  tile-name flattening moves out of the grid.
- No **BREAKING** change: the editor's event payloads and the public structs are crate-internal surfaces
  for a Rust-only app, and both change in one wave with every caller.

## Out of Scope

- Reverting the stateless `ui` controls from components to plain functions. The guides allow a component when a
  control has props or events of its own, the app registers nothing for TSX, and the rollback would move about
  eighty call sites for no visible change. The rule for new code is a plain function for decoration and a
  component when the piece owns state or an event seam.
- A shipped gallery screen. `tests/gallery.rs` renders every control and `tests/appearance.rs` renders every
  screen in both appearances, which is the verification such a page exists for.
- Making the sidebar's search chord a plain field. `Sidebar` is `Copy`, and its copy is what lets about fifteen
  closures in `shell.rs` capture it by value; a plain `SharedString` is clone-only, so the swap would ripple
  through every library key handler. The chord stays a signal written once, and its doc comment says so.
- The editor's remount on every store write. The draft is written per keystroke, so text survives and focus does
  not; it is recorded in `design.md` as an accepted risk rather than fixed here.

## Capabilities

### New Capabilities

None. Every behaviour below belongs to a capability the project already specifies.

### Modified Capabilities

- `card-grid`: the selection is a card identity (not a position), filter and reorder preserve it, and the
  keyboard model covers arrows, Home and End.
- `visual-language`: a control's size is one value, hover and press are drawn from the motion tokens and
  reduced motion, motion matches the written inventory, popups share one surface, and a command that opens
  a dialog says so with an ellipsis.
- `preferences`: the daily-cap field carries a visible label, not only a placeholder.
- `project-structure`: public API additions only widen, and a component's events carry one named object
  each.

## Impact

- `crates/typr/src/shell.rs` and `crates/typr/src/screens/library.rs`: the selection becomes
  `Option<Id>`, and Enter, Delete and the arrows resolve the target by identity.
- `crates/typr/src/screens/grid.rs`: the highlight compares the tile's id, and the grid reports a chosen
  id rather than an index.
- `crates/typr/src/screens/editor.rs`: `EditorEvent` variants gain named payloads; the module is split
  into the component, the diagram pane and the field table.
- `crates/typr/src/ui/*`: a `size` prop on the controls that have one, `min_h` instead of `h`, hover and
  press states, and the popup surface.
- `crates/typr/src/screens/settings.rs`: a visible label for the daily-cap field.
- `crates/typr/src/screens/study.rs`: a refused grade leaves as an event, not a `bool`.
- `DESIGN.md`: the motion inventory is reconciled with the shipped behaviour.
- `crates/typr/tests/*`: a regression test for filter-then-Enter, keyboard-model tests, and the size and
  state assertions the appearance tests cannot see.
