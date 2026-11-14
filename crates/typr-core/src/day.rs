//! The study day: a local calendar day that rolls over at 04:00.
//!
//! A queue named "due today" has to agree with the cap on new cards and with the statistics
//! window about which day a moment belongs to. Rolling over at four in the morning keeps a session
//! that runs past midnight inside the same day, which is what a user expects and what Mochi does
//! with its tomorrow/next-due split.

/// One day in milliseconds.
pub const DAY_MS: i64 = 24 * 60 * 60 * 1000;

/// When one study day becomes the next, counted from local midnight.
pub const ROLLOVER_MS: i64 = 4 * 60 * 60 * 1000;

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
