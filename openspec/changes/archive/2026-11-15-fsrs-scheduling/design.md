# Design — FSRS scheduling

## Context

`crates/typr-core/src/schedule.rs` already defines `trait Scheduler { fn grade(&self, state:
PromptState, grade: Grade, at_ms: i64) -> Outcome }` and one implementation, `FixedMultiplier`. `Db::grade`
reads the prompt's `phase` and `interval_ms`, asks the scheduler, appends one `review_log` row carrying
the outcome and rewrites the prompt from the log (`refresh_prompt`).

That shape is the whole reason this change is additive: FSRS needs two more numbers per prompt and a
different function, not a different pipeline.

## Goals / Non-Goals

**Goals**

- FSRS behind the same trait, so the review loop, the store and the session do not learn about it.
- A scheduler switch that cannot lose or rewrite history.
- Testable against published reference values, not against intuition.

**Non-Goals**

- Running the weight optimiser inside the app. It needs many reviews and a solver; the app exports the
  data and accepts weights.
- Per-deck or per-card FSRS. Mochi switches it for the library.
- Fuzzing intervals or same-day (short-term) scheduling beyond the existing learning step.

## Decisions

**The formula lives in `crates/typr-core/src/fsrs.rs` with no dependency.** The `fsrs` crate exists, but
adding it to the backend for one pure function imports its transitive graph (chrono, serde, ndarray) and
puts a versioned third party between typr and its own schedule. FSRS-5 is a page of arithmetic: `w`
weights, a stability update per grade, a difficulty update with mean reversion, and a
`interval = 9 * (stability / FACTOR) * (retention^(-1/DECAY) - 1)` step. Pinning it here keeps the numbers
inspectable and testable.

**Golden tests against published examples decide correctness.** The unit tests carry a table of
(stability, difficulty, grade, elapsed days) to (new stability, new difficulty, interval) taken from the
reference implementation's own worked examples, so a typo in the exponent fails a test rather than
drifting silently. If a discrepancy turns up, the reconciliation goes in the design, not in a loosened
assertion.

**Stability and difficulty are nullable columns on `prompt`.** Existing prompts have none, and FSRS
bootstraps stability from the current interval the first time it grades one (`stability =
interval_days.clamp(0.1, MAX)`), which is the documented behaviour for migrating a deck into FSRS.

**The settings are three rows, not a struct column.** `scheduler` (`fixed` or `fsrs`), `retention`
(integer percent) and `fsrs_weights` (comma-separated, validated to the expected count) reuse the
existing `settings` table and its `StoreError::Setting` refusal, which is already how `new_per_day`
reports a bad value.

**The trait stays synchronous and pure.** FSRS needs the elapsed days, which the caller supplies as
the time since the prompt was last reviewed (`at_ms - prompt.last_at`, falling back to
`due_at - interval_ms` for a prompt the fixed scheduler scheduled and never replayed), not the time
since it fell due: a card answered exactly on time has still waited a whole interval, and anchoring
on the due date would score it `t = 0`, hold retrievability at 1 and freeze stability. Nothing in the
algorithm needs a clock or a database, so the trait signature does not change and both schedulers
remain trivially testable.

## Risks / Trade-offs

- Getting FSRS subtly wrong is worse than not having it: an over-long interval hides a card for months.
  Mitigations are the golden tests, the off-by-default setting, and keeping the fixed scheduler one
  click away.
- Exporting history for the optimiser means a stable output format. It is the optimiser's own review
  log schema (`card_id,review_time,review_rating,review_state,review_duration`, from its README), not
  a layout of our choosing: the numbers are its numbers and the state is the state before the grade,
  because a format we invent is a format no optimiser reads.
- FSRS's first-grade behaviour differs from the fixed scheduler's ten-minute learning step; the spec
  requires the fixed path to be unchanged, so a switch is visible in the first review rather than
  hidden.
- The weights list is nineteen numbers today and FSRS revisions change the count. A stored list whose
  length differs from the implementation's expectation is refused rather than padded, so an upgrade
  cannot silently reinterpret old weights.
