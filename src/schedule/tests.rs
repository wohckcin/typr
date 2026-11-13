use super::{DAY_MS, FixedMultiplier, LEARNING_STEP_MS, MAX_INTERVAL_MS, Outcome, PromptState, Scheduler};
use crate::store::{Grade, Phase};

const AT: i64 = 1_700_000_000_000;

fn grade(state: PromptState, grade: Grade) -> Outcome {
    FixedMultiplier.grade(&state, grade, AT)
}

#[test]
fn a_new_card_remembered_leaves_the_new_queue_for_ten_minutes() {
    assert_eq!(
        grade(PromptState::NEW, Grade::Remembered),
        Outcome { phase: Phase::Review, interval_ms: 0, due_at: Some(AT + LEARNING_STEP_MS) }
    );
}

#[test]
fn a_new_card_forgotten_stays_new_and_unscheduled() {
    assert_eq!(
        grade(PromptState::NEW, Grade::Forgot),
        Outcome { phase: Phase::Learn, interval_ms: 0, due_at: None }
    );
}

#[test]
fn the_step_after_the_first_success_sets_one_day() {
    let stepped = PromptState { phase: Phase::Review, interval_ms: 0 };
    assert_eq!(
        grade(stepped, Grade::Remembered),
        Outcome { phase: Phase::Review, interval_ms: DAY_MS, due_at: Some(AT + DAY_MS) }
    );
}

#[test]
fn repeated_successes_stretch_by_two_and_a_half() {
    let ten_days = PromptState { phase: Phase::Review, interval_ms: 10 * DAY_MS };
    let outcome = grade(ten_days, Grade::Remembered);
    assert_eq!(outcome.interval_ms, 25 * DAY_MS);
    assert_eq!(outcome.due_at, Some(AT + 25 * DAY_MS));
}

#[test]
fn the_interval_is_capped_at_a_year() {
    let long = PromptState { phase: Phase::Review, interval_ms: 200 * DAY_MS };
    assert_eq!(grade(long, Grade::Remembered).interval_ms, MAX_INTERVAL_MS);
    assert!(MAX_INTERVAL_MS < 200 * DAY_MS * 5 / 2);
}

#[test]
fn a_failure_in_review_starts_a_ten_minute_relearn_step() {
    let scheduled = PromptState { phase: Phase::Review, interval_ms: 10 * DAY_MS };
    assert_eq!(
        grade(scheduled, Grade::Forgot),
        Outcome { phase: Phase::Relearn, interval_ms: 10 * DAY_MS, due_at: Some(AT + LEARNING_STEP_MS) }
    );
}

#[test]
fn success_after_a_failure_starts_short_again() {
    let relearning = PromptState { phase: Phase::Relearn, interval_ms: 10 * DAY_MS };
    assert_eq!(
        grade(relearning, Grade::Remembered),
        Outcome { phase: Phase::Review, interval_ms: DAY_MS, due_at: Some(AT + DAY_MS) }
    );
}

#[test]
fn a_failure_inside_a_relearn_step_stays_in_it() {
    let relearning = PromptState { phase: Phase::Relearn, interval_ms: DAY_MS };
    let outcome = grade(relearning, Grade::Forgot);
    assert_eq!(outcome.phase, Phase::Relearn);
    assert_eq!(outcome.due_at, Some(AT + LEARNING_STEP_MS));
}
