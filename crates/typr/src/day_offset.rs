//! The machine's offset east of UTC.
//!
//! `time` can only answer this on the main thread of a single-threaded program, so the app reads it
//! once at startup and every later reader reuses the answer. A program that cannot know its zone
//! (a test, a locked-down sandbox) studies on UTC days, which is wrong by hours at worst and never
//! inconsistent with itself.

use std::sync::LazyLock;

/// The machine's offset east of UTC, in milliseconds.
pub fn local_offset_ms() -> i64 {
    *LOCAL_OFFSET_MS
}

/// Resolved on first use, which the app arranges to happen on the main thread before it starts the
/// store task.
static LOCAL_OFFSET_MS: LazyLock<i64> =
    LazyLock::new(|| match time::UtcOffset::current_local_offset() {
        Ok(offset) => i64::from(offset.whole_seconds()) * 1000,
        Err(_) => 0,
    });
