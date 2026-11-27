//! Every setting a library holds, as a key the store can validate and a value it can read back.
//!
//! The database stores settings as text, because a library may arrive from a build with fewer or
//! more of them. Text is a storage detail: the rest of the app names a [`SettingKey`] and a typed
//! [`SettingValue`], so a misspelt key or a value of the wrong shape cannot be stored, and the
//! ranges below are checked on the way in and on the way out.

use crate::error::StoreError;
use crate::fsrs;
use crate::model::{
    Group, MAX_RETENTION, MIN_RETENTION, SchedulerKind, SettingKey, SettingValue, Settings, Theme,
};

/// The narrowest and widest library-wide new-card cap. Zero pauses new cards; a thousand is far
/// beyond any study day a person finishes.
pub const MIN_NEW_PER_DAY: u32 = 0;
pub const MAX_NEW_PER_DAY: u32 = 1_000;

/// The shortest and longest wait a learning or relearning step can ask for.
pub const MIN_LEARNING_STEP_MINUTES: u32 = 1;
pub const MAX_LEARNING_STEP_MINUTES: u32 = 1_440;

/// The shortest and longest cap on a scheduled interval.
pub const MIN_MAX_INTERVAL_DAYS: u32 = 1;
pub const MAX_MAX_INTERVAL_DAYS: u32 = 3_650;

/// The body text size the editor and review use, in points.
pub const MIN_PREVIEW_FONT_SIZE: u32 = 10;
pub const MAX_PREVIEW_FONT_SIZE: u32 = 28;

