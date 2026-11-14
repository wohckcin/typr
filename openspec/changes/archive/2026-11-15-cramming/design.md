# Design — cramming

## Context

`Session` holds `queue: Queue`, `entries: Vec<Entry>`, `awaiting`/`pending` for the grade in flight, and
`grades: Vec<(Id, Grade)>`. Grading goes through `State::submit_grade`, which sends `store.grade(...)`
and acknowledges on the event, so scheduling happens in the store and never in the session.

`Queue { New, Due }` is the only mode the session knows. `ViewSpec` arrives with `custom-views`, and
`Snapshot::cards_matching` already turns one into cards.

## Goals / Non-Goals

**Goals**

- A session that cannot write, structurally, rather than by remembering not to.
- The same reveal, keyboard, backward, reshuffle and summary behaviour as a real session.
- Cram filtering reuses `ViewSpec`, so a cram run and the list it came from cannot disagree.

**Non-Goals**

- Optional "record this as a review" behaviour. Mochi keeps cramming and real reviews apart, and a
  toggle would make a mistyped key rewrite a schedule.
- Time limits, goal counts, or a score target.
- Interleaving cram cards with due cards in one session.

## Decisions

**Mode lives on `Session`, and the grade path branches once.** `Session::mode() -> Mode { New, Due, Cram }`
plus a guard in `State::submit_grade`: in `Cram` the grade is applied to the session directly and no
command is sent. A separate `CramSession` type would duplicate reveal, backward, reshuffle and the
summary for one behavioural difference.

**Nothing is written, so no record can leak.** The alternative, writing records with a `cram` marker
column, would need the whole store to learn about the marker (due queries, counts, history, fold
equality). Writing nothing is a smaller change and a stronger guarantee.

**Forgot requeues to the end; only remembered removes.** This is the same rule the new-card queue
already uses for `Forgot`, so a cram session needs no new requeue logic, and it terminates because each
pass removes at least one card from the queue.

**The cap does not apply.** The cap exists to bound daily learning; a cram session is the user asking
for more, on cards they already have, without scheduling consequences.

**Saving a view is a summary action, not automatic.** A drill every day for a week should not leave five
throwaway views behind.

## Risks / Trade-offs

- A user may expect a cram answer to count. The screen names the mode and the summary says nothing was
  scheduled, which is the whole mitigation, and the first-run cost of getting it wrong is a re-cram
  rather than a corrupted schedule.
- `cards_for_view` is a second way to select cards; the tests must assert it agrees with the list for
  the same spec, mirroring the existing view-versus-snapshot agreement test.
- A cram session on a large filter mounts the whole card set in memory. That is the same order as the
  library snapshot, which already holds every card, so it is bounded by the library's size rather than
  by the session's length.
