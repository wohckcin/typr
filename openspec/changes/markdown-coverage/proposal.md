# Proposal — markdown coverage

## Why

Mochi's shortcut panel documents the syntax its editor accepts, and cards written for it use all of it. This
app renders a subset: the parser is built with strikethrough, tables and task lists only
(`crates/rune-kit` `markdown/parser.rs:466`), so highlighted text, superscript, subscript, definition lists,
ruby glosses and math either vanish or show as raw markers.

A card imported from Mochi that says `==important==` currently reads as `==important==`. That is a content
regression, not a cosmetic one.

## What Changes

- Highlighted text (`==text==`), superscript (`20^th^`), subscript (`H~2~0`) and definition lists render.
- Ruby glosses (`{base}(reading)`) render the base with its reading above it.
- Math (`$..$`, `$$..$$`) is preserved and shown verbatim in a distinguishable style instead of being dropped.
- Any construct the renderer does not understand falls back to its literal text, never to nothing.
- Syntax the parser can do but the renderer cannot is either taught to the renderer or declared unsupported in
  one place, so no span silently disappears.

## Capabilities

### New Capabilities

- `markdown-coverage`: which markdown constructs render, how unknown constructs degrade, and where the list of
  supported constructs lives.

### Modified Capabilities

None. `card-editor`'s "A card's text is markdown" stays true; this change widens what that sentence means.

## Impact

- `crates/rune-kit/src/markdown/parser.rs`: parser options gain superscript, subscript and definition lists.
- `crates/rune-kit/src/markdown/render.rs` (upstream): rendering for highlight, ruby, definition lists and the
  math events, plus a style hook for the highlight background.
- A pinned-revision bump of `rune` and `rune-kit` in `Cargo.toml` and of the four `[patch.crates-io]` entries,
  which the workspace requires to move together.
- A pre-pass in `crates/typr-core/src/markdown_ext.rs` for the two constructs the parser cannot see (`==` and
  ruby), proven not to fire inside code spans or fences.
- No behavioural change to scheduling, storage or the review loop.

## Explicitly out of scope

LaTeX typesetting. `changes/archive/2026-11-14-typr-v1/design.md:30` already records that Rune has no math
renderer; this change keeps math *readable* (verbatim, set apart) and leaves an actual typesetter to a later
change. Mochi typesets math; this is a known divergence, not an oversight.
