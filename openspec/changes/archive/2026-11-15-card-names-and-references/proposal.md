# Proposal — card names and references

## Why

Mochi cards have names, and a card can point at another with `[[Like This]]`. The target shows every
card that points at it, so a deck becomes a small graph: the card for "Krebs cycle" is reachable from
the five cards that mention it, and the mention is one keystroke rather than a copy of the text.

typr cards are anonymous rows with a preview, `[[…]]` renders as literal brackets, and tags are flat, so
"verbs" and "verbs/past" are unrelated strings.

## What Changes

- Every card has a name, taken from a name template when the card has one, else from its first side.
- `[[Another card]]` in a card's text is a reference: it renders as a link, opens the target on use, and
  appears in that target's backlink list.
- A reference that matches no card stays visible as text and is listed as unresolved, so a typo is
  findable instead of silently dead.
- Tags become hierarchical: `spanish/verbs/past` nests under `spanish/verbs`, and filtering by a parent
  includes its children.
- The editor gains a backlinks panel and a reference picker that completes names as the user types.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None: additions, for the same reason the earlier clone changes gave.

## Impact

- `crates/typr-core/src/store/schema.rs`: `card.name` column with an index, and `reference(from_card,
  to_card)` rebuilt on save so backlinks are a query rather than a scan.
- `crates/typr-core/src/store/{mod,db}.rs`: `CardSummary` grows `name`; commands resolve references on
  save; a `backlinks(card)` query; tag filters compare on path segments.
- New `crates/typr-core/src/name.rs`: the single place a card's name is decided.
- `crates/typr/src/library.rs`: names in rows, hierarchical tag tree, filter by a tag's subtree.
- `crates/typr/src/editor.rs`: backlinks panel, reference picker, reference rendering.
- No dependency changes; unnamed cards keep a name derived from their text, so nothing breaks.
