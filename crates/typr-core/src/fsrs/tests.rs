//! The numbers here come from the published FSRS-5 formulas, evaluated by hand for the default
//! weights: they are the check that the exponent, the factor and the weight index are the ones the
//! algorithm names, not merely ones that produce plausible output.

use super::{
    DEFAULT_RETENTION, DEFAULT_WEIGHTS, Memory, WEIGHT_COUNT, initial, interval_days, update,
    weight_text, weights,
};
use crate::store::{Grade, StoreError};

/// Both values of a grade, spelled out so a test reads as the case it is about.
const HIT: Grade = Grade::Remembered;
const MISS: Grade = Grade::Forgot;

/// A comparison that names both sides when it fails, to the precision the formulas are pinned at.
#[track_caller]
fn near(what: &str, got: f64, want: f64) {
    let tolerance = 1e-9_f64.max(want.abs() * 1e-12);
    assert!(
        (got - want).abs() <= tolerance,
        "{what}: got {got}, want {want}"
    );
}

#[test]
fn a_first_hit_starts_stable_and_a_first_miss_starts_unstable() {
    let hit = initial(HIT, &DEFAULT_WEIGHTS);
    near("a first hit's stability is w2", hit.stability, 3.173);
    near(
        "a first hit's difficulty is D0(3)",
        hit.difficulty,
        5.282_434_422_319_005,
    );
    let miss = initial(MISS, &DEFAULT_WEIGHTS);
    near("a first miss's stability is w0", miss.stability, 0.40255);
    near("a first miss's difficulty is w4", miss.difficulty, 7.1949);
}

#[test]
fn a_hit_on_the_due_date_grows_stability_and_lowers_difficulty() {
    let start = Memory {
        stability: 3.173,
        difficulty: 5.282_434_422_319_005,
    };
    let after = update(start, HIT, start.stability, &DEFAULT_WEIGHTS);
    near(
        "the stability grows",
        after.stability,
        11.131_259_853_438_268,
    );
    near(
        "the difficulty falls towards the easy end",
        after.difficulty,
        5.272_967_931_287_446,
    );
}

#[test]
fn a_miss_drops_stability_and_raises_difficulty() {
    let start = Memory {
        stability: 3.173,
        difficulty: 5.282_434_422_319_005,
    };
    let after = update(start, MISS, start.stability, &DEFAULT_WEIGHTS);
    near(
        "the stability collapses",
        after.stability,
        1.066_877_827_375_298_8,
    );
    near(
        "the difficulty rises",
        after.difficulty,
        6.796_932_579_932_991,
    );
}

#[test]
fn a_longer_wait_pushes_stability_further_but_not_without_limit() {
    let start = Memory {
        stability: 3.173,
        difficulty: 5.282_434_422_319_005,
    };
    let one = update(start, HIT, 1.0, &DEFAULT_WEIGHTS).stability;
    let ten = update(start, HIT, 10.0, &DEFAULT_WEIGHTS).stability;
    let thirty = update(start, HIT, 30.0, &DEFAULT_WEIGHTS).stability;
    near("one day late", one, 5.869_141_822_641_378);
    near("ten days late", ten, 23.895_875_500_557_327);
    near("thirty days late", thirty, 45.445_656_390_273_39);
    assert!(
        one < ten && ten < thirty,
        "the gain is monotone in the wait"
    );
    assert!(
        thirty < start.stability * 1_000.0,
        "an overdue card cannot jump to centuries"
    );
}

#[test]
fn the_interval_is_the_stability_at_the_default_retention() {
    near(
        "a ten-day stability at 90%",
        interval_days(10.0, DEFAULT_RETENTION),
        10.0,
    );
    near(
        "and the same for any stability",
        interval_days(3.173, DEFAULT_RETENTION),
        3.173,
    );
}

#[test]
fn a_higher_retention_asks_for_a_shorter_interval() {
    // 95% and 70% are pinned because the closed form is where a wrong `FACTOR` shows up: a
    // parameterisation that is right at 90% (where any factor cancels) lands nowhere near here.
    near(
        "95% retention",
        interval_days(10.0, 0.95),
        4.605_627_642_513_489,
    );
    near(
        "70% retention",
        interval_days(10.0, 0.7),
        44.371_643_394_199_786,
    );
    assert!(
        interval_days(10.0, 0.95) < interval_days(10.0, 0.9),
        "asking to remember more shortens the wait"
    );
    assert!(interval_days(10.0, 0.9) < interval_days(10.0, 0.7));
}

#[test]
fn stability_never_falls_to_nothing() {
    let tiny = Memory {
        stability: 0.1,
        difficulty: 10.0,
    };
    let after = update(tiny, MISS, 0.0, &DEFAULT_WEIGHTS);
    assert!(after.stability >= 0.1, "got {}", after.stability);
    assert!(
        interval_days(0.0, DEFAULT_RETENTION) >= 0.1,
        "a zero stability still asks for a real interval"
    );
}

#[test]
fn difficulty_stays_between_one_and_ten() {
    let mut memory = initial(HIT, &DEFAULT_WEIGHTS);
    for _ in 0..50 {
        memory = update(memory, MISS, 30.0, &DEFAULT_WEIGHTS);
        assert!(memory.difficulty <= 10.0, "got {}", memory.difficulty);
    }
    for _ in 0..50 {
        memory = update(memory, HIT, 1.0, &DEFAULT_WEIGHTS);
        assert!(memory.difficulty >= 1.0, "got {}", memory.difficulty);
    }
}

#[test]
fn a_prompt_with_no_memory_borrows_the_interval_it_already_reached() {
    let day = crate::day::DAY_MS;
    let bootstrapped = Memory::bootstrapped(5 * day);
    near("five days", bootstrapped.stability, 5.0);
    assert_eq!(bootstrapped.difficulty, 5.5, "middling difficulty");
    assert_eq!(
        Memory::bootstrapped(0).stability,
        0.1,
        "a new prompt is never zero"
    );
}

#[test]
fn a_weight_list_reads_back_as_it_was_written() {
    let text = weight_text(&DEFAULT_WEIGHTS);
    let parsed = weights(&text).expect("the defaults are a valid list");
    assert_eq!(parsed, DEFAULT_WEIGHTS.to_vec());
    assert_eq!(parsed.len(), WEIGHT_COUNT);
}

#[test]
fn a_malformed_weight_list_is_refused() {
    let short = weight_text(&DEFAULT_WEIGHTS[..WEIGHT_COUNT - 1]);
    let long = format!("{},0.5", weight_text(&DEFAULT_WEIGHTS));
    for (what, text) in [
        ("eighteen numbers", short.as_str()),
        ("twenty numbers", long.as_str()),
        ("a word", "0.40255,one,3.173"),
        ("an empty string", ""),
        (
            "a zero",
            "0.40255,0,3.173,15.69105,7.1949,0.5345,1.4604,0.0046,1.54575,0.1192,1.01925,1.9395,0.11,0.29605,2.2698,0.2315,2.9898,0.51655,0.6621",
        ),
    ] {
        match weights(text) {
            Err(StoreError::Setting { key, .. }) => assert_eq!(key, "fsrs_weights", "{what}"),
            other => panic!("{what} should be refused, got {other:?}"),
        }
    }
}

#[test]
fn a_lapse_never_raises_stability() {
    let memory = Memory {
        stability: 1.0,
        difficulty: 5.0,
    };
    let lapsed = update(memory, MISS, 30.0, &DEFAULT_WEIGHTS);
    assert!(
        lapsed.stability <= memory.stability,
        "a miss after a long gap raised stability from {} to {}",
        memory.stability,
        lapsed.stability
    );
}
