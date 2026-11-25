//! Spaced repetition: what one grade does to one prompt.
//!
//! Mochi's own constants are undocumented, so this copies its *shape* — two binary grades, a
//! ten-minute entry step, an adaptive interval after that — rather than its numbers. The trait is
//! the seam FSRS slots into later: a different implementation, the same call.

use crate::day::DAY_MS as ONE_DAY_MS;
use crate::fsrs::{self, Memory};
use crate::model::{Grade, Phase, Settings};

pub use crate::day::{DAY_MS, MINUTE_MS};

/// The step a new card's first success, and every failure, waits.
pub const LEARNING_STEP_MS: i64 = 10 * MINUTE_MS;

/// How much a success in the review phase stretches the interval.
const MULTIPLIER_NUM: i64 = 5;
const MULTIPLIER_DEN: i64 = 2;

/// The longest interval a prompt can reach before a library asks for another cap.
pub const MAX_INTERVAL_MS: i64 = 365 * DAY_MS;

/// The bounds a library's settings put on its schedule. The defaults are what a library that has
/// never touched the two settings uses, so an unconfigured scheduler behaves exactly as before.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Policy {
    /// How long a learning or relearning step waits.
    pub learning_step_ms: i64,
    /// The furthest ahead a remembered card can be scheduled.
    pub max_interval_ms: i64,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            learning_step_ms: LEARNING_STEP_MS,
            max_interval_ms: MAX_INTERVAL_MS,
        }
    }
}

impl Policy {
    /// The bounds the library's settings ask for.
    pub fn from_settings(settings: &Settings) -> Self {
        Self {
            learning_step_ms: settings.learning_step_ms(),
            max_interval_ms: settings.max_interval_ms(),
        }
    }
}

/// What the scheduler needs to know about a prompt.
///
/// The fields after the first two are what FSRS needs and the fixed scheduler ignores: when the
/// prompt fell due, when it was last reviewed (elapsed time is `at_ms - last_at`), and the memory
/// state the last grade left behind.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PromptState {
    pub phase: Phase,
    pub interval_ms: i64,
    pub due_at: Option<i64>,
    /// When this prompt was last reviewed. FSRS measures elapsed time from here, not from the due
    /// date: a card answered on time has still waited a whole interval.
    pub last_at: Option<i64>,
    pub memory: Option<Memory>,
}

impl PromptState {
    /// A prompt that has never been graded.
    pub const NEW: Self = Self {
        phase: Phase::Learn,
        interval_ms: 0,
        due_at: None,
        last_at: None,
        memory: None,
    };
}

/// What a grade decided.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Outcome {
    pub phase: Phase,
    pub interval_ms: i64,
    pub due_at: Option<i64>,
    /// The memory FSRS left behind, for the store to keep; the fixed scheduler has none.
    pub memory: Option<Memory>,
}

impl Outcome {
    /// A scheduling outcome from its computed fields.
    pub fn new(
        phase: Phase,
        interval_ms: i64,
        due_at: Option<i64>,
        memory: Option<Memory>,
    ) -> Self {
        Self {
            phase,
            interval_ms,
            due_at,
            memory,
        }
    }
}

/// How a prompt moves when it is graded.
pub trait Scheduler: Send + Sync {
    fn grade(&self, state: &PromptState, grade: Grade, at_ms: i64, policy: Policy) -> Outcome;
}

/// Mochi's entry steps with a fixed multiplier after them.
#[derive(Clone, Copy, Debug, Default)]
pub struct FixedMultiplier;

/// An outcome from a scheduler that keeps no memory: the only field the fixed path does not fill.
fn fixed(phase: Phase, interval_ms: i64, due_at: Option<i64>) -> Outcome {
    Outcome {
        phase,
        interval_ms,
        due_at,
        memory: None,
    }
}

impl Scheduler for FixedMultiplier {
    fn grade(&self, state: &PromptState, grade: Grade, at_ms: i64, policy: Policy) -> Outcome {
        match (state.phase, grade) {
            // A failure before the card has ever been scheduled changes nothing: it stays new.
            (Phase::Learn, Grade::Forgot) => fixed(Phase::Learn, state.interval_ms, None),
            // The first success takes the card out of the new queue and sets a ten-minute step.
            (Phase::Learn, Grade::Remembered) => {
                fixed(Phase::Review, 0, Some(at_ms + policy.learning_step_ms))
            }
            // The step's success, and any success after a failure, starts from one day.
            (Phase::Review, Grade::Remembered) if state.interval_ms == 0 => {
                fixed(Phase::Review, ONE_DAY_MS, Some(at_ms + ONE_DAY_MS))
            }
            (Phase::Relearn, Grade::Remembered) => {
                fixed(Phase::Review, ONE_DAY_MS, Some(at_ms + ONE_DAY_MS))
            }
            (Phase::Review, Grade::Remembered) => {
                let stretched = state.interval_ms * MULTIPLIER_NUM / MULTIPLIER_DEN;
                let interval = stretched.clamp(ONE_DAY_MS, policy.max_interval_ms);
                fixed(Phase::Review, interval, Some(at_ms + interval))
            }
            // A failure in the review or relearning phase drops into a ten-minute step, keeping the
            // interval it had reached so the recovery starts short without forgetting how far it got.
            (Phase::Review | Phase::Relearn, Grade::Forgot) => fixed(
                Phase::Relearn,
                state.interval_ms,
                Some(at_ms + policy.learning_step_ms),
            ),
        }
    }
}

