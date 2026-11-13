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
