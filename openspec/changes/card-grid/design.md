# Design — card grid

## Context

`crates/typr/src/library.rs` builds `col().key("rows").growth(..).virtual_rows(count, ROW_HEIGHT, ..)` and
each row is a 44 px line of text with a preview and a metadata line. `Library::visible` already holds the
filtered cards, and `CardSummary` (`crates/typr-core/src/store/mod.rs:87`) carries `preview`, `sides`,
`markdown`, `tags`, `updated_at` and `trashed_at` — but no schedule, because the snapshot deliberately
holds no scheduling state. A tile's badge needs three facts that are not there, so this change adds them
in one place, and `sidebar`'s deck counts read the same fields instead of inventing a second query.

Measured on the Mochi web app, dark theme, 2026-11-14: tile 233x157 (845 px window) and 287x132 (1485 px
window), radius 8 px, border `1px solid rgba(255,255,255,0.04)`, the shared card shadow, 10 px padding,
footer strip 27 px with `0 0 8px 8px` radius, badges in 12 px uppercase, first side in 16 px, tags as chips
6 px apart.

## Goals

- A deck is scannable: each tile carries its card's first side, how much it hides, and where it stands.
- The grid still builds only what is on screen.
- The tile is one constructor, so `card-grid`, `deck-views` and `visual-parity` all draw the same object.

## Non-Goals

- No masonry, drag-to-reorder, or per-tile resize.
- No change to which cards a deck lists, or to search, tags or trash.
- No change to the review surface, the editor or the session.

## Decisions

### A tile row is the virtualisation unit

`virtual_rows` counts rows of one element each. The grid needs rows of several tiles, so the code virtualises
tile rows: `fn tile_rows(cards: &[CardSummary], columns: usize) -> usize` is `cards.len().div_ceil(columns)`,
each row builds `columns` tiles, and the row height is `TILE_HEIGHT + GAP`. Passed `columns`, the same
`virtual_rows` call works, and a 5000-card deck still builds only the rows in view.

### The column count comes from the window width, floored at one

`fn columns_for(width: f32) -> usize` returns `((width + GAP) / (MIN_TILE_WIDTH + GAP)).floor().max(1.0)` with
`MIN_TILE_WIDTH = 233.0` and `GAP = 16.0`. Tiles then take `(width - GAP * (columns - 1)) / columns`, so they
fill the line instead of leaving a ragged edge. Measured Mochi widths (233 at 845, 287 at 1485) both fall out
of this rule with the same constants, which is why the numbers are these and not round ones.

### The badge vocabulary is three states, not a date

A tile states `new`, or the interval in whole days with the word day, or `due` when the prompt is due now.
Dates were rejected: a tile is read at a glance, and "in 3 days" versus "13 Nov" costs a decode step the
badge cannot afford.

**The schedule arrives with the snapshot, as three fields on `CardSummary`.** `Snapshot` becomes the one
source a view reads, so a tile never waits on a round trip. The card query at
`crates/typr-core/src/store/db.rs:302` gains a left join on `studyable_prompt` and two aggregates:

```sql
SELECT c.id, c.deck_id, c.sides, c.archived, c.trashed_at, c.updated_at, c.markdown, c.search_text,
       COUNT(sp.card_id), SUM(sp.due_at IS NULL), MIN(sp.due_at)
FROM card c
LEFT JOIN studyable_prompt sp ON sp.card_id = c.id
GROUP BY c.id
```

`CardSummary` gains `prompts: u32` (how many studyable prompts the card carries), `unscheduled: u32` (how
many of them have never been scheduled) and `due_at: Option<i64>` (the earliest of them, which is what a
badge wants). A card with `prompts == 0` is a note and draws no badge; `due_at` of `None` with
`prompts > 0` means new; a `due_at` at or before now means due; anything later is an interval. Counting
`unscheduled` separately is what lets a card with one scheduled direction and one new direction say so
later, without another migration.

### Tags are chips with a counted overflow

`Snapshot::cards_matching` already returns `tags` per card. The tile lays chips out until the measured width
is spent, then draws `+N`. Measuring inside the builder is not available at build time, so the count is
computed from the tag strings' character widths with the same 12 px monospace-free estimate the header uses;
the estimate is a single function so `deck-views` and the sidebar can share it.

### The first side is rendered, not truncated

A tile renders the first side through the same `Markdown` path the preview uses, capped to the tile's height
by the container's clip, so a card that opens with a heading or a list looks like one. Truncating the source
string was rejected: it would cut `**bold**` in half and show asterisks.

## Risks / Trade-offs

- **Rendered tiles cost more per frame than text rows.** Only rows in view are built, so the cost is bounded by
  `columns × visible_rows`, not by the deck.
- **A long first side is clipped**, not elided, so a card can look cut off. Mitigated by the preview cap of
  three lines inside the tile, with the remainder hidden rather than scrolled.
- **The width estimate for tags is approximate**; a wide glyph can push `+N` off the tile. Accepted, and the
  same estimate backs the sidebar counts.

## Migration Plan

No schema change. Tiles read the existing snapshot. Rollback is a revert of `library.rs` and `tile.rs`.

## Open Questions

- Whether the notebook view (`deck-views`) reuses this tile or draws its own block. Decided there.
