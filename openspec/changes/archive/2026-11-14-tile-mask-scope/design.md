# Design — tile masking

## Context

`typr_core::hidden` reads a card into a `Shape`: `Sides(Vec<String>)` for a card split by `---`, and
`Hidden { text, groups }` for a card holding `{{text}}` / `{{n::text}}`. It offers two renderings:
`masked(text, shape, key)` hides one group, which is what a review face needs, and since this change
`masked_all(text, shape)` hides every span and labels a numbered group as `[n … ]`.

`crates/typr/src/editor.rs::preview_body` and `crates/typr/src/library/tile.rs::first_side` both call
`masked_all`; `crates/typr/src/main.rs::session_entries` builds the review faces with `masked`, one per
prompt key.

## Goals

- The requirement text names the scope the code has: every span masked on a tile.

## Non-Goals

- Changing how a review face masks, or how many prompts a card is cut into.
- Styling the group label beyond the `[n … ]` form the mask already uses.

## Decisions

### The tile masks all groups, the review masks one

A tile is a browsing surface: it is the one place where a card's answers sit side by side on screen, and a
reader who wanted to peek would only need to open the grid. A review face is an asking surface, so it hides
the group it asks and leaves the others readable — which is what makes a card with two groups useful. The
two scopes answer different questions, so the spec states each separately instead of comparing them.

### The group number rides on the mask

Labelling costs four characters and tells the writer, on the grid, how many prompts a card asks without
opening it. The unnumbered form stays `[ … ]`, because one group has nothing to tell apart.

## Risks

- A label reads as content to a skimming eye; the `[ … ]` form already does, so the risk is unchanged.
- A card whose group numbers are large (`{{12::x}}`) widens the mask; nothing measures or truncates it.
