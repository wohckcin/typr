//! Spaced repetition: what one grade does to one prompt.
//!
//! Mochi's own constants are undocumented, so this copies its *shape* — two binary grades, a
//! ten-minute entry step, an adaptive interval after that — rather than its numbers. The trait is
//! the seam FSRS slots into later: a different implementation, the same call.

use crate::store::{Grade, Phase};

pub use typr_core::day::DAY_MS;

pub const MINUTE_MS: i64 = 60_000;

/// The step a new card's first success, and every failure, waits.
pub const LEARNING_STEP_MS: i64 = 10 * MINUTE_MS;

/// How much a success in the review phase stretches the interval.
const MULTIPLIER_NUM: i64 = 5;
const MULTIPLIER_DEN: i64 = 2;

/// The longest interval a prompt can reach.
pub const MAX_INTERVAL_MS: i64 = 365 * DAY_MS;

/// What the scheduler needs to know about a prompt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PromptState {
    pub phase: Phase,
    pub interval_ms: i64,
}

impl PromptState {
    /// A prompt that has never been graded.
    pub const NEW: Self = Self {
        phase: Phase::Learn,
        interval_ms: 0,
    };
}

/// What a grade decided.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub phase: Phase,
    pub interval_ms: i64,
    pub due_at: Option<i64>,
}

/// How a prompt moves when it is graded.
pub trait Scheduler: Send + Sync {
    fn grade(&self, state: &PromptState, grade: Grade, at_ms: i64) -> Outcome;
}

/// Mochi's entry steps with a fixed multiplier after them.
#[derive(Clone, Copy, Debug, Default)]
pub struct FixedMultiplier;

impl Scheduler for FixedMultiplier {
    fn grade(&self, state: &PromptState, grade: Grade, at_ms: i64) -> Outcome {
        match (state.phase, grade) {
            // A failure before the card has ever been scheduled changes nothing: it stays new.
            (Phase::Learn, Grade::Forgot) => Outcome {
                phase: Phase::Learn,
                interval_ms: state.interval_ms,
                due_at: None,
            },
            // The first success takes the card out of the new queue and sets a ten-minute step.
            (Phase::Learn, Grade::Remembered) => Outcome {
                phase: Phase::Review,
                interval_ms: 0,
                due_at: Some(at_ms + LEARNING_STEP_MS),
            },
            // The step's success, and any success after a failure, starts from one day.
            (Phase::Review, Grade::Remembered) if state.interval_ms == 0 => Outcome {
                phase: Phase::Review,
                interval_ms: DAY_MS,
                due_at: Some(at_ms + DAY_MS),
            },
            (Phase::Relearn, Grade::Remembered) => Outcome {
                phase: Phase::Review,
                interval_ms: DAY_MS,
                due_at: Some(at_ms + DAY_MS),
            },
            (Phase::Review, Grade::Remembered) => {
                let stretched = state.interval_ms * MULTIPLIER_NUM / MULTIPLIER_DEN;
                let interval = stretched.clamp(DAY_MS, MAX_INTERVAL_MS);
                Outcome {
                    phase: Phase::Review,
                    interval_ms: interval,
                    due_at: Some(at_ms + interval),
                }
            }
            // A failure in the review or relearning phase drops into a ten-minute step, keeping the
            // interval it had reached so the recovery starts short without forgetting how far it got.
            (Phase::Review | Phase::Relearn, Grade::Forgot) => Outcome {
                phase: Phase::Relearn,
                interval_ms: state.interval_ms,
                due_at: Some(at_ms + LEARNING_STEP_MS),
            },
        }
    }
}

#[cfg(test)]
mod tests;
