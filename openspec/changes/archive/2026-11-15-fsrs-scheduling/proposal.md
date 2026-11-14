# Proposal — FSRS scheduling

## Why

typr schedules with fixed multipliers: remembered doubles and a half, forgot resets to ten minutes. It
is predictable and it is also indifferent to how hard each card is. A card answered slowly and wrongly
again and again keeps the same multiplier as one that has never been missed.

Mochi offers FSRS as an alternative: it models a card's stability and difficulty, aims at a target
retention (90% by default), and can be tuned from the card's own review history. For a library that has
been studied for months, that is the difference between reviewing thirty cards and reviewing ninety.

## What Changes

- A scheduler setting: the existing fixed multipliers, or FSRS.
- Per-prompt stability and difficulty, filled by FSRS and ignored by the fixed scheduler.
- A target retention setting, default 90%, which FSRS uses to choose intervals.
- FSRS weights are stored so they can be replaced wholesale, and the app exports the review history in
  the shape an external optimiser reads.
- Switching schedulers affects future answers only: no history is deleted, and a prompt that has no
  stability yet is bootstrapped from the interval it already has.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None: the interval rules in `review-scheduling` describe the fixed scheduler, which stays the default,
and FSRS is added beside it as its own requirement.

## Impact

- New `crates/typr-core/src/fsrs.rs`: the algorithm as a pure function of weights, a memory state and a
  grade, with no I/O and no clock.
- `crates/typr-core/src/schedule.rs`: `Scheduler` gains an FSRS implementation beside `FixedMultiplier`,
  chosen by the settings.
- `crates/typr-core/src/store/schema.rs`: `prompt.stability REAL`, `prompt.difficulty REAL`, and three
  settings rows (`scheduler`, `retention`, `fsrs_weights`).
- `crates/typr-core/src/store/db.rs`: `grade` passes the memory state through, and `export_reviews()`
  returns the history in the optimiser's shape.
- `crates/typr/src/`: the setting is changed from the settings screen once it exists; until then a
  command and a test-only path set it.
- No new dependency: the algorithm is implemented in the crate, because the backend's dependency graph
  is a stated invariant and a scheduling formula is not worth outsourcing.
