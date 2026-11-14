//! One card as a tile: its first side rendered, what it hides, where its schedule stands, and its
//! tags. `card-grid` and later views share this constructor so a card looks the same wherever it is
//! listed.

use rune::tok::{self, type_};
use rune::{El, col, memo, row, text};
use rune_kit::{Markdown, MarkdownStyle};

use super::{Host, TILE_MAX_HEIGHT};
use typr_core::store::{CardSummary, Id};

/// A tile's corner radius.
pub const RADIUS: f32 = 8.0;
/// The strip a hidden-side count is written on, at the bottom of the body.
pub const STRIP_HEIGHT: f32 = 22.0;
/// The footer that states where the card's schedule stands.
pub const FOOTER_HEIGHT: f32 = 26.0;
/// The padding inside a tile's body and footer.
pub const PADDING: f32 = 10.0;

/// How many sides a card hides, which is every side but the one the tile shows.
pub fn hidden_sides(card: &CardSummary) -> usize {
    (card.sides as usize).saturating_sub(1)
}

/// What a tile says about the card's schedule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// A note: the card carries no prompt at all.
    Note,
    /// A prompt that has never been scheduled.
    New,
    /// A scheduled prompt whose due time has passed.
    Due,
    /// A scheduled prompt whose due time is still ahead, with its interval in whole days.
    Interval(i64),
}

/// Which state a card is in at `now_ms`, read from the snapshot's prompt fields.
pub fn state(card: &CardSummary, now_ms: i64) -> State {
    if card.prompts == 0 {
        return State::Note;
    }
    match card.due_at {
        None => State::New,
        Some(due) if due <= now_ms => State::Due,
        Some(due) => State::Interval(day_round_up(due.saturating_sub(now_ms))),
    }
}

/// The interval in whole days, rounded up so a prompt due in an hour does not read as zero days.
fn day_round_up(ms: i64) -> i64 {
    const DAY: i64 = 24 * 60 * 60 * 1000;
    ms.div_euclid(DAY) + i64::from(ms.rem_euclid(DAY) > 0)
}

/// The words a tile shows for a state: the footer's word and its uppercase kicker.
pub fn words(state: State) -> (String, &'static str) {
    match state {
        State::Note => ("Note".to_string(), "NOTE"),
        State::New => ("New".to_string(), "NEW"),
        State::Due => ("Due".to_string(), "DUE"),
        State::Interval(1) => ("1 day".to_string(), "DAY"),
        State::Interval(days) => (format!("{days} days"), "DAYS"),
    }
}

/// The chips a tile fits in `width`, and how many it had to drop.
pub fn chips(tags: &[String], width: f32) -> (Vec<&String>, usize) {
    let mut used = 0.0;
    let mut shown = 0;
    for tag in tags {
        let tag_width = chip_width(tag);
        if used + tag_width > width && shown > 0 {
            break;
        }
        used += tag_width;
        shown += 1;
    }
    (tags[..shown].iter().collect(), tags.len() - shown)
}

/// How wide a chip is: its estimate at 11 points plus the chip's padding and its gap.
fn chip_width(tag: &str) -> f32 {
    const GLYPH: f32 = 6.0;
    const PADDING: f32 = 12.0;
    const GAP: f32 = 4.0;
    tag.chars().count() as f32 * GLYPH + PADDING + GAP
}

/// One tile. The width is the tile's share of its line, so nothing here fixes it.
pub fn tile<S: Host>(card: &CardSummary, width: f32, now_ms: i64) -> El {
    let t = tok::theme(true);
    let id = card.id;
    let sides = hidden_sides(card);
    let (word, kicker) = words(state(card, now_ms));

    let mut body = col()
        .key("body")
        .grow(1.0)
        .w_full()
        .padding(PADDING)
        .gap(4.0)
        .child(memo("tile-side", side_key(card), || {
            Markdown::new(first_side(card)).render(&MarkdownStyle::neutral(true))
        }));
    if sides > 0 {
        body = body.child(hidden_strip(sides));
    }

    let mut footer = row()
        .key("footer")
        .w_full()
        .h(FOOTER_HEIGHT)
        .px(PADDING)
        .center_y()
        .gap(6.0)
        .fill(t.control_off.alpha(0.1))
        .child(text(word).font(type_::LABEL).grow(1.0).color(t.text))
        .child(
            text(kicker)
                .font(type_::MONO_CAPTION)
                .color(t.text_secondary),
        );
    if !card.tags.is_empty() {
        footer = footer.child(chip_row(card.tags.clone(), width - 2.0 * PADDING));
    }

    // The tile is the card's primary action, the way a row was: assistive technology exposes one
    // button per tile, so a press opens the card.
    col()
        .key(format!("tile-{id}"))
        .w(width)
        .max_h(TILE_MAX_HEIGHT)
        .radius(RADIUS)
        .clip_rounded(RADIUS)
        .fill(t.card)
        .child(body)
        .child(footer)
        .on_press(move |s: &mut S| {
            s.library_mut().selected_card = Some(id);
            s.open_card(id);
        })
}

/// The full-width strip that states how many sides a tile hides.
fn hidden_strip(sides: usize) -> El {
    let t = tok::theme(true);
    let label = if sides == 1 {
        "1 hidden side".to_string()
    } else {
        format!("{sides} hidden sides")
    };
    row()
        .key("hidden")
        .w_full()
        .h(STRIP_HEIGHT)
        .px(PADDING)
        .center_y()
        .fill(t.control_off.alpha(0.1))
        .child(
            text(label.to_uppercase())
                .font(type_::MONO_CAPTION)
                .color(t.text_secondary),
        )
}

/// The chips a tile fits, and how many it had to drop.
fn chip_row(tags: Vec<String>, width: f32) -> El {
    let t = tok::theme(true);
    let (shown, dropped) = chips(&tags, width);
    let mut chips = row().key("tags").gap(4.0).center_y();
    for (index, tag) in shown.iter().enumerate() {
        chips = chips.child(
            row()
                .key(format!("chip-{index}"))
                .px(6.0)
                .radius(4.0)
                .fill(t.control_off.alpha(0.6))
                .child(text((*tag).clone()).font(type_::MONO_CAPTION).color(t.text)),
        );
    }
    if dropped > 0 {
        chips = chips.child(
            text(format!("+{dropped}"))
                .font(type_::MONO_CAPTION)
                .color(t.text_secondary),
        );
    }
    chips
}

/// The card's first side, which is what a tile shows.
pub fn first_side(card: &CardSummary) -> String {
    typr_core::sides::split(&card.markdown)
        .first()
        .map(|side| (*side).to_string())
        .unwrap_or_default()
}

/// A key that changes when the card's first side or its identity does, so the parse is reused
/// between frames and redone when the text changes.
pub(super) fn side_key(card: &CardSummary) -> u64 {
    let mut key = card.id as u64;
    key = key.wrapping_mul(31).wrapping_add(card.updated_at as u64);
    key ^ Id::from(card.sides as i64) as u64
}

#[cfg(test)]
mod tests;