/// FSRS-5: stability and difficulty decide the interval, and a target retention decides how much of
/// the stability is spent.
///
/// The fixed path's learning step is not reproduced: a new card graded remembered is scheduled for
/// whatever its first stability asks (about three days at the defaults), and a failure is scheduled
/// from the stability it fell to. That difference is the point of switching, so it is visible in the
/// first review rather than hidden behind a ten-minute step.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Fsrs {
    pub weights: Vec<f64>,
    /// The chance of recall each review aims at, from 0.05 to 0.99.
    pub retention: f64,
}

impl Fsrs {
    /// The scheduler the settings ask for before anyone changes them.
    pub fn defaults() -> Self {
        Self {
            weights: fsrs::DEFAULT_WEIGHTS.to_vec(),
            retention: fsrs::DEFAULT_RETENTION,
        }
    }

    /// How many days the card has been held, which is the time since its last review.
    ///
    /// Anchoring on the due date instead would make an on-time review look instantaneous: its
    /// retrievability would be 1, its stability unchanged, and the card would never leave the
    /// interval it started with. A prompt the fixed scheduler scheduled has no record of a review
    /// time to read here, so its last review is where the interval that made it due began.
    fn elapsed_days(state: &PromptState, at_ms: i64) -> f64 {
        let held_since = state
            .last_at
            .or_else(|| state.due_at.map(|due| due - state.interval_ms));
        match held_since {
            None => 0.0,
            Some(since) => ((at_ms - since).max(0) as f64) / (ONE_DAY_MS as f64),
        }
    }
}

impl Scheduler for Fsrs {
    fn grade(&self, state: &PromptState, grade: Grade, at_ms: i64, policy: Policy) -> Outcome {
        let memory = match (state.memory, state.due_at) {
            // Nothing has graded this prompt before: its first memory comes from the grade alone.
            (None, None) => fsrs::initial(grade, &self.weights),
            // A prompt FSRS is meeting for the first time keeps the interval it already reached.
            (None, Some(_)) => fsrs::update(
                Memory::bootstrapped(state.interval_ms),
                grade,
                Self::elapsed_days(state, at_ms),
                &self.weights,
            ),
            (Some(memory), _) => fsrs::update(
                memory,
                grade,
                Self::elapsed_days(state, at_ms),
                &self.weights,
            ),
        };
        let days = fsrs::interval_days(memory.stability, self.retention);
        let interval = ms_from_days(days, policy.max_interval_ms)
            .clamp(policy.learning_step_ms, policy.max_interval_ms);
        let (phase, due) = match grade {
            Grade::Remembered => (Phase::Review, at_ms + interval),
            Grade::Forgot => (Phase::Relearn, at_ms + interval),
        };
        Outcome {
            phase,
            interval_ms: interval,
            due_at: Some(due),
            memory: Some(memory),
        }
    }
}

/// How many milliseconds a span of days is.
///
/// A float-to-integer cast would be the obvious spelling, and this workspace refuses lossy casts;
/// going through `Duration` rounds the same way without one. The clamp keeps the input inside the
/// range a duration accepts, so the conversion cannot fail.
fn ms_from_days(days: f64, cap_ms: i64) -> i64 {
    let ceiling = (cap_ms as f64) / (ONE_DAY_MS as f64);
    let bounded = days.clamp(0.0, ceiling);
    let millis =
        std::time::Duration::from_secs_f64(bounded * (ONE_DAY_MS as f64) / 1_000.0).as_millis();
    i64::try_from(millis).unwrap_or(cap_ms)
}

/// How a card's schedule reads in the library.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Schedule {
    /// Never graded: the card is new.
    #[default]
    New,
    /// Graded and due later: the whole days until it comes back.
    Interval(u32),
    /// Graded and due now.
    Due,
}

/// The schedule state of a card at `now_ms`.
pub fn schedule_of(reviews: u32, due_at: Option<i64>, now_ms: i64) -> Schedule {
    match due_at {
        _ if reviews == 0 || due_at.is_none() => Schedule::New,
        Some(due) if due <= now_ms => Schedule::Due,
        Some(due) => {
            Schedule::Interval(u32::try_from((due - now_ms) / ONE_DAY_MS).unwrap_or(u32::MAX))
        }
        None => Schedule::New,
    }
}

#[cfg(test)]
mod schedule_tests {
    use super::*;

    #[test]
    fn schedule_of_classifies_new_due_and_future_intervals() {
        assert_eq!(schedule_of(0, None, 1_000), Schedule::New);
        assert_eq!(schedule_of(0, Some(500), 1_000), Schedule::New);
        assert_eq!(schedule_of(1, None, 1_000), Schedule::New);
        assert_eq!(schedule_of(1, Some(1_000), 1_000), Schedule::Due);
        assert_eq!(schedule_of(1, Some(500), 1_000), Schedule::Due);
        assert_eq!(
            schedule_of(1, Some(1_000 + 5 * ONE_DAY_MS), 1_000),
            Schedule::Interval(5)
        );
    }
}

#[cfg(test)]
mod tests;
