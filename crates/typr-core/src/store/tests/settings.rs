//! Settings: their defaults, their ranges, and what a library does with a value nobody wrote.

use super::*;
use crate::store::settings::{
    MAX_LEARNING_STEP_MINUTES, MAX_MAX_INTERVAL_DAYS, MAX_NEW_PER_DAY, MAX_PREVIEW_FONT_SIZE,
    MAX_SCALE, MIN_LEARNING_STEP_MINUTES, MIN_MAX_INTERVAL_DAYS, MIN_NEW_PER_DAY,
    MIN_PREVIEW_FONT_SIZE, MIN_SCALE,
};

#[test]
fn every_key_defaults_to_the_value_it_documents() {
    let settings = Settings::default();
    for key in SettingKey::ALL {
        assert_eq!(
            settings.value(key),
            key.default_value(),
            "{} is not at its documented default",
            key.as_str()
        );
        let text = key.default_value().text();
        assert_eq!(
            key.parse_value(&text).expect("the default parses"),
            key.default_value(),
            "{} does not read back the default it writes",
            key.as_str()
        );
    }
}

#[test]
fn a_number_at_either_edge_is_accepted_and_one_past_it_is_refused() {
    let ranges = [
        (SettingKey::NewPerDay, MIN_NEW_PER_DAY, MAX_NEW_PER_DAY),
        (
            SettingKey::LearningStepMinutes,
            MIN_LEARNING_STEP_MINUTES,
            MAX_LEARNING_STEP_MINUTES,
        ),
        (
            SettingKey::MaxIntervalDays,
            MIN_MAX_INTERVAL_DAYS,
            MAX_MAX_INTERVAL_DAYS,
        ),
        (
            SettingKey::Retention,
            crate::store::MIN_RETENTION,
            crate::store::MAX_RETENTION,
        ),
        (
            SettingKey::PreviewFontSize,
            MIN_PREVIEW_FONT_SIZE,
            MAX_PREVIEW_FONT_SIZE,
        ),
    ];
    for (key, min, max) in ranges {
        for edge in [min, max] {
            key.validate(&SettingValue::Number(edge))
                .unwrap_or_else(|error| panic!("{} at {edge}: {error}", key.as_str()));
        }
        if let Some(below) = min.checked_sub(1) {
            assert!(
                key.validate(&SettingValue::Number(below)).is_err(),
                "{} accepts {below}, below its minimum",
                key.as_str()
            );
        }
        assert!(
            key.validate(&SettingValue::Number(max + 1)).is_err(),
            "{} accepts {}, above its maximum",
            key.as_str(),
            max + 1
        );
    }
}

#[test]
fn a_scale_at_either_edge_is_accepted_and_one_past_it_is_refused() {
    for edge in [MIN_SCALE, MAX_SCALE] {
        SettingKey::Scale
            .validate(&SettingValue::Scale(edge))
            .unwrap_or_else(|error| panic!("scale at {edge}: {error}"));
    }
    for outside in [MIN_SCALE - 0.01, MAX_SCALE + 0.01, f32::NAN, f32::INFINITY] {
        assert!(
            SettingKey::Scale
                .validate(&SettingValue::Scale(outside))
                .is_err(),
            "scale accepts {outside}"
        );
    }
}

#[test]
fn a_typed_value_is_refused_by_the_key_that_does_not_hold_it() {
    let pairs = [
        (SettingKey::NewPerDay, SettingValue::Flag(true)),
        (SettingKey::ReverseDefault, SettingValue::Number(1)),
        (
            SettingKey::Scheduler,
            SettingValue::Theme(crate::store::Theme::Light),
        ),
        (
            SettingKey::Theme,
            SettingValue::Scheduler(SchedulerKind::Fsrs),
        ),
        (SettingKey::Weights, SettingValue::Number(19)),
        (SettingKey::Scale, SettingValue::Number(1)),
    ];
    for (key, value) in pairs {
        assert!(
            key.validate(&value).is_err(),
            "{} accepted a value of another shape",
            key.as_str()
        );
    }
}

#[test]
fn a_refused_value_leaves_the_stored_one_alone() {
    let dir = TempDir::new().expect("temp dir");
    let db = open_temp(&dir);
    db.apply(
        &Command::SetSetting {
            key: SettingKey::NewPerDay,
            value: SettingValue::Number(5),
        },
        0,
    )
    .expect("a cap in range");

    let refused = db.apply(
        &Command::SetSetting {
            key: SettingKey::NewPerDay,
            value: SettingValue::Number(MAX_NEW_PER_DAY + 1),
        },
        0,
    );
    assert!(refused.is_err(), "a cap above the range is refused");
    assert_eq!(
        db.settings().expect("settings").new_per_day,
        5,
        "the refused value did not replace the one already stored"
    );
}

#[test]
fn a_hand_edited_value_is_reported_rather_than_replaced() {
    let dir = TempDir::new().expect("temp dir");
    let db = open_temp(&dir);
    db.connection_for_test()
        .execute(
            "INSERT INTO settings (key, value) VALUES ('new_per_day', 'plenty')",
            [],
        )
        .expect("a row no build writes");

    match db.settings() {
        Err(StoreError::Setting { key, value }) => {
            assert_eq!(key, "new_per_day");
            assert_eq!(value, "plenty");
        }
        other => panic!("a stored value nothing can read should be reported, got {other:?}"),
    }
}

#[test]
fn an_unknown_stored_key_is_left_alone() {
    let dir = TempDir::new().expect("temp dir");
    let db = open_temp(&dir);
    db.connection_for_test()
        .execute(
            "INSERT INTO settings (key, value) VALUES ('a_later_build_setting', 'future')",
            [],
        )
        .expect("a row a later build wrote");

    assert_eq!(
        db.settings().expect("settings"),
        Settings::default(),
        "a key this build does not know cannot change what it reads"
    );
    let kept: String = db
        .connection_for_test()
        .query_row(
            "SELECT value FROM settings WHERE key = 'a_later_build_setting'",
            [],
            |row| row.get(0),
        )
        .expect("the later build's row");
    assert_eq!(kept, "future", "and it is still there for that build");
}

#[test]
fn the_learning_step_and_the_interval_cap_are_what_the_schedule_reads() {
    let settings = Settings {
        learning_step_minutes: 30,
        max_interval_days: 90,
        ..Settings::default()
    };
    assert_eq!(settings.learning_step_ms(), 30 * 60_000);
    assert_eq!(settings.max_interval_ms(), 90 * crate::day::DAY_MS);
}

#[test]
fn a_text_value_no_build_writes_names_the_key_and_the_text() {
    match SettingKey::Scheduler.parse_value("annealing") {
        Err(StoreError::Setting { key, value }) => {
            assert_eq!(key, "scheduler");
            assert_eq!(value, "annealing");
        }
        other => panic!("an unknown scheduler text should be refused, got {other:?}"),
    }
    assert!(
        SettingKey::parse("new_cards").is_none(),
        "an unknown key has no SettingKey to carry it, so it cannot be stored"
    );
}
