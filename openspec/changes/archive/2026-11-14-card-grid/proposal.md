# Proposal — card grid

## Why

The library draws every card as a 44 px row of text. Mochi draws a wall of card-shaped tiles: each one
shows the card's first side, how many sides it hides, and where its schedule stands. Those three pieces
are what make a deck scannable without opening anything, and a row list cannot carry them.

The gap was measured on the running app (2026-11-14): tiles 233x157 at a 1485 px window, 287x132 at 845 px,
8 px radius, a 1 px border, the shared card shadow, 10 px padding, and a 27 px footer strip whose bottom
corners carry the tile radius. The footer reads `NEW` or `0 DAYS`. A two-sided card's tile shows only its
first side plus a `1 HIDDEN SIDE` badge. The empty slot is a dashed tile of the same size carrying the
label and its `N` keycap.

## What Changes

- The library renders the selected deck as a wrapped grid of tiles instead of a row list.
- A tile shows the first side only, a hidden-side badge when it hides more, its schedule state, and its
  tag chips with an overflow count.
- The empty library renders a dashed placeholder tile that starts a new card.
- The grid is virtualised by tile rows, so a deck of thousands still builds only what is on screen.

## Capabilities

### New Capabilities

- `card-grid`: the tile, its badges, the wrapping rule, and the placeholder that stands in for nothing.

### Modified Capabilities

None. The library's listing requirement is about which cards appear and how search reaches them; this
change alters how they are drawn, not which ones are selected.

## Impact

- New `crates/typr/src/library/tile.rs`: the tile, its badges, and the footer strip.
- `crates/typr/src/library.rs`: the row stack becomes a column of tile rows, each holding as many tiles as
  fit; `Library::visible` feeds the same filtered list.
- `crates/typr/src/tests.rs`: the row-count assertion becomes a tile-count assertion; one scene per deck
  shape.
- No dependency changes. `rune::el::virtual_rows` still virtualises, one row per line of tiles.
- Supersedes the "list of card objects" bullet in `changes/visual-parity/proposal.md`; the tokens, type
  scale, elevation and baselines that change defines are still what these tiles are drawn with.
