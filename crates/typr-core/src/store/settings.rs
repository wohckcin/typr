//! Every setting a library holds, as a key the store can validate and a value it can read back.
//!
//! The database stores settings as text, because a library may arrive from a build with fewer or
//! more of them. Text is a storage detail: the rest of the app names a [`SettingKey`] and a typed
//! [`SettingValue`], so a misspelt key or a value of the wrong shape cannot be stored, and the
//! ranges below are checked on the way in and on the way out.

use crate::fsrs;
use crate::schedule::{DAY_MS, MINUTE_MS};
use crate::store::{SchedulerKind, Settings, StoreError};

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

/// The interface scales a reader can choose between.
pub const MIN_SCALE: f32 = 0.80;
pub const MAX_SCALE: f32 = 1.50;

/// The body text size the editor and review use, in points.
pub const MIN_PREVIEW_FONT_SIZE: u32 = 10;
pub const MAX_PREVIEW_FONT_SIZE: u32 = 28;

/// Whether the app draws dark or light. Stored in the library so a copied library looks the same.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Theme {
    /// The default: the one every library that has never touched the setting uses.
    #[default]
    Dark,
    Light,
}

impl Theme {
    pub fn as_str(self) -> &'static str {
        match self {
            Theme::Dark => "dark",
            Theme::Light => "light",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "dark" => Some(Theme::Dark),
            "light" => Some(Theme::Light),
            _ => None,
        }
    }

    /// Whether the kit drawn with this theme is the dark one.
    pub fn is_dark(self) -> bool {
        self == Theme::Dark
    }

    pub fn label(self) -> &'static str {
        match self {
            Theme::Dark => "Dark",
            Theme::Light => "Light",
        }
    }
}

/// The three groups the settings screen is read in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    Review,
    Appearance,
    Library,
}

impl Group {
    pub const ALL: [Group; 3] = [Group::Review, Group::Appearance, Group::Library];

    pub fn title(self) -> &'static str {
        match self {
            Group::Review => "Review",
            Group::Appearance => "Appearance",
            Group::Library => "Library",
        }
    }
}

/// One setting, named. The stored spelling is [`SettingKey::as_str`]; the text of a value is
/// [`SettingValue::text`], and the pair is the only thing the store writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingKey {
    NewPerDay,
    LearningStepMinutes,
    MaxIntervalDays,
    Scheduler,
    Retention,
    Weights,
    ReverseDefault,
    Theme,
    Scale,
    PreviewFontSize,
}

