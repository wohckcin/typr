# Design — reverse reviews

## Context

A prompt is a row keyed `(card_id, key)` (`crates/typr-core/src/store/schema.rs`), and studyability is
decided in the `studyable_prompt` view (migration V3): a card queues when `c.sides >= 2` and neither it
nor an ancestor deck is archived or trashed. `sync_prompt` (`crates/typr-core/src/store/db.rs`)
inserts `fwd` with `ON CONFLICT DO NOTHING`, which is what keeps an edit from resetting a schedule.

`Session::Entry` carries `card`, `sides: Vec<String>` and `overdue`; `revealed` counts how many sides
are showing. `State::session_entries` maps store answers to entries, and `Session::holds` decides
whether a prompt falling due mid-session is already in the queue.

The settings table is key/value text and today holds only `new_per_day`; `Settings` is a small `Copy`
struct read once per snapshot.

## Goals / Non-Goals

**Goals**

- Two prompts per two-sided card, each with independent state, without touching `fwd` history.
- A three-level switch (card, nearest deck, library) whose changes never rewrite history.
- Reverse faces reviewed last side first, so the reviewer answers the side that was the answer.

**Non-Goals**

- Reverse for hidden-text groups. A cloze group already is the question, so this change leaves those
  prompts alone; whether `rev` applies to them is a question for the change that adds cloze.
- Typed answers, masks, templates.
- Any change to the interval rules.

## Decisions

**`rev` rows exist for every card that carries a prompt; the view decides whether they queue.**
Enabling or disabling reverse then costs no backfill, no recursive update over a subtree, and cannot
lose a schedule, which is what "affects future reviews only" means in practice. The alternative, an
`active` flag on the prompt maintained at write time, would need a subtree pass on every deck change
and would leave the flag and the deck tree able to disagree.

**Effective reverse is computed in the view through a second recursive CTE.** `deck_reverse(id, on)`
starts at the root decks with `COALESCE(reverse, (SELECT COALESCE(CAST(value AS INTEGER), 0) FROM
settings WHERE key = 'reverse_default'))` and walks down with `COALESCE(d.reverse, parent.on)`; a
card's effective value is `COALESCE(c.reverse, deck_value, 0)`. The view gained a third disjunct,
`OR (p.key = 'rev' AND c.sides >= 2 AND <effective reverse>)`: V4's body is key-aware, so a `rev` row is
not admitted by any clause until this migration names one, and no exclusion filter is needed. The alternative, resolving in Rust and writing a
`card.reverse_enabled` cache, duplicates the chain walk that already exists for archived decks.

**The claim set includes `rev`, so the retirement pass keeps it.** `cloze-deletions` made
`sync_prompts` retire every prompt of a card and clear the mark only for the keys the shape claims; a
`rev` row inserted outside that list is retired by the same call and never queues. The claim set is
therefore the shape's keys plus `rev` for a card with two or more sides, which also keeps the promise
that enabling reverse writes nothing: the row exists from the moment the card is two-sided, and the
view decides whether it queues.

**Flag columns are nullable on purpose.** `deck.reverse INTEGER` and `card.reverse INTEGER` use NULL
for "inherit", 0 for off and 1 for on, so the nearest explicit setting wins and inheritance needs no
extra table.

**Migration V5 recreates the studyable view from the one V4 leaves, not from the V3 text quoted above.** Three changes in this roadmap each recreate `studyable_prompt`, so each copies the body the previous migration applied and widens one predicate; restating it from an older baseline would silently drop `c.has_cloze`.

**A reverse entry reverses `faces`, not the card.** `Session` builds `faces` from `sides` and, when
`prompt` starts with `rev`, reverses the vector. Everything downstream (`revealed`, `read_only`, the
grade gate, `reshuffle`) keeps working because it only counts faces.

**The cap counts cards, not directions.** The queue holds a prompt per direction, but the allowance is
a limit on how much new material a day admits, so `new_prompts` admits whole cards against it and
`learned_on` counts a card whose first `review`-phase record falls on that day. Counting directions
would halve the day's new material for every card asked both ways.

**`cloze-deletions` owns the `Each card has at most one prompt` block and this change does not touch
it.** Two open changes cannot both modify one requirement and archive, and the cloze variant machinery
lands first, so this change adds requirements beside it. Whichever of the two archives second will
carry the prompt-count wording forward.

## Risks / Trade-offs

- Every two-sided card gains a `rev` row that may never queue. That is a row per card in a file that is
  already the source of truth; the alternative costs a subtree rewrite on each toggle.
- A view with two recursive CTEs (archived decks, deck reverse) is harder to read. The existing
  `due_prompts`/`new_prompts` callers are unchanged, and the agreement test between the view and the
  snapshot keeps them honest.
- Reverse doubles the work for cards that are enabled, which is the point, and it also doubles the
  new queue for those cards; the cap does not absorb that, because it counts cards. The library
  default is off and every deck and card can say otherwise, so nothing changes until asked.
- A card toggled per card while its deck flips later keeps its own value, which can surprise; the row
  says the card is asked in both directions, so the state is visible where the toggle lives.
