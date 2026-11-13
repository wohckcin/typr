use super::{ROLLOVER_MS, study_day, study_day_bounds};
use crate::schedule::DAY_MS;

/// 2026-01-11T00:00:00Z as milliseconds, in a fixed UTC offset so the test never depends on the
/// machine's zone.
const MIDNIGHT: i64 = 1_768_089_600_000;

#[test]
fn a_moment_just_after_midnight_belongs_to_the_previous_study_day() {
    let just_after_midnight = MIDNIGHT + 30 * 60 * 1000;
    let late_evening = MIDNIGHT - 10 * 60 * 1000;
    assert_eq!(study_day(just_after_midnight, 0), study_day(late_evening, 0));
}

#[test]
fn the_study_day_turns_over_at_four() {
    let before_four = MIDNIGHT + ROLLOVER_MS - 60_000;
    let at_four = MIDNIGHT + ROLLOVER_MS;
    assert_eq!(study_day(at_four, 0), study_day(before_four, 0) + 1);
}

#[test]
fn bounds_wrap_the_moments_of_one_day() {
    let moment = MIDNIGHT + 13 * 60 * 60 * 1000;
    let day = study_day(moment, 0);
    let (from, to) = study_day_bounds(day, 0);
    assert!(from <= moment && moment < to, "{from} <= {moment} < {to}");
    assert_eq!(to - from, DAY_MS);
    assert_eq!(study_day(from, 0), day);
    assert_eq!(study_day(to - 1, 0), day);
    assert_eq!(study_day(to, 0), day + 1);
}

#[test]
fn an_offset_moves_the_boundary_with_the_clock() {
    // 02:00 UTC is 04:00 at UTC+2: the same instant lands on the next study day.
    let two_am = MIDNIGHT + 2 * 60 * 60 * 1000;
    assert_eq!(study_day(two_am, 0), study_day(two_am, 2 * 60 * 60 * 1000) - 1);
}
