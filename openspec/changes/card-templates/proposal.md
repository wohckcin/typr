# Proposal — card templates

## Why

Mochi cards need not be written one by one. A deck can carry a template: markdown with placeholders
like `<<Word>>` and `<<Meaning>>`, and every card in that deck is just those fields filled in. One
template makes a deck of five hundred vocabulary cards a table of values, keeps the wording identical
across them, and lets a section appear only when a field has a value.

In typr every card is written out in full, so a deck of similar cards means retyping the same
structure, and there is no structured data to filter, sort or export later.

## What Changes

- A card gains fields: named values stored beside its markdown.
- A template is markdown with `<<Field>>` placeholders, plus sections rendered only when a field has a
  value. The template supplies the card's text; fields supply the values.
- A deck carries a default template, and a card may carry its own. A deck without a template changes
  nothing about how its cards are written.
- A few fields are dynamic: their value comes from the card rather than from the user.
- The editor shows the template on one side and the fields on the other, with the rendered card as the
  preview, and the card's sides come from the rendered text rather than from typed separators.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None, for the same reason as the two changes before this one: the prompt and reveal blocks are held by
an open change, so this change adds requirements beside them.

## Impact

- `crates/typr-core/src/store/schema.rs`: migration adds `card.template`, `deck.template` and a
  `field(card_id, name, value)` table.
- `crates/typr-core/src/store/{mod,db}.rs`: `CardSummary` grows `fields` and `template`; commands set
  a card's fields, its template and a deck's template; `sides` and `search_text` are computed from the
  rendered text.
- New `crates/typr-core/src/template.rs`: placeholder substitution, conditional sections and dynamic
  values, with no I/O and no knowledge of the store.
- `crates/typr/src/editor.rs`: a template pane, a field table, and the rendered preview.
- `crates/typr/src/library.rs`: a deck's template control and a marker on cards that inherit one.
- No dependency changes; a library with no templates renders exactly as it does today.
