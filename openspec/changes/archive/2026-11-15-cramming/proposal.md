# Proposal — cramming

## Why

Mochi lets you study anything right now, without waiting for a due date: pick a search, a tag or a
saved view, start a cram session, and answer the same card again and again until it sticks. Nothing is
scheduled and nothing is recorded as a review, so a cram run cannot damage a real schedule it did not
earn.

typr has one way into a session: the new queue or the due queue, scoped to a deck. With an exam
tomorrow there is no way to drill one tag, and with no cards due there is nothing to open at all.

## What Changes

- A session can be started as a cram session from whatever the library is currently filtered to: a
  deck, a saved view, a search, a tag.
- A cram session ignores due dates and the new-card cap and visits the cards the filter selects.
- A grade in a cram session writes nothing: no record, no interval, no due date. The session keeps the
  answers in memory and reports them when it ends.
- A card marked forgot in a cram session returns later in the same session, and the session can be
  repeated without limit.
- A cram session can be saved as a view, so next week's drill is one click.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None: additions again, for the same reason the earlier clone changes gave.

## Impact

- `crates/typr-core/src/session.rs`: `Session` gains a mode, and the grade path records nothing in that
  mode while still counting answers for the summary.
- `crates/typr-core/src/store/db.rs`: one new query, `cards_for_view(spec)`, returning the cards a cram
  session visits; no writes.
- `crates/typr/src/study.rs`: the mode is named on screen and the bindings are unchanged.
- `crates/typr/src/library.rs` and `crates/typr/src/main.rs`: a "Cram" action beside the two queues, and
  a "Save as view" action on the summary.
- No dependency changes; existing sessions are untouched because the default mode writes as it does
  today.