impl SettingKey {
    pub const ALL: [SettingKey; 10] = [
        SettingKey::NewPerDay,
        SettingKey::LearningStepMinutes,
        SettingKey::MaxIntervalDays,
        SettingKey::Scheduler,
        SettingKey::Retention,
        SettingKey::Weights,
        SettingKey::ReverseDefault,
        SettingKey::Theme,
        SettingKey::Scale,
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
            SettingKey::Scale => "scale",
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
            "scale" => Some(SettingKey::Scale),
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
            SettingKey::Theme | SettingKey::Scale | SettingKey::PreviewFontSize => {
                Group::Appearance
            }
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
            SettingKey::Scale => "Interface scale",
            SettingKey::PreviewFontSize => "Preview text size",
        }
    }

    /// One sentence saying what the setting does and what its range means. The settings screen
    /// and the README both describe the same keys, so the description lives here.
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
            SettingKey::Theme => "Dark or light. Both are drawn from the same palette.",
            SettingKey::Scale => "Scales the whole interface; Reset puts it back at 1.00.",
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
            SettingKey::Scale => SettingValue::Scale(Settings::DEFAULT_SCALE),
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
            SettingKey::Scale => SettingValue::Scale(settings.scale),
            SettingKey::PreviewFontSize => SettingValue::Number(settings.preview_font_size),
        }
    }

    /// Read stored text as this key's value, refusing anything [`validate`](Self::validate) would.
    ///
    /// This is the only reader: a hand-edited row that no longer parses fails the snapshot with
    /// the key and the offending text, rather than being replaced with a default in silence.
    pub fn parse_value(self, text: &str) -> Result<SettingValue, StoreError> {
        let refused = || StoreError::Setting {
            key: self.as_str().to_string(),
            value: text.to_string(),
        };
        let value = match self {
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
            SettingKey::Scale => SettingValue::Scale(text.parse::<f32>().map_err(|_| refused())?),
        };
        self.validate(&value)?;
        Ok(value)
    }

    /// Whether this key accepts this value. The ranges are per group: whole numbers check their
    /// bounds, a scheduler or a theme is whichever variant it is, and the weights go through the
    /// parser FSRS itself trusts.
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
                in_range(*n, crate::store::MIN_RETENTION, crate::store::MAX_RETENTION)
            }
            (SettingKey::Weights, SettingValue::Weights(weights)) => fsrs::are_weights(weights),
            (SettingKey::ReverseDefault, SettingValue::Flag(_)) => true,
            (SettingKey::Theme, SettingValue::Theme(_)) => true,
            (SettingKey::Scale, SettingValue::Scale(scale)) => in_scale(*scale),
            (SettingKey::PreviewFontSize, SettingValue::Number(n)) => {
                in_range(*n, MIN_PREVIEW_FONT_SIZE, MAX_PREVIEW_FONT_SIZE)
            }
            // A number for a flag, a theme for a scheduler: the caller picked the wrong pair even
            // when the value would be valid for some other key.
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
            (SettingKey::Scale, SettingValue::Scale(scale)) => settings.scale = scale,
            (SettingKey::PreviewFontSize, SettingValue::Number(n)) => {
                settings.preview_font_size = n
            }
            // `validate` has already refused every other pair.
            _ => {}
        }
    }
}

/// A setting's value, typed so a caller cannot hand a flag to a number.
#[derive(Clone, Debug, PartialEq)]
pub enum SettingValue {
    /// A whole number: a cap, a step in minutes, a percentage or a size in points.
    Number(u32),
    /// An on/off switch, stored as `1` or `0`.
    Flag(bool),
    Scheduler(SchedulerKind),
    /// The nineteen FSRS weights, checked before they are written.
    Weights(Vec<f64>),
    Theme(Theme),
    /// The interface scale, kept as a float so 0.85 survives a round trip.
    Scale(f32),
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
            SettingValue::Scale(scale) => scale.to_string(),
        }
    }

    /// The value shown on the settings screen: flags and themes in words, everything else in the
    /// spelling a reader would type.
    pub fn display(&self) -> String {
        match self {
            SettingValue::Flag(true) => "On".to_string(),
            SettingValue::Flag(false) => "Off".to_string(),
            SettingValue::Theme(theme) => theme.label().to_string(),
            SettingValue::Scale(scale) => format!("{scale:.2}"),
            SettingValue::Number(n) => n.to_string(),
            SettingValue::Scheduler(kind) => kind.as_str().to_string(),
            SettingValue::Weights(weights) => format!("{} numbers", weights.len()),
        }
    }
}

impl Settings {
    /// How long a learning step waits, in milliseconds.
    pub fn learning_step_ms(&self) -> i64 {
        i64::from(self.learning_step_minutes) * MINUTE_MS
    }

    /// The furthest a remembered card can be scheduled, in milliseconds.
    pub fn max_interval_ms(&self) -> i64 {
        i64::from(self.max_interval_days) * DAY_MS
    }

    /// The value of `key` in this library.
    pub fn value(&self, key: SettingKey) -> SettingValue {
        key.value(self)
    }
}

fn in_range(value: u32, min: u32, max: u32) -> bool {
    (min..=max).contains(&value)
}

fn in_scale(value: f32) -> bool {
    value.is_finite() && (MIN_SCALE..=MAX_SCALE).contains(&value)
}
