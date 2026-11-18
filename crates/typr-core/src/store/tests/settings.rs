//! Settings: their defaults, their ranges, and what a library does with a value nobody wrote.

use super::*;
use crate::store::settings::{
    MAX_LEARNING_STEP_MINUTES, MAX_MAX_INTERVAL_DAYS, MAX_NEW_PER_DAY, MAX_PREVIEW_FONT_SIZE,
    MIN_LEARNING_STEP_MINUTES, MIN_MAX_INTERVAL_DAYS, MIN_NEW_PER_DAY, MIN_PREVIEW_FONT_SIZE,
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
fn every_setting_round_trips() {
    let dir = TempDir::new().expect("temp dir");
    let db = open_temp(&dir);
    let mut weights = crate::fsrs::DEFAULT_WEIGHTS.to_vec();
    weights[0] = 0.5;
    let values = [
        (SettingKey::NewPerDay, SettingValue::Number(37)),
        (SettingKey::LearningStepMinutes, SettingValue::Number(25)),
        (SettingKey::MaxIntervalDays, SettingValue::Number(120)),
        (
            SettingKey::Scheduler,
            SettingValue::Scheduler(SchedulerKind::Fsrs),
        ),
        (SettingKey::Retention, SettingValue::Number(85)),
        (SettingKey::Weights, SettingValue::Weights(weights)),
        (SettingKey::ReverseDefault, SettingValue::Flag(true)),
        (
            SettingKey::Theme,
            SettingValue::Theme(crate::store::Theme::Light),
        ),
        (SettingKey::PreviewFontSize, SettingValue::Number(18)),
    ];
    // Every theme, not one: `System` is a stored value like the other two.
    let themes = crate::store::Theme::ALL.map(|theme| (SettingKey::Theme, SettingValue::Theme(theme)));
    for (key, value) in values.into_iter().chain(themes) {
        db.apply(
            &Command::SetSetting {
                key,
                value: value.clone(),
            },
            0,
        )
        .unwrap_or_else(|error| panic!("writing {}: {error}", key.as_str()));
        assert_eq!(
            db.settings().expect("settings").value(key),
            value,
            "{} did not read back what was written",
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
fn a_library_with_a_stale_scale_row_opens() {
    // The interface scale was a setting before it was removed; a library written then still holds it.
    let dir = TempDir::new().expect("temp dir");
    let db = open_temp(&dir);
    db.connection_for_test()
        .execute(
            "INSERT INTO settings (key, value) VALUES ('scale', '1.25')",
            [],
        )
        .expect("a row an older build wrote");

    assert_eq!(
        db.settings().expect("opens with no refusal"),
        Settings::default()
    );
    let kept: String = db
        .connection_for_test()
        .query_row("SELECT value FROM settings WHERE key = 'scale'", [], |row| {
            row.get(0)
        })
        .expect("the old row");
    assert_eq!(kept, "1.25", "the row is left where it was");
}

#[test]
fn a_stored_system_theme_reads_as_system() {
    let dir = TempDir::new().expect("temp dir");
    let db = open_temp(&dir);
    db.connection_for_test()
        .execute(
            "INSERT INTO settings (key, value) VALUES ('theme', 'system')",
            [],
        )
        .expect("a theme row");
    assert_eq!(db.settings().expect("settings").theme, Theme::System);
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