impl SettingKey {
    pub const ALL: [SettingKey; 9] = [
        SettingKey::NewPerDay,
        SettingKey::LearningStepMinutes,
        SettingKey::MaxIntervalDays,
        SettingKey::Scheduler,
        SettingKey::Retention,
        SettingKey::Weights,
        SettingKey::ReverseDefault,
        SettingKey::Theme,
        SettingKey::PreviewFontSize,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            SettingKey::NewPerDay => "new_per_day",
            SettingKey::LearningStepMinutes => "learning_step_minutes",
            SettingKey::MaxIntervalDays => "max_interval_days",
            SettingKey::Scheduler => "scheduler",
            SettingKey::Retention => "retention",
            SettingKey::Weights => "fsrs_weights",
            SettingKey::ReverseDefault => "reverse_default",
            SettingKey::Theme => "theme",
            SettingKey::PreviewFontSize => "preview_font_size",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "new_per_day" => Some(SettingKey::NewPerDay),
            "learning_step_minutes" => Some(SettingKey::LearningStepMinutes),
            "max_interval_days" => Some(SettingKey::MaxIntervalDays),
            "scheduler" => Some(SettingKey::Scheduler),
            "retention" => Some(SettingKey::Retention),
            "fsrs_weights" => Some(SettingKey::Weights),
            "reverse_default" => Some(SettingKey::ReverseDefault),
            "theme" => Some(SettingKey::Theme),
            "preview_font_size" => Some(SettingKey::PreviewFontSize),
            _ => None,
        }
    }

    pub fn group(self) -> Group {
        match self {
            SettingKey::NewPerDay
            | SettingKey::LearningStepMinutes
            | SettingKey::MaxIntervalDays
            | SettingKey::Scheduler
            | SettingKey::Retention
            | SettingKey::Weights
            | SettingKey::ReverseDefault => Group::Review,
            SettingKey::Theme | SettingKey::PreviewFontSize => Group::Appearance,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SettingKey::NewPerDay => "New cards per day",
            SettingKey::LearningStepMinutes => "Learning step",
            SettingKey::MaxIntervalDays => "Longest interval",
            SettingKey::Scheduler => "Scheduler",
            SettingKey::Retention => "Target retention",
            SettingKey::Weights => "FSRS weights",
            SettingKey::ReverseDefault => "Review answers too",
            SettingKey::Theme => "Theme",
            SettingKey::PreviewFontSize => "Preview text size",
        }
    }

    /// One sentence saying what the setting does and what its range means.
    pub fn help(self) -> &'static str {
        match self {
            SettingKey::NewPerDay => {
                "How many cards enter the new queue in one study day; 0 pauses new cards."
            }
            SettingKey::LearningStepMinutes => {
                "How long a new card, or a missed one, waits before it is offered again."
            }
            SettingKey::MaxIntervalDays => "The furthest ahead a remembered card can be scheduled.",
            SettingKey::Scheduler => {
                "fixed uses Mochi's entry steps; fsrs uses stability, difficulty and retention."
            }
            SettingKey::Retention => {
                "The chance of recall FSRS aims at. The fixed scheduler ignores it."
            }
            SettingKey::Weights => {
                "Nineteen positive numbers, comma separated. An optimiser's output replaces the defaults whole."
            }
            SettingKey::ReverseDefault => {
                "Whether the library asks a card from its answer side too, unless a deck or card overrides it."
            }
            SettingKey::Theme => {
                "Light, dark, or whichever the desktop is set to. Both are drawn from the same palette."
            }
            SettingKey::PreviewFontSize => "The size of body text in the editor and the review.",
        }
    }

    /// A value no library has ever written for this key.
    pub fn default_value(self) -> SettingValue {
        match self {
            SettingKey::NewPerDay => SettingValue::Number(Settings::DEFAULT_NEW_PER_DAY),
            SettingKey::LearningStepMinutes => {
                SettingValue::Number(Settings::DEFAULT_LEARNING_STEP_MINUTES)
            }
            SettingKey::MaxIntervalDays => {
                SettingValue::Number(Settings::DEFAULT_MAX_INTERVAL_DAYS)
            }
            SettingKey::Scheduler => SettingValue::Scheduler(SchedulerKind::default()),
            SettingKey::Retention => SettingValue::Number(Settings::DEFAULT_RETENTION),
            SettingKey::Weights => SettingValue::Weights(fsrs::DEFAULT_WEIGHTS.to_vec()),
            SettingKey::ReverseDefault => SettingValue::Flag(false),
            SettingKey::Theme => SettingValue::Theme(Theme::default()),
            SettingKey::PreviewFontSize => {
                SettingValue::Number(Settings::DEFAULT_PREVIEW_FONT_SIZE)
            }
        }
    }

    /// The value this key holds in `settings`.
    pub fn value(self, settings: &Settings) -> SettingValue {
        match self {
            SettingKey::NewPerDay => SettingValue::Number(settings.new_per_day),
            SettingKey::LearningStepMinutes => SettingValue::Number(settings.learning_step_minutes),
            SettingKey::MaxIntervalDays => SettingValue::Number(settings.max_interval_days),
            SettingKey::Scheduler => SettingValue::Scheduler(settings.scheduler),
            SettingKey::Retention => SettingValue::Number(settings.retention),
            SettingKey::Weights => SettingValue::Weights(settings.weights.clone()),
            SettingKey::ReverseDefault => SettingValue::Flag(settings.reverse_default),
            SettingKey::Theme => SettingValue::Theme(settings.theme),
            SettingKey::PreviewFontSize => SettingValue::Number(settings.preview_font_size),
        }
    }

    /// Read stored text as this key's value, refusing anything [`validate`](Self::validate) would.
    pub fn parse_value(self, text: &str) -> Result<SettingValue, StoreError> {
        let refused = || StoreError::Setting {
            key: self.as_str().to_string(),
            value: text.to_string(),
        };
        let value = match self {
            #[expect(clippy::map_err_ignore, reason = "the value is in Setting")]
            SettingKey::NewPerDay
            | SettingKey::LearningStepMinutes
            | SettingKey::MaxIntervalDays
            | SettingKey::Retention
            | SettingKey::PreviewFontSize => {
                SettingValue::Number(text.parse::<u32>().map_err(|_| refused())?)
            }
            SettingKey::Scheduler => {
                SettingValue::Scheduler(SchedulerKind::parse(text).ok_or_else(refused)?)
            }
            SettingKey::Weights => SettingValue::Weights(fsrs::weights(text)?),
            SettingKey::ReverseDefault => match text {
                "0" => SettingValue::Flag(false),
                "1" => SettingValue::Flag(true),
                _ => return Err(refused()),
            },
            SettingKey::Theme => SettingValue::Theme(Theme::parse(text).ok_or_else(refused)?),
        };
        self.validate(&value)?;
        Ok(value)
    }

    /// Whether this key accepts this value.
    pub fn validate(self, value: &SettingValue) -> Result<(), StoreError> {
        let accepted = match (self, value) {
            (SettingKey::NewPerDay, SettingValue::Number(n)) => {
                in_range(*n, MIN_NEW_PER_DAY, MAX_NEW_PER_DAY)
            }
            (SettingKey::LearningStepMinutes, SettingValue::Number(n)) => {
                in_range(*n, MIN_LEARNING_STEP_MINUTES, MAX_LEARNING_STEP_MINUTES)
            }
            (SettingKey::MaxIntervalDays, SettingValue::Number(n)) => {
                in_range(*n, MIN_MAX_INTERVAL_DAYS, MAX_MAX_INTERVAL_DAYS)
            }
            (SettingKey::Scheduler, SettingValue::Scheduler(_)) => true,
            (SettingKey::Retention, SettingValue::Number(n)) => {
                in_range(*n, MIN_RETENTION, MAX_RETENTION)
            }
            (SettingKey::Weights, SettingValue::Weights(weights)) => fsrs::are_weights(weights),
            (SettingKey::ReverseDefault, SettingValue::Flag(_)) => true,
            (SettingKey::Theme, SettingValue::Theme(_)) => true,
            (SettingKey::PreviewFontSize, SettingValue::Number(n)) => {
                in_range(*n, MIN_PREVIEW_FONT_SIZE, MAX_PREVIEW_FONT_SIZE)
            }
            _ => false,
        };
        if accepted {
            Ok(())
        } else {
            Err(StoreError::Setting {
                key: self.as_str().to_string(),
                value: value.text(),
            })
        }
    }

    /// Store this key's value into `settings`. The caller has already validated the pair.
    pub(crate) fn assign(self, settings: &mut Settings, value: SettingValue) {
        match (self, value) {
            (SettingKey::NewPerDay, SettingValue::Number(n)) => settings.new_per_day = n,
            (SettingKey::LearningStepMinutes, SettingValue::Number(n)) => {
                settings.learning_step_minutes = n;
            }
            (SettingKey::MaxIntervalDays, SettingValue::Number(n)) => {
                settings.max_interval_days = n
            }
            (SettingKey::Scheduler, SettingValue::Scheduler(kind)) => settings.scheduler = kind,
            (SettingKey::Retention, SettingValue::Number(n)) => settings.retention = n,
            (SettingKey::Weights, SettingValue::Weights(weights)) => settings.weights = weights,
            (SettingKey::ReverseDefault, SettingValue::Flag(on)) => settings.reverse_default = on,
            (SettingKey::Theme, SettingValue::Theme(theme)) => settings.theme = theme,
            (SettingKey::PreviewFontSize, SettingValue::Number(n)) => {
                settings.preview_font_size = n
            }
            _ => {}
        }
    }
}

