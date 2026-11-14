use super::{
    DAY_MS, FixedMultiplier, Fsrs, LEARNING_STEP_MS, MAX_INTERVAL_MS, MULTIPLIER_DEN,
    MULTIPLIER_NUM, Outcome, Policy, PromptState, Scheduler,
};
use crate::fsrs::{self, Memory};
use crate::store::{Grade, Phase};

const AT: i64 = 1_700_000_000_000;

fn grade(state: PromptState, grade: Grade) -> Outcome {
    FixedMultiplier.grade(&state, grade, AT, Policy::default())
}

#[test]
fn a_new_card_remembered_leaves_the_new_queue_for_ten_minutes() {
    assert_eq!(
        grade(PromptState::NEW, Grade::Remembered),
        Outcome {
            phase: Phase::Review,
            interval_ms: 0,
            due_at: Some(AT + LEARNING_STEP_MS),
            memory: None
        }
    );
}

#[test]
fn a_new_card_forgotten_stays_new_and_unscheduled() {
    assert_eq!(
        grade(PromptState::NEW, Grade::Forgot),
        Outcome {
            phase: Phase::Learn,
            interval_ms: 0,
            due_at: None,
            memory: None
        }
    );
}

#[test]
fn the_step_after_the_first_success_sets_one_day() {
    let stepped = PromptState {
        phase: Phase::Review,
        interval_ms: 0,
        ..PromptState::NEW
    };
    assert_eq!(
        grade(stepped, Grade::Remembered),
        Outcome {
            phase: Phase::Review,
            interval_ms: DAY_MS,
            due_at: Some(AT + DAY_MS),
            memory: None
        }
    );
}

#[test]
fn repeated_successes_stretch_by_two_and_a_half() {
    let ten_days = PromptState {
        phase: Phase::Review,
        interval_ms: 10 * DAY_MS,
        ..PromptState::NEW
    };
    let outcome = grade(ten_days, Grade::Remembered);
    assert_eq!(outcome.interval_ms, 25 * DAY_MS);
    assert_eq!(outcome.due_at, Some(AT + 25 * DAY_MS));
}

#[test]
fn the_interval_is_capped_at_a_year() {
    let long = PromptState {
        phase: Phase::Review,
        interval_ms: 200 * DAY_MS,
        ..PromptState::NEW
    };
    let uncapped = long.interval_ms * MULTIPLIER_NUM / MULTIPLIER_DEN;
    assert!(
        uncapped > MAX_INTERVAL_MS,
        "the fixture has to exceed the cap"
    );
    assert_eq!(grade(long, Grade::Remembered).interval_ms, MAX_INTERVAL_MS);
}

#[test]
fn a_failure_in_review_starts_a_ten_minute_relearn_step() {
    let scheduled = PromptState {
        phase: Phase::Review,
        interval_ms: 10 * DAY_MS,
        ..PromptState::NEW
    };
    assert_eq!(
        grade(scheduled, Grade::Forgot),
        Outcome {
            phase: Phase::Relearn,
            interval_ms: 10 * DAY_MS,
            due_at: Some(AT + LEARNING_STEP_MS),
            memory: None
        }
    );
}

#[test]
fn success_after_a_failure_starts_short_again() {
    let relearning = PromptState {
        phase: Phase::Relearn,
        interval_ms: 10 * DAY_MS,
        ..PromptState::NEW
    };
    assert_eq!(
        grade(relearning, Grade::Remembered),
        Outcome {
            phase: Phase::Review,
            interval_ms: DAY_MS,
            due_at: Some(AT + DAY_MS),
            memory: None
        }
    );
}

#[test]
fn a_failure_inside_a_relearn_step_stays_in_it() {
    let relearning = PromptState {
        phase: Phase::Relearn,
        interval_ms: DAY_MS,
        ..PromptState::NEW
    };
    let outcome = grade(relearning, Grade::Forgot);
    assert_eq!(outcome.phase, Phase::Relearn);
    assert_eq!(outcome.due_at, Some(AT + LEARNING_STEP_MS));
}

