# Design — card names and references

## Context

`card.markdown` holds the rendered text, `search_text` its folded form, and `sides` the count that makes
a card studyable. `CardSummary.preview` is the first line truncated to eighty characters, which is what
the list shows today. Tags live in `tag(id, name COLLATE NOCASE UNIQUE)` and `card_tag`, normalised to
one spelling per distinct name, with no notion of hierarchy.

The editor renders markdown through `rune_kit`, whose link handling covers URLs only, so `[[…]]` needs a
pass before parsing, exactly as `attachments` supplies an image resolver.

## Goals / Non-Goals

**Goals**

- A name that is stable, derived, and searchable, with no migration of existing cards.
- References that cannot corrupt text: resolution is a read, never a rewrite.
- One tag tree, so a filter has one meaning.

**Non-Goals**

- Renaming a card's references when the target changes. Mochi does not do it either, and guessing at
  intent from a string is worse than leaving a visible unresolved link.
- A graph view or a global backlinks screen.
- Aliases and redirects; a rename leaves the references to be fixed deliberately.

## Decisions

**Names are derived, with an explicit template override.** `name::for_card(first_side, template, fields)`
is pure and called at write time, so `card.name` is a stored column and the list, the reference resolver
and search all read one value. A stored name is also what makes `[[…]]` resolution a single indexed
lookup instead of a text scan.

**References are resolved when a card is saved, into a `reference` table.** `reference(from_card,
to_card)` is rebuilt for the saving card in the same transaction as its text, which keeps backlinks a
join and lets an unresolved name simply have no row. Resolving at render time would put a lookup in
every frame of the review screen.

**Reference resolution prefers the referencing card's deck, then its ancestors, then the library.** That
matches how a deck of notes refers to its own "Introduction" rather than a distant one, and it is the
order Mochi's names imply. Ties beyond that are broken by the oldest card, so the same text always
opens the same card.

**Unresolved references stay in the text.** Rewriting them away would lose the author's words and make a
typo unrecoverable; leaving them visible with a warning mark is also what makes a renamed target
findable.

**Tags nest by path, and descendants always count.** A tag is stored whole, as one string with slashes,
and the filter compares path prefixes; the alternative, a `tag_tree` table with parent ids, needs
migration and pruning for no gain, because a tag's segments are never renamed independently of the tag.

**A reference is a markdown link after one pass.** `[[Name]]` becomes `[Name](typr-card:<id>)` before the
markdown parser sees the text, and the rendered link handler intercepts the `typr-card:` scheme to open
that card. The stored text keeps `[[Name]]`, so the pass is re-run on every render and the store's
`search_text` keeps the author's form.

## Risks / Trade-offs

- Names can be long and repetitive, so a name template is the fix rather than truncation; `Name` shares
  a column with the derived value, so a card renamed by editing its text silently changes what `[[…]]`
  matches. The unresolved-reference report is what surfaces it.
- `[[` in prose about markdown itself; fenced and inline code are skipped, as hidden text already is.
- Two cards with one name in one deck get a suffix, which makes their names differ from what a human
  would write. Names stay stable because the suffix is applied at write time, not per render.
- Rebuilding `reference` rows on every save is a delete-and-insert of a handful of rows; it cannot be
  allowed outside the save's transaction, which is where the invariant lives.
