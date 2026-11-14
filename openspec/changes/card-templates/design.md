# Design — card templates

## Context

`CardSummary` carries `markdown`, `search_text`, `sides` and `tags`, all produced by the store from the
`card` row. `sides` is written by `CreateCard`/`SaveCard` through `crate::sides::split`, and
`studyable_prompt` admits a card at `sides >= 2`. `search_text` is `crate::store::folded(markdown)`.
The editor holds one `TextEdit` for markdown plus one for tags and previews markdown through
`rune_kit::markdown::Markdown`.

`Sides(split)` is already the only place a card's faces are decided, which is what lets a template
produce sides without touching the session.

## Goals / Non-Goals

**Goals**

- Fields as first-class data, not text parsed out of the card every time.
- One template for a whole deck, with a per-card override, and no behaviour change where no template
  exists.
- Rendering in one pure module, so the preview, the store's `sides` and the review all agree.

**Non-Goals**

- Conditional expressions, arithmetic or a scripting language. A field either has a value or it does
  not.
- Card naming, `[[references]]` and dynamic fields beyond the fixed set; names are the next roadmap
  row.
- Importing Mochi templates, which waits for `mochi-import-export`.
- Templates that generate cards (Mochi does not either: a template formats a card, it does not create a
  deckful).

## Decisions

**Rendering is a pure function in `typr-core`.** `template::render(template, &fields, &context) ->
String` takes the template text, the field map and the few dynamic values it may need, and returns the
card's text. The store calls it to compute `sides` and `search_text`; the editor calls it for the
preview; the session reads the stored text. One function, three callers, no drift, and it is testable
without a database or a window.

**Text is stored rendered, not re-derived on read.** `card.markdown` keeps holding the card's text as
it is reviewed, and the renderer writes it whenever a field, the card's own template or an inherited
deck template changes. Re-rendering on every read would put a template walk in the due queue and in
every list frame; storing it keeps `search_text`, `sides` and `preview` as cheap as they are today,
and the cost is a re-render pass when a deck template changes, which is bounded by that deck's cards.

**Fields are a table keyed `(card_id, name)`.** A JSON blob in a column would need a parser in Rust to
answer "which cards have an Example", which the next roadmap row (custom views) will want; the table
gives that for a `WHERE` clause and keeps `search_text` cheap by folding values in at write time.

**Sections use `{{#Field}}…{{/Field}}`, placeholders use `<<Field>>`.** The two sigils differ so a card
that legitimately contains `{{` for hidden text (the cloze syntax) cannot be mistaken for a section.
An unclosed section renders as text rather than swallowing the card, because a typo in a template
should not silently delete half of five hundred cards.

**Dynamic fields are a fixed list, marked in the field table.** `Name` and `Deck` are computed at
render time and never stored, so a card renamed by editing its first side does not need a migration.
Any user value sharing a dynamic field's name is ignored rather than shadowing it, and the field table
says so.

## Risks / Trade-offs

- `card.markdown` becomes a cache of a function. The invariant is that it always equals
  `render(template, fields)`, and a test must assert it after every mutation path (field edit, template
  edit, deck template edit, deck move), mirroring how `prompt` is checked against `review_log`.
- A deck template change rewrites every card's markdown in that deck. That is a visible bulk write; it
  is bounded and it must run in one transaction per deck so a failure leaves nothing half rendered.
- `sides` can flip when a field empties, which changes queue membership. This is the intended
  behaviour and is already the rule for a card that becomes a note; the history stays.
- Two sigils for two features is a syntax burden. The editor's preview shows the rendered result before
  saving, and the README gains a table of the three syntactic forms the app now has.
