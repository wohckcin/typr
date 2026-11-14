# Proposal — reverse reviews

## Why

Mochi lets any card be asked in both directions: the same card whose front is "What is mitosis?" can
also be reviewed with the answer showing and the question asked of it. Each direction keeps its own
history and schedule, so a card known one way does not count as known the other. typr has exactly one
prompt per two-sided card, keyed `fwd`, and a review always shows the sides in their written order.

This is the second rail of the clone. A language card with a word and its meaning is half learned
without it, and every card type added later inherits the direction rule for free.

## What Changes

- Every card that has two or more sides gains a second prompt, key `rev`, scheduled independently of
  `fwd`.
- A reverse review shows the sides in reverse order: the last side is what the reviewer sees first,
  and grading happens after the first side is revealed.
- Reverse is enabled per card, per deck, or for the whole library, with the nearest setting winning.
  Changing it affects future reviews only: no history is deleted and no interval is recomputed.
- The queue, the due ordering and the history view treat the two directions as separate prompts,
  because they already key on `(card, key)`; the per-day cap counts the card once, because it limits
  how much new material a day admits.
- The library shows which direction a card is waiting on, and the editor offers a per-card toggle.

## Capabilities

### New Capabilities

None. Both capabilities already exist and their requirement sets are extended.

### Modified Capabilities

None by refusal, by design. The live `review-scheduling` and `study-session` texts are modified by the
`cloze-deletions` change that is still open, and two changes modifying one requirement cannot both
archive. This change therefore adds requirements instead of editing those blocks, and repeats the
consequence in `design.md`.

## Impact

- `crates/typr-core/src/store/schema.rs`: migration adds `deck.reverse`, `card.reverse` and a
  `reverse_default` settings row, and recreates `studyable_prompt` so `rev` prompts queue only where
  reverse is on.
- `crates/typr-core/src/store/db.rs`: `sync_prompt` also inserts `rev`; new commands set the card and
  deck flags and the library default.
- `crates/typr-core/src/session.rs`: a reverse entry reverses its faces and reports its direction.
- `crates/typr/src/study.rs`, `crates/typr/src/library.rs`, `crates/typr/src/editor.rs`: direction
  label, queue counts, per-card toggle.
- No dependency changes. Existing libraries migrate in place; `fwd` prompts are untouched.
