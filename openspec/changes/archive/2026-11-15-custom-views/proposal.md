# Proposal — custom views

## Why

Mochi's sidebar is not only decks. You can pin a card, search, filter by tag and due state, then save
that combination as a view with a name and its own sort order. A view answers questions the deck tree
cannot: everything due this week, every card tagged `exam` across three decks, everything I pinned,
everything I edited but never studied.

typr has one list with a search box, a tag filter and three modes. Nothing survives a restart, nothing
can be ordered except by the store's fixed order, and there is no way to keep a filter that took work
to build.

## What Changes

- A card can be pinned, and pinned cards can be listed without a deck.
- A view is saved: a name, an optional deck scope, a search text, a tag set, a mode, a pinned-only
  flag and a sort order.
- Views are listed in the sidebar beside the decks, and selecting one applies all of its parts at once.
- The list gains sort orders: last edited, created, first side, due date, review count, and shuffled.
- A view remembers changes the user makes to its sort while it is selected, so reopening it is not a
  surprise; the other parts stay as saved until the view is edited deliberately.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None: `card-library`'s listing requirement is extended by an addition rather than rewritten, for the
same reason the earlier clone changes gave. `study-session` is untouched; starting a session from a
saved view is the next change's business.

## Impact

- `crates/typr-core/src/store/schema.rs`: migration adds `card.pinned` and a `view` table.
- `crates/typr-core/src/store/{mod,db}.rs`: `CardSummary` grows `pinned`; a `ViewSpec` type in core;
  commands create, rename, delete and update a view; `Snapshot` carries the views.
- `crates/typr/src/library.rs`: a views section in the sidebar, the sort control, a pin toggle on the
  row, and the filtering path routed through `ViewSpec`.
- `crates/typr-core/src/store/mod.rs` `Snapshot::cards_matching` takes a `ViewSpec` instead of four
  loose arguments.
- No dependency changes; a library with no views behaves exactly as it does today.
