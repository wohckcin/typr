//! The study day: a local calendar day that rolls over at 04:00.
//!
//! A queue named "due today" has to agree with the cap on new cards and with the statistics
//! window about which day a moment belongs to. Rolling over at four in the morning keeps a session
//! that runs past midnight inside the same day, which is what a user expects and what Mochi does
//! with its tomorrow/next-due split.

use std::sync::LazyLock;

use crate::schedule::DAY_MS;

/// When one study day becomes the next, counted from local midnight.
pub const ROLLOVER_MS: i64 = 4 * 60 * 60 * 1000;

/// The machine's offset east of UTC, in milliseconds.
///
/// `time` can only answer this on the main thread of a single-threaded program, so the app reads it
/// once at startup and every later reader reuses the answer. A program that cannot know its zone
/// (a test, a locked-down sandbox) studies on UTC days, which is wrong by hours at worst and never
/// inconsistent with itself.
pub fn local_offset_ms() -> i64 {
    *LOCAL_OFFSET_MS
}

/// Resolved on first use, which the app arranges to happen on the main thread before it starts the
/// store task.
static LOCAL_OFFSET_MS: LazyLock<i64> = LazyLock::new(|| {
    match time::UtcOffset::current_local_offset() {
        Ok(offset) => i64::from(offset.whole_seconds()) * 1000,
        Err(_) => 0,
    }
});

/// Which study day `at_ms` falls on, counting from the epoch.
pub fn study_day(at_ms: i64, offset_ms: i64) -> i64 {
    (at_ms + offset_ms - ROLLOVER_MS).div_euclid(DAY_MS)
}

/// The first and last instant of a study day, as `[from, to)`.
pub fn study_day_bounds(day: i64, offset_ms: i64) -> (i64, i64) {
    let from = day * DAY_MS + ROLLOVER_MS - offset_ms;
    (from, from + DAY_MS)
}

#[cfg(test)]
mod tests;