#[test]
fn an_fsrs_grade_bootstraps_a_prompt_it_has_never_seen() {
    let fsrs = Fsrs::defaults();
    // A prompt whose library predates FSRS: it reached five days under the fixed scheduler and
    // carries no memory, so the first FSRS grade has to start from the interval it already has.
    let scheduled = PromptState {
        phase: Phase::Review,
        interval_ms: 5 * DAY_MS,
        due_at: Some(AT),
        ..PromptState::NEW
    };
    let outcome = fsrs.grade(&scheduled, Grade::Remembered, AT, Policy::default());
    assert_eq!(outcome.phase, Phase::Review);
    // Answered on its due date, so it has been held exactly five days: five-day stability, 90%
    // recall, remembered. Starting from `initial` instead would give a different number, which is
    // what makes this an assertion about bootstrapping rather than about the formula.
    assert_eq!(
        outcome.interval_ms, 1_419_282_260,
        "five days of stability, thirty-eight percent of it recovered, is sixteen days"
    );
    let memory = outcome.memory.expect("FSRS leaves a memory behind");
    assert_eq!(memory.stability, 16.42687800983301, "grown from five days");
    assert!(
        (5.4..5.6).contains(&memory.difficulty),
        "and middling difficulty, pulled a hair towards the easy anchor: {}",
        memory.difficulty
    );
}

#[test]
fn an_fsrs_first_grade_of_a_new_card_starts_from_its_stability() {
    let fsrs = Fsrs::defaults();
    let outcome = fsrs.grade(&PromptState::NEW, Grade::Remembered, AT, Policy::default());
    let memory = outcome.memory.expect("a memory");
    assert_eq!(
        memory,
        fsrs::initial(Grade::Remembered, &fsrs.weights),
        "the first grade sets the memory outright"
    );
    assert_eq!(
        outcome.interval_ms, 274_147_200,
        "which is w2 = 3.173 days, not the fixed path's ten minutes"
    );
    assert_eq!(outcome.due_at, Some(AT + outcome.interval_ms));
}

#[test]
fn a_failed_fsrs_review_waits_less_than_a_remembered_one() {
    let fsrs = Fsrs::defaults();
    let due = PromptState {
        phase: Phase::Review,
        interval_ms: 10 * DAY_MS,
        due_at: Some(AT),
        last_at: Some(AT - 10 * DAY_MS),
        memory: Some(Memory {
            stability: 10.0,
            difficulty: 5.0,
        }),
    };
    let miss = fsrs.grade(&due, Grade::Forgot, AT, Policy::default());
    let hit = fsrs.grade(&due, Grade::Remembered, AT, Policy::default());
    assert_eq!(miss.phase, Phase::Relearn);
    assert_eq!(hit.phase, Phase::Review);
    assert!(
        miss.interval_ms < hit.interval_ms,
        "a lapse waits less than a success: {} vs {}",
        miss.interval_ms,
        hit.interval_ms
    );
    assert!(
        miss.memory.expect("a memory").stability < 10.0,
        "and its stability fell"
    );
}

#[test]
fn the_fixed_scheduler_still_produces_the_same_numbers() {
    let ten_days = PromptState {
        phase: Phase::Review,
        interval_ms: 10 * DAY_MS,
        ..PromptState::NEW
    };
    let outcome = FixedMultiplier.grade(&ten_days, Grade::Remembered, AT, Policy::default());
    assert_eq!(
        outcome.interval_ms,
        25 * DAY_MS,
        "the multiplier is unchanged"
    );
    assert_eq!(outcome.due_at, Some(AT + 25 * DAY_MS));
    assert_eq!(
        outcome.memory, None,
        "the fixed path keeps no memory, whatever the library is set to"
    );
}

#[test]
fn a_review_on_its_due_date_still_raises_the_stability() {
    let fsrs = Fsrs::defaults();
    let mut state = PromptState {
        phase: Phase::Review,
        interval_ms: DAY_MS,
        due_at: Some(AT),
        last_at: Some(AT - DAY_MS),
        memory: Some(Memory {
            stability: 1.0,
            difficulty: 5.0,
        }),
    };

    // Twice in a row, each answer landing exactly on the due date.
    for _ in 0..2 {
        let outcome = fsrs.grade(
            &state,
            Grade::Remembered,
            state.due_at.expect("due"),
            Policy::default(),
        );
        let memory = outcome.memory.expect("FSRS keeps a memory");
        assert!(
            memory.stability > state.memory.expect("a memory").stability,
            "an on-time review is still an interval of forgetting"
        );
        assert!(
            outcome.interval_ms > state.interval_ms,
            "so the next wait is longer than the last"
        );
        state = PromptState {
            phase: outcome.phase,
            interval_ms: outcome.interval_ms,
            due_at: outcome.due_at,
            last_at: state.due_at,
            memory: Some(memory),
        };
    }
}
