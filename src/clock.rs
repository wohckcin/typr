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
            .map(|d| d.as_millis() as i64)
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
mod tests;
