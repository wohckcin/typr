//! FSRS-5: how long a card can wait and still be remembered at a target retention.
//!
//! The algorithm models two numbers per prompt — stability (the interval at which recall is 90%
//! likely) and difficulty (how much the material resists stabilising) — and moves them on each
//! grade. Nothing here reads a clock or a database: elapsed time arrives as a parameter, and the
//! weights arrive as a slice, so a test can pin a published formula instead of a wall clock.
//!
//! The formulas are FSRS-5's, from the `open-spaced-repetition/awesome-fsrs` wiki *The Algorithm*,
//! FSRS-5 section: `R(t,S) = (1 + FACTOR * t / S)^DECAY` with `DECAY = -0.5` and `FACTOR = 19/81`,
//! so `R = 0.9` exactly when `t = S`; `I(r,S) = S / FACTOR * (r^(1/DECAY) - 1)` for a target
//! retention; and difficulty drifts by `-w6 * (G-3)` damped by `(10 - D) / 9` before mean reversion
//! towards `D_0(4)`.
//!
//! FSRS-5's same-day stability update (`w17`, `w18`) is not applied: this build keeps the fixed
//! scheduler's ten-minute learning step and the change that introduced FSRS leaves short-term
//! scheduling out of scope, so a review in the learning phase is treated as any other review.
//!
//! Implementing it here rather than depending on the `fsrs` crate keeps the backend's dependency
//! graph what it is: arithmetic, no `chrono`, no `serde`, no `ndarray`.

use crate::error::StoreError;
use crate::model::Grade;
pub use crate::model::{DEFAULT_RETENTION, DEFAULT_WEIGHTS, WEIGHT_COUNT};

/// The forgetting curve's shape. `R = 0.9` at `t = S` only for this pair, so they travel together.
const DECAY: f64 = -0.5;
const FACTOR: f64 = 19.0 / 81.0;

/// Stability never reaches zero: a card just forgotten is still worth something later than now.
const MIN_STABILITY: f64 = 0.1;

/// Difficulty is bounded by the algorithm, not by its inputs.
const MIN_DIFFICULTY: f64 = 1.0;
const MAX_DIFFICULTY: f64 = 10.0;

/// FSRS's fourth rating, "easy", which this build never sends but its mean reversion is anchored to.
const EASY: f64 = 4.0;

/// What FSRS remembers about a prompt between reviews.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct Memory {
    /// Days at which recall is 90% likely.
    pub stability: f64,
    /// How hard the card is to stabilise, from 1 (easy) to 10 (hard).
    pub difficulty: f64,
}

impl Memory {
    /// The state a prompt had before FSRS knew it, for a scheduler that must answer a grade without
    /// one: the interval it already reached, and middling difficulty.
    pub fn bootstrapped(interval_ms: i64) -> Self {
        let days = interval_ms as f64 / crate::day::DAY_MS as f64;
        Self {
            stability: days.clamp(MIN_STABILITY, f64::MAX),
            difficulty: (MIN_DIFFICULTY + MAX_DIFFICULTY) / 2.0,
        }
    }
}

/// Two grades become two of FSRS's four ratings: a miss is `again`, a hit is `good`.
///
/// `hard` and `easy` are the rating scale's other two points, reachable only by a UI that offers four
/// buttons; typr's review screen offers two, so the scheduler maps onto the same scale instead of
/// inventing a fifth.
fn rating(grade: Grade) -> f64 {
    f64::from(rating_of(grade))
}

/// The rating the optimiser's review log uses: `1` for again, `3` for good.
///
/// The exporter writes this, and `rating` feeds the same number to the formula, so a grade cannot
/// mean one thing in the maths and another in the file.
pub fn rating_of(grade: Grade) -> u8 {
    match grade {
        Grade::Forgot => 1,
        Grade::Remembered => 3,
    }
}

/// The stability a first grade gives: `S_0(G) = w[G-1]`, which is `w0` for again and `w2` for good.
fn initial_stability(grade: Grade, weights: &[f64]) -> f64 {
    let index = match grade {
        Grade::Forgot => 0,
        Grade::Remembered => 2,
    };
    weights[index].max(MIN_STABILITY)
}

/// `D_0(G) = w4 - e^(w5 * (G-1)) + 1`, so `D_0(1) = w4`.
fn difficulty_of(rating: f64, weights: &[f64]) -> f64 {
    let offset = rating - 1.0;
    (weights[4] - (weights[5] * offset).exp() + 1.0).clamp(MIN_DIFFICULTY, MAX_DIFFICULTY)
}