impl SettingValue {
    /// The text the settings row holds. It is what [`SettingKey::parse_value`] accepts.
    pub fn text(&self) -> String {
        match self {
            SettingValue::Number(n) => n.to_string(),
            SettingValue::Flag(on) => if *on { "1" } else { "0" }.to_string(),
            SettingValue::Scheduler(kind) => kind.as_str().to_string(),
            SettingValue::Weights(weights) => fsrs::weight_text(weights),
            SettingValue::Theme(theme) => theme.as_str().to_string(),
        }
    }

    /// The value shown on the settings screen.
    pub fn display(&self) -> String {
        match self {
            SettingValue::Flag(true) => "On".to_string(),
            SettingValue::Flag(false) => "Off".to_string(),
            SettingValue::Theme(theme) => theme.label().to_string(),
            SettingValue::Number(n) => n.to_string(),
            SettingValue::Scheduler(kind) => kind.as_str().to_string(),
            SettingValue::Weights(weights) => format!("{} numbers", weights.len()),
        }
    }
}

impl Settings {
    /// The value of `key` in this library.
    pub fn value(&self, key: SettingKey) -> SettingValue {
        key.value(self)
    }
}

/// Checks a new-card cap typed by the user against the limit the store keeps. The error names the
/// limit so the user knows what is allowed.
#[expect(clippy::map_err_ignore, reason = "this is UI copy for the cap field")]
pub fn check_new_card_cap(text: &str) -> Result<u32, String> {
    let value: u32 = text
        .trim()
        .parse()
        .map_err(|_| format!("New cards a day must be a whole number up to {MAX_NEW_PER_DAY}"))?;
    if value > MAX_NEW_PER_DAY {
        return Err(format!("New cards a day must be {MAX_NEW_PER_DAY} or less"));
    }
    Ok(value)
}

fn in_range(value: u32, min: u32, max: u32) -> bool {
    (min..=max).contains(&value)
}

#[cfg(test)]
mod cap_tests {
    use super::*;

    #[test]
    fn accepts_whole_numbers_within_limit_and_refuses_others() {
        assert_eq!(check_new_card_cap("0"), Ok(0));
        assert_eq!(check_new_card_cap(" 25 "), Ok(25));
        assert_eq!(check_new_card_cap("1000"), Ok(1000));
        assert!(check_new_card_cap("1001").unwrap_err().contains("1000"));
        assert!(check_new_card_cap("-1").is_err());
        assert!(check_new_card_cap("abc").is_err());
    }
}
