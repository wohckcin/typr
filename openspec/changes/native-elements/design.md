# Design — native-elements

## Context

**The base crate's inventory**, read at `~/.cargo/registry/src/*/nsh-base-0.5.1/src/`: `command.rs`,
`table.rs`, `data_table/`, `tree.rs`, `listbox.rs`, `toolbar.rs`, `navigation.rs`, `menu.rs`, `popover.rs`,
`popup.rs`, `sheet.rs`, `tabs.rs`, `scrollbar.rs`. None of them has a look of its own; each takes the app's
classes and renderers, which is exactly what `crates/typr/src/ui/` supplies.

**What `Command` gives that the hand-built palette lacks.** From `command.rs`'s module docs: one index built per
change of the items; rows in a virtualized list in a scroll view; Up/Down/PageUp/PageDown/Return/Escape bound
while the search field holds the focus; typeahead; the highlight projected as `focus:` because "NSH has no
active descendant"; screen readers announce the highlighted label; each row drawn once per item and not per
state.

**What the hand-built palette does** (`crates/typr/src/screens/keymap.rs`): `rows(surface)` (`:154`) filters
`typr_core::keymap`'s table, `visible_rows` (`:161`) filters by substring, `palette` (`:223`) draws them into a
`view().gap_2()` and decides the Enter/Escape handling itself; `keymap_panel` (`:195`) does the same without the
field. It has no virtualization (fine at today's ~20 bindings, wrong at 200), no highlight, and nothing for a
screen reader.

**The rows typr redraws elsewhere.** `screens/history.rs:41-64` puts every record in its own `surface()` with two
concatenated labels where a table has four columns. `screens/sidebar.rs` builds deck rows from `on_press` and a
`badge` for the due count, with folding in `Sidebar::toggle_collapsed` (`:129`), where `tree.rs` offers
`TreeItem`, `focus` (`:531`) and `reveal_item` (`:540`). `screens/grid.rs:121-124` labels the empty deck with
`badge()` — a status chip — followed by a bare chord character.

**The one exception.** `ui/status.rs:57`'s `Keycap` has no base counterpart: `nsh-base/src/keys.rs` binds a
control's keys while it holds focus and draws nothing. It stays, and it gets a line in `DESIGN.md`.

## Goals

- Each screen's list, tree or panel is the base element that already implements its interaction, keyboard and
  accessibility.
- The bindings table stays the only source of a command's name and chord: the panel maps table rows to
  `CommandItem`s and nothing else.
- Screens keep composing from `crates/typr/src/ui/`; `tests/tokens.rs` and `scripts/check-tokens.sh` stay green.
- The change removes code: `screens/keymap.rs` loses its row drawing, `history.rs` its per-record surfaces.

## Non-Goals

- Restyling anything: the base elements are themed to look like today's screens, not redesigned.
- Adopting `nsh-base` elements the app does not currently need (`tabs.rs`, `pagination.rs`, `carousel.rs` …).
- Replacing `keycap`, `Empty`, `badge` or `skeleton` — the first has no base equivalent, and the others are the
  app's own composition of primitives, not reimplementations.

## Decisions

1. **The palette and the shortcut panel become one `Command`.** The panel is the same element with the footer's run
   action hidden and its search field kept: `keymap-panel` requires the panel to be filtered by typing
   (`openspec/specs/keymap-panel/spec.md:54`), so hiding the field would leave that scenario unmet. One list
   implementation instead of two; the items come from `bindings_of(surface)` — label and chord — so the panel
   still cannot list a binding that does not exist.
2. **The history is a table, not a list of cards.** Four fields per record are columns; `table.rs` supplies the
   row model and the keyboard movement. The count stays a `badge` in the header.
3. **The deck tree becomes `tree.rs`, keeping the due counts.** A tree item carries the deck's name; the due count
   becomes the item's trailing content rather than a separate badge, because a badge states a status and the
   count is metadata. If the base tree cannot express a trailing count, the count moves into the label and this
   decision is amended — task 3.2 records which.
4. **The empty state's label is text, and the chord is dropped.** "New card" already names the button beneath it;
   the chip above it is the wrong element for a label, and the bare `c` is a chord with no keycap and no verb. The
   button keeps its key in the sidebar's hint, where a chord is shown as a chord.
5. **The panel and the palette are mounted, not just replaced.** `keymap_panel` and `palette` have no caller in
   `crates/typr/src`: the spec's two requirements — the panel opens on the shortcut key, the palette opens on its
   chord — are met by code nothing draws. Replacing a screen nothing shows would leave the requirements unmet with
   a nicer element, so task 1.6 draws it in the shell and the TSX tests press the keys.
6. **Hand-built replacements get a `DESIGN.md` line.** The motion inventory's form — a table row stating the
   decision and its reason — applies to elements for the same reason: it makes the next one deliberate.
