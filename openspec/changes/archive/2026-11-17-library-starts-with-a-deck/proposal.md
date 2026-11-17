# Proposal — library-starts-with-a-deck

## Why

A library that did not exist yet came up with no deck at all. `crates/typr/src/paths.rs:37`'s
`open_from_environment` created the folder, the attachments folder and the database, and left the database
empty; nothing in `crates/typr/src` creates a deck on the app's own initiative.

Two things then failed at once, both visible from a fresh `TYPR_DATA_DIR`:

- The sidebar listed no deck, so there was no card to be in.
- The grid's empty state offered one action, and pressing it did nothing: `crates/typr/src/shell.rs:626` and
  `:748` passed `|| {}` where the grid takes `on_new`, and the new-card action returns early when no deck is
  selected and the snapshot holds none (`shell.rs:405-437`).

`openspec/specs/visual-language/spec.md:87-89` already promises the second half — "the library holds no card
… the list area shows an empty state whose action creates a card" — so the button that wrote nothing was a
defect against a requirement in force, not a missing feature.

## What Changes

- Opening the library seeds one deck when the library holds no deck at all, so the empty screen's action has
  somewhere to put the card it offers.
- The empty deck's button runs the same action the new-card chord runs: one closure in the shell, handed to the
  grid at both of its call sites.
- The seeded deck is named `Default`, and is seeded only when the library holds none: a library with decks,
  archived ones included, is left as it is.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `app-startup`: adds the deck a new library starts with, next to the existing requirement that the app opens
  on the library from its run path.

## Impact

- `crates/typr/src/paths.rs`: `open_from_environment` seeds the first deck after the database opens.
- `crates/typr/src/shell.rs`: the grid's `on_new` at both call sites.
- `crates/typr/tests/startup.rs`, `crates/typr/tests/shell.rs`: one test each.
