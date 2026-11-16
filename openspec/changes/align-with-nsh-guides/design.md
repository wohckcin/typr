# Design

## Context

The app is a Rust-only NSH app. `shell.rs` owns the work area as a `Screen` value and the library's
transient state, `screens/grid.rs` draws tiles inside a `virtual_list`, and `ui/` holds the styled controls.
The specs govern behaviour; this document covers the technical choices the change needs.

Constraints that shape it: the grid is virtualized and cells are reused, so anything keyed to a cell index is
stale by construction. The controls are used by every screen, so a size prop and state styles change many call
sites at once. Tests live in external crates (`crates/typr/tests`), so a `#[non_exhaustive]` struct is a
breaking change for them exactly as it would be for a downstream user.

## Goals / Non-Goals

**Goals:**

- One owner for the selected card, addressed by `Id`.
- One value per control size, so height, padding, text and radius cannot drift apart.
- Hover and press states that exist once, in `ui`, and honour reduced motion.
- A written motion inventory that a reviewer can check line by line.

**Non-Goals:**

- A general density system with per-screen overrides. One small size set is enough for this app today.
- New screens, new capabilities, or changes to the store's data model.
- Fixing the editor's remount on every store write. That is pre-existing and independent of this change.

## Decisions

### Selection is `Signal<Option<Id>>`

The shell keeps `Option<Id>`, the grid highlights a tile whose id matches, and Enter and Delete use the id
directly. Arrow movement finds the selected id's position in `visible()` and steps from there, falling back to
the first card when nothing is selected.

Alternatives considered. Keeping the index and re-deriving it whenever the filter changes needs a watcher that
runs on every snapshot and query change, and a card that is filtered out still has no honest position.
A `Signal<usize>` plus an `Id` side table reproduces the same two values that can disagree.

### The grid reports and highlights by id

`grid(tiles, chosen: Signal<Option<Id>>, …)` keeps one owner of the choice and lets the cell compare ids, which
survives cell reuse because the comparison reads the tile at its current index. A closure prop
(`selected: impl Fn(Id) -> bool`) was the other candidate; a signal is what the shell already has, and the ring
is a binding, so no wrapper is needed.

### Home and End are keymap commands

The keymap table in `typr-core` gains `SelectionFirst` and `SelectionLast`, so the chords stay in one place with
the rest. Handling literal keys in the shell was rejected because the project's rule is that a screen's keys
come from the table, and the sidebar states those chords.

### Editor events carry named fields

`Save { text }`, `Draft { text }`, `AddMask { mask }`, `MergeMasks { masks }`, `RemoveMaskGroup { group }`,
`ClearDiagram`, `DiagramFromImage { name }` becomes `StartDiagram { name }`. Named fields satisfy the one-object
rule and give the TypeScript declaration a field name per value. A struct payload per event was considered and
rejected as an extra type per event with no gain.

### One `Size` value per control

`Size` is a `PropEnum` with `Sm`, `Md` and `Lg`. A `metrics()` method returns the height, horizontal padding,
text size and radius for that size, and each control forwards it. Frames use `min_h` so a larger text size
grows the control. The existing 32-point height becomes the `Md` height, so the default look does not change.
Alternatives. Keeping per-control constants and adding a prop for each dimension passes the same five numbers
around every call site. A theme-token table alone cannot set a text size from a Rust builder chain.

### Hover and press in `ui`, gated on `reduce_motion()`

Hover is a wash from `tokens::hover()`; press is a stronger wash plus a small scale for buttons and tiles, and
a deepened wash for rows, which is the guide's split. Transitions use the motion tokens and are skipped when
`reduce_motion()` is true. The alternative of styling `active` at call sites was rejected because it would
reintroduce per-screen state styling.

### The motion inventory is reconciled, not broadly built

The sidebar's current-item selection becomes the one moving selection, since `DESIGN.md` already promises it and
it is the clearest spatial move in the app. The tile wave and the skeleton cross-fade are rewritten as "no
motion" with a reason, because a list that loads from SQLite in one frame spends native work on nothing visible.

### `Field` gains a label while the size work happens

`FieldProps` gains `label: Prop<Option<SharedString>>`, drawn above the input when present. This is the guide's
core field (label, control, description, error) with only the part the app needs. Placing a bare text element in
the settings screen was considered and rejected, because the next field would repeat it.

### Public structs evolve through `non_exhaustive` and a constructor

The eight field structs get `#[non_exhaustive]`, which blocks struct literals and `..Default::default()` outside
the defining crate. The integration tests are a separate crate, so they move to `Default` plus field assignment
or to a constructor, and `Item` gains the `Default` it lacks. `Values` keeps `Values::new` as its only
constructor and documents why it has no `Default`, because its fields are signals created under the caller's
owner and a default value would own signals nobody created. Keeping the structs exhaustive was the alternative,
rejected because the guides treat a public field set as a compatibility surface.

### The state fills are a mapping, and the paint is checked on a host

`Variant::fill(state)` maps a variant and an interaction state to a role (`Selection(85)`, `Hover(60)`,
`Card`, `Clear`), and `resolve` turns the role into a theme colour. The mapping is a pure value, so a unit test
proves rest, hover and press differ without a reactive runtime; calling the token accessors in a test would
panic, because they need the engine's environment. Painting is a different claim, and the headless harness
keeps no view style or screenshot, so `desktop.test.tsx` compares two screenshots around a hover on the macOS
host and skips that step where the app opens as a phone.

### The sidebar's search chord stays a signal

`Sidebar` is `Copy`, and about fifteen closures in `shell.rs` capture it by value: one guard per library key,
the pick handler and the tiles memo. A plain `SharedString` field is clone-only, so the swap would turn every
one of those captures into a clone. The chord is written once at construction and read in the search row, so
the signal is cheap and keeps the struct copyable. Documenting it as set once was the other option; rejected
because a field that reads as mutable state invites the next reader to look for its writer.

### The editor module splits along its seams

`screens/editor/mod.rs` keeps the component, its props and its events. `diagram.rs` keeps the pane, the drag
geometry and the covers. `fields.rs` keeps the field table and template rendering. Public paths stay
`screens::editor::*`, so callers and tests do not move.

## Risks / Trade-offs

- [The `non_exhaustive` change breaks the app's own tests and `ui` call sites, because another crate cannot use
  struct literals or functional update] → Migrate them in one wave to the constructors or `Default` plus field
  assignment; the compiler finds every site.
- [The animated selection fill adds native work and a fallback path] → If it does not land cleanly, rewrite that
  inventory line as instant with the reason, which the spec allows.
- [Adding hover to `Button` changes how every screen looks] → The wash comes from the existing hover token and
  stays weaker than a selection; the appearance tests re-render every screen and catch unintended moves.
- [The grid's arrow keys change meaning once selection is `Option<Id>`] → The regression test types a filter,
  presses Enter, and asserts the opened card is the highlighted one; the keymap tests cover Home and End.
- [A store write still remounts the editor] → Accepted for now. The draft is written per keystroke, so text
  survives; focus does not. Recorded so the next reader does not mistake it for a regression.

## Migration Plan

The change lands in one branch, in the order tasks.md sets: behaviour first (selection identity, events), then
the control size and states, then copy and documentation. There is no stored data to migrate and no released
API, so rollback is a revert of the branch.
