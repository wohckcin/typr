# Proposal — hidden text (cloze deletions)

## Why

Mochi's most-used card type hides part of a sentence so the learner recalls it in context:
`The hormone {{insulin}} is produced by the {{pancreas}}.` typr can only study a card by revealing
sides in order, so a card that asks about each fact inside one sentence cannot be built at all, and
`review-scheduling` currently forbids more than one prompt per card. This is the first of the changes
that close the gap to Mochi, and it is the one that exercises several prompts per card, which reverse
reviews and diagram masks will reuse.

## What Changes

- `{{hidden text}}` in a card is parsed as hidden text, outside code fences and inline code.
- `{{1::insulin}}` and `{{2::pancreas}}` are numbered groups. Each distinct number becomes its own
  review prompt with its own schedule and history, keyed `cloze:<n>`.
- Unnumbered hidden text forms one further prompt keyed `cloze:0`, hiding every unnumbered span at
  once, so a simple card still reviews as one prompt.
- A card holding hidden text is studyable even with no side separator, and on such a card the
  three-hyphen line stops separating sides: the text is one face and the groups are the prompts.
- A review face now carries what is hidden and what is shown, so revealing a hidden-text prompt
  unmasks the sentence instead of advancing to a second side.
- Cards without hidden text keep their current behaviour exactly: `fwd`, sides in order, nothing
  else changes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `review-scheduling`: "Each card has at most one prompt" becomes one prompt per review variant, and a
  new requirement defines what hidden text does to studyability and to prompt keys.
- `card-editor`: a new requirement says hidden text is written inline, previewed masked, and that the
  separator stops splitting sides on such a card.
- `study-session`: the reveal requirement covers unmasking within one face, which is what a
  hidden-text prompt shows.

## Impact

- `crates/typr-core/src/`: a hidden-text parser next to `sides.rs`; migration V4 adds a `has_cloze`
  column, recreates the `studyable_prompt` view to accept it, and `sync_prompt` becomes
  `sync_prompts(card, shape)`; `Entry` carries the prompt key and per-face hidden/shown text.
- `crates/typr/src/`: the editor preview masks the active group; the session screen renders the masked
  face.
- Data: no table is dropped, existing prompts and history are untouched, and a card that never
  contains hidden text behaves as it does today.
- Not in this change: typed hidden-text answers, diagram masks, reverse reviews.