/// The difficulty a first grade gives.
fn initial_difficulty(grade: Grade, weights: &[f64]) -> f64 {
    difficulty_of(rating(grade), weights)
}

/// How likely the card is to be recalled after `elapsed_days`.
fn retrievability(elapsed_days: f64, stability: f64) -> f64 {
    let days = elapsed_days.max(0.0);
    (1.0 + FACTOR * days / stability.max(MIN_STABILITY)).powf(DECAY)
}

/// The memory a card starts with when it is graded for the first time.
pub fn initial(grade: Grade, weights: &[f64]) -> Memory {
    Memory {
        stability: initial_stability(grade, weights),
        difficulty: initial_difficulty(grade, weights),
    }
}

/// The memory after a grade `elapsed_days` since the prompt fell due.
///
/// A hit stretches stability by however much the wait and the difficulty allow — a card recalled
/// after a long gap is stronger than one recalled on time — and a miss drops it to the post-lapse
/// value. Difficulty rises on a miss and falls on a hit, with a pull back towards the easy end so a
/// repeatedly missed card does not sink to the bottom forever.
pub fn update(memory: Memory, grade: Grade, elapsed_days: f64, weights: &[f64]) -> Memory {
    let stability = memory.stability.max(MIN_STABILITY);
    let retrieval = retrievability(elapsed_days, stability);
    let stability = match grade {
        Grade::Remembered => {
            let growth = (weights[8]).exp()
                * (11.0 - memory.difficulty)
                * stability.powf(-weights[9])
                * ((weights[10] * (1.0 - retrieval)).exp() - 1.0);
            stability * (1.0 + growth)
        }
        // `w15` and `w16` scale a `hard` and an `easy` hit; the two grades here are neither.
        Grade::Forgot => {
            let lapsed = weights[11]
                * memory.difficulty.powf(-weights[12])
                * ((stability + 1.0).powf(weights[13]) - 1.0)
                * (weights[14] * (1.0 - retrieval)).exp();
            lapsed.min(stability)
        }
    };
    let drifted = memory.difficulty - weights[6] * (rating(grade) - 3.0);
    // The drift is damped as difficulty approaches ten, so a card that is already hard cannot be
    // pushed past the end of the scale by one more miss.
    let damped = memory.difficulty
        + (drifted - memory.difficulty) * (MAX_DIFFICULTY - memory.difficulty)
            / (MAX_DIFFICULTY - MIN_DIFFICULTY);
    // Mean reversion pulls towards the *easy* anchor, not towards this grade's own start, which is
    // what keeps a repeatedly missed card from sinking forever.
    let difficulty = weights[7] * difficulty_of(EASY, weights) + (1.0 - weights[7]) * damped;
    Memory {
        stability: stability.clamp(MIN_STABILITY, f64::MAX),
        difficulty: difficulty.clamp(MIN_DIFFICULTY, MAX_DIFFICULTY),
    }
}

/// The interval, in days, that leaves the card `retention` likely to be recalled.
///
/// `I(0.9, S) == S`: the stability *is* the ninety-percent interval, so a library at the default
/// retention reviews each card when the model expects to have to remind it.
pub fn interval_days(stability: f64, retention: f64) -> f64 {
    let retention = retention.clamp(0.05, 0.99);
    let days = (stability.max(MIN_STABILITY) / FACTOR) * (retention.powf(1.0 / DECAY) - 1.0);
    days.max(MIN_STABILITY)
}

/// A weight list produced elsewhere, refused whole if any part of it could not have come from an
/// optimiser.
pub fn weights(text: &str) -> Result<Vec<f64>, StoreError> {
    let refused = || StoreError::Setting {
        key: "fsrs_weights".to_string(),
        value: text.to_string(),
    };
    #[expect(clippy::map_err_ignore, reason = "the text is in Setting")]
    let parsed = text
        .split(',')
        .map(|part| part.trim().parse::<f64>())
        .collect::<Result<Vec<f64>, _>>()
        .map_err(|_| refused())?;
    if !are_weights(&parsed) {
        return Err(refused());
    }
    Ok(parsed)
}

/// Whether a list could have come from an optimiser: the right count, every number finite and
/// positive. The one rule both the text parser and a typed setting value go through.
pub fn are_weights(weights: &[f64]) -> bool {
    weights.len() == WEIGHT_COUNT && weights.iter().all(|w| w.is_finite() && *w > 0.0)
}

/// The list in the shape [`weights`] reads back, so a stored default and a pasted list look alike.
pub fn weight_text(weights: &[f64]) -> String {
    weights
        .iter()
        .map(|w| w.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests;
