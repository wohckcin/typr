# Proposal — native-elements

## Why

`nsh/base` 0.5.1 ships the surfaces typr redraws by hand. Verified against the crate sources:

| Surface | `nsh/base` element | What typr does instead |
|---|---|---|
| Command palette | `command.rs`: `Command`/`CommandItem`, an index, sections, a virtualized row list, Up/Down/PageUp/Return/Escape bound to the search field, the highlight projected as `focus:` for screen readers | `screens/keymap.rs:223` builds rows in `view().gap_2()` from a filtered `Vec`, with no virtualization, no highlight state and no a11y |
| Shortcut panel | the same `Command`, without the field | `screens/keymap.rs:195` and its row builder at `:169` |
| Review history | `table.rs` / `data_table` | `screens/history.rs:41-64`: one `surface()` per record, four facts concatenated into two labels |
| Deck tree | `tree.rs`: `TreeItem`, `focus`, `reveal_item`, folding | `screens/sidebar.rs`: rows with `on_press` (`:265`, `:295`) and a `badge` for the due count (`:272`) |
| Empty state's label | `text` | `screens/grid.rs:121-124`: `badge().label("New card")` and a bare chord character |

Neither surface has a caller today: `grep -rn 'keymap_panel\|palette(' crates/typr/src` finds only their
definitions, so `?` and the palette chord open nothing while `keymap-panel` promises both.

The last row is the one a reader sees first: the empty grid shows a status chip reading "New card", then a lone
`c`, then the sentence "This deck has no cards yet", then a button reading "New card" again.

`ui/status.rs:57`'s `keycap` is the one hand-built piece with no counterpart — `nsh-base/src/keys.rs` is the
keyboard-binding helper, not a keycap visual — so it stays.

## What Changes

- The palette and the shortcut panel are `nsh_base::command`, themed by `ui/`, keep the bindings table as their
  only source of items, and are drawn in the shell so their chords open them.
- The history is a table; the deck tree is a tree; the empty state's label is text.
- A hand-built replacement of a base element is recorded in `DESIGN.md` with the reason, so the next one is a
  decision rather than a habit.

## Capabilities

### New Capabilities

- `native-elements`: which surface a screen builds from, when a `nsh/base` element already provides it, and how
  a hand-built replacement is justified.

### Modified Capabilities

None. `keymap-panel`'s palette and panel requirements stay true word for word — the panel still lists exactly the
bindings of the current surface and the palette still runs the command a query narrows to; this change only
stops those requirements being satisfied by hand-drawn rows.

## Impact

- `crates/typr/src/screens/keymap.rs` (272 lines) shrinks to the table-to-items mapping and the panel's chrome.
- `crates/typr/src/screens/history.rs`, `sidebar.rs`, `grid.rs`.
- `crates/typr/src/shell.rs`, which draws the palette and the panel and binds their chords.
- `keyboard.test.tsx`, whose two tests press those chords under `nsh test`.
- `crates/typr/src/ui/`: whatever styling the base elements need (`command.rs` takes the app's `itemClass`,
  `renderItem`, `empty`, `footer`), added as tokens and one component each.
- `DESIGN.md` gains the elements section and the hand-built list.
