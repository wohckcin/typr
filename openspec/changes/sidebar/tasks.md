# Tasks: sidebar as the app's navigator

## 1. Structure and rows

- [x] 1.1 Add `crates/typr/src/sidebar.rs` with `WIDTH = 225.0`, `ROW_HEIGHT = 25.0`,
      `INSET = 7.0`, `ICON = 20.0`, `GAP_AFTER_ICON = 14.0`, `GROUP_GAP = 20.0`,
      `pub enum Group { Navigation, Decks, Utilities }`, and `pub fn row<S: Host>(...)`
      producing one 25-point row with a filled, rounded body and a label at
      `INSET + ICON + GAP_AFTER_ICON`.
      *Verification*: a scene draws three rows of two kinds; their label x offsets are
      equal and their heights are 25.

- [x] 1.2 Add `pub fn icon(kind: Kind, name: &str) -> El` drawing a 20-point box with
      either a monogram or a glyph, tinted from a hash of `name`.
      *Verification*: two decks with different names draw different tints; the label
      offset is unchanged by the icon.

- [x] 1.3 Render groups in order and skip an empty group, with the tag group drawn only
      when tags exist.
      *Verification*: a library with no tags draws no `tags` element, and one with tags
      draws it after the deck group.

- [x] 1.4 Put the utility group (`Import`, `Settings`, `Trash`) at the end of the column
      with `GROUP_GAP` before it, drawn as disabled rows until their changes land.
      *Verification*: the rows appear after the deck group, separated by 20 points, and
      pressing a disabled row does nothing.

## 2. Deck rows

- [x] 2.1 Add `fn deck_due_counts(snapshot: &Snapshot, now_ms: i64) -> HashMap<Id, u32>` reading the
      `due_at` field `card-grid` adds to `CardSummary`, skipping archived and trashed cards and
      crediting every ancestor deck.
      *Verification*: a snapshot with one due card under `Spanish` and one not due counts exactly
      `{spanish: 1, parent: 1}`; an archived deck's cards are absent. If `card-grid` has not landed,
      this task starts with the `CardSummary` widening and says so.

- [x] 2.2 Draw each deck's count right-aligned, and draw nothing for a zero.
      *Verification*: a deck with one due prompt draws `1`; a quiet deck draws no badge.

- [x] 2.3 Indent nested decks with the existing `10.0 + depth * 12.0` step, and fold a
      parent's subtree away when it is collapsed, with the parent's own count unaffected.
      *Verification*: collapsing a parent with one child removes exactly the child row.

- [x] 2.4 Keep `collapsed: HashSet<Id>` on `Library` so it survives a snapshot refresh.
      *Verification*: collapsing, then applying a new snapshot, leaves the deck collapsed.

- [ ] 2.5 Fill the selected deck's row with the selection surface and draw its label at
      full contrast.
      *Verification*: the selected row and a sibling differ in background and in text
      colour.

## 3. Search, chord and state

- [ ] 3.1 Draw the search row full-bleed at 28 points with the chord as a keycap read from
      the keymap table, and fail a test if the drawn chord and the binding disagree.
      *Verification*: changing the binding in the table changes what the row draws, and
      the test fails when the row is given a constant instead.

- [ ] 3.2 Draw the state row: neutral when the library is open, danger tone naming the
      reason when it is not.
      *Verification*: a state with no store draws the danger tone and the status line below
      the screen names the reason.

## 4. Verification

- [ ] 4.1 Scenes `sidebar-decks`, `sidebar-collapsed`, `sidebar-closed`; snapshot each and
      compare the row geometry with the measured numbers above.
      *Verification*: `cargo run -p typr -- --scene sidebar-decks --snap` writes a
      non-empty file, and the tree lists rows 25 points tall with equal label offsets.

- [ ] 4.2 Run `cargo test --workspace --locked`, `cargo fmt --check` and
      `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
      *Verification*: all three exit zero.

## Workflow follow-up

- [ ] 5.1 Record how the column now behaves in `openspec/changes/keymap-panel` if the chord
      lookup forces a different seam, then archive with
      `openspec archive sidebar --yes`.
