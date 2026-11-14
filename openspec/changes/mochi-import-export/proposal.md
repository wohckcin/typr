# Proposal — Mochi import and export

## Why

A Mochi user has months of work in their library. A clone that cannot read those files asks that user to
abandon it, and a typr library that cannot leave is a lock-in of the same kind.

typr's library is already a single SQLite file, so exporting everything is a copy. What is missing is the
other direction: reading Mochi's own export, and the text formats every other tool speaks.

## What Changes

- Export the whole library as one file that reopens as a library, including history, fields, masks and
  images.
- Export cards as CSV and Markdown for tools that want text, with a documented column set.
- Import a Mochi export: decks, cards, templates, fields, tags, images and each card's review history,
  mapping Mochi's two grades and its per-variant histories onto prompts.
- Import CSV and Markdown into a chosen deck.
- Refuse an archive whose shape is not recognised, listing what was found, instead of importing part of
  it.

## Capabilities

### New Capabilities

- `transfer`: importing and exporting a library, the file shapes involved, and what survives a round
  trip.

### Modified Capabilities

None.

## Impact

- New `crates/typr-core/src/transfer/` module: `export_library`, `import_mochi`, `export_csv`,
  `import_csv`, `export_markdown`, `import_markdown`, each pure over paths and the store.
- `crates/typr-core/src/store/schema.rs`: no new columns; the importer writes through existing commands.
- `crates/typr/src/library.rs`: an Import action and an Export menu behind the native file dialog.
- A zip reader is the one likely new dependency, since a Mochi export is a zip; the backend's rule is
  "no desktop toolkit", so a pure-Rust archive crate is allowed and must be recorded in `Cargo.toml`.
- No behaviour change for a library that never imports or exports.
