//! Where the app reads the current time.
//!
//! Scheduling decisions take a `&dyn Clock` rather than calling the system clock, so a test can
//! put a prompt ten minutes out and step over it without sleeping.

use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// The length of a learning step: a new card's first success, and a failure's relearning step.
pub const LEARNING_STEP_MS: i64 = 10 * 60 * 1000;

/// A source of the current time.
pub trait Clock {
    /// Milliseconds since the Unix epoch.
    fn now_ms(&self) -> i64;
}

/// The real clock.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_ms(&self) -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
            .unwrap_or(0)
    }
}

/// A clock a test moves by hand.
#[derive(Debug)]
pub struct TestClock(AtomicI64);

impl TestClock {
    pub fn new(now_ms: i64) -> Self {
        Self(AtomicI64::new(now_ms))
    }

    /// Move time forward by `ms` milliseconds.
    pub fn advance(&self, ms: i64) {
        self.0.fetch_add(ms, Ordering::Relaxed);
    }
}

impl Clock for TestClock {
    fn now_ms(&self) -> i64 {
        self.0.load(Ordering::Relaxed)
    }
}

/// Whether something due at `due_at_ms` is due to `clock`.
pub fn is_due(clock: &dyn Clock, due_at_ms: i64) -> bool {
    due_at_ms <= clock.now_ms()
}

#[cfg(test)]
mod tests {
    use super::{LEARNING_STEP_MS, SystemClock, TestClock, is_due};
    use crate::clock::Clock;

    #[test]
    fn a_prompt_ten_minutes_out_is_not_due_until_the_clock_reaches_it() {
        let clock = TestClock::new(1_700_000_000_000);
        let due_at = clock.now_ms() + LEARNING_STEP_MS;

        assert!(!is_due(&clock, due_at));
        clock.advance(LEARNING_STEP_MS);
        assert!(is_due(&clock, due_at));
    }

    #[test]
    fn a_prompt_due_exactly_now_is_due() {
        let clock = TestClock::new(1_700_000_000_000);
        assert!(is_due(&clock, clock.now_ms()));
    }

    #[test]
    fn the_system_clock_reports_a_plausible_time() {
        // Later than 2020-01-01 and earlier than 2100-01-01, in milliseconds.
        let now = SystemClock.now_ms();
        assert!(
            now > 1_577_836_800_000 && now < 4_102_444_800_000,
            "now_ms = {now}"
        );
    }
}
