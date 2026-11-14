//! A card's review count and full record list, so the schedule can be argued with.

use rune::tok::{self, type_};
use rune::{El, ScrollSpec, col, row, text};
use rune_kit::root;

use crate::store::{Grade, Id, ReviewRow};

/// One card's history, as the screen shows it.
#[derive(Debug, Default)]
pub struct History {
    pub card: Option<Id>,
    /// Records that count as reviews: `learn` rows are learning cycles, not reviews.
    pub count: u32,
    pub rows: Vec<ReviewRow>,
}

impl History {
    /// Show one card's records. `reviews` comes from the store, so the rule for what counts as a
    /// review lives in one place.
    pub fn set(&mut self, card: Id, rows: Vec<ReviewRow>, reviews: u32) {
        self.card = Some(card);
        self.count = reviews;
        self.rows = rows;
    }
}

/// What the history panel needs from the app.
pub trait Host: 'static {
    fn history(&self) -> &History;
    fn history_mut(&mut self) -> &mut History;
}

impl Host for History {
    fn history(&self) -> &History {
        self
    }

    fn history_mut(&mut self) -> &mut History {
        self
    }
}

/// A moment as the user's clock shows it, `2026-11-14 09:05`.
fn when(at_ms: i64) -> String {
    let seconds = i32::try_from(crate::day_offset::local_offset_ms() / 1000).unwrap_or(0);
    let Ok(offset) = time::UtcOffset::from_whole_seconds(seconds) else {
        return String::new();
    };
    let Ok(moment) = time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(at_ms) * 1_000_000)
    else {
        return String::new();
    };
    let local = moment.to_offset(offset);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        local.year(),
        u8::from(local.month()),
        local.day(),
        local.hour(),
        local.minute()
    )
}

fn row_view(entry: &ReviewRow) -> El {
    let t = tok::theme(true);
    let grade = match entry.grade {
        Grade::Remembered => "remembered",
        Grade::Forgot => "forgot",
    };
    let due = match entry.due_at {
        Some(due_at) => when(due_at),
        None => "none".to_string(),
    };
    row()
        .key(format!("history-{}", entry.at))
        .w_full()
        .gap(16.0)
        .child(
            text(when(entry.at))
                .color(t.text_secondary)
                .font(type_::MONO_CAPTION),
        )
        .child(
            text(entry.phase.as_str())
                .color(t.text_secondary)
                .font(type_::MONO_CAPTION),
        )
        .child(text(grade).color(t.text).font(type_::MONO_CAPTION))
        .child(
            text(format!("due {due}"))
                .color(t.text_secondary)
                .font(type_::MONO_CAPTION),
        )
}

/// The panel: a count that only reviews contribute to, then every record.
pub fn view<S: Host>(s: &S) -> El {
    let t = tok::theme(true);
    let history = s.history();
    let mut list = col()
        .key("history")
        .w_full()
        .gap(8.0)
        .child(
            text(format!("{} reviews", history.count))
                .key("history-count")
                .font(type_::LABEL)
                .color(t.text),
        )
        .child(
            text(format!("{} records", history.rows.len()))
                .key("history-records")
                .font(type_::LABEL)
                .color(t.text_secondary),
        );
    for entry in &history.rows {
        list = list.child(row_view(entry));
    }
    root(
        list.scroll(ScrollSpec {
            stick_bottom: false,
        }),
        Vec::new(),
    )
}

#[cfg(test)]
mod tests;
