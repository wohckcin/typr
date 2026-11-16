//! Every colour and radius a screen draws with, by role.
//!
//! A screen names the role (`tokens::card()`), never a token string or a colour value, so one edit
//! here restyles every screen an it follows the appearance on its own: each accessor returns the
//! theme's `AdaptiveColor`, which resolves per commit.

use nsh_base::{theme, tokens_of};
use nsh_core::prelude::{AdaptiveColor, Points, px};

/// The app's theme file: the default theme with the roles this app adds.
pub const THEME: &str = include_str!("../../theme.json");

/// Installs [`THEME`] for the running engine.
///
/// `nsh run` reads `nsh.json`'s `"theme"` before the first frame; this is how a test, or a run
/// that starts an engine itself, gets the same tokens.
pub fn install() {
    match tokens_of(Some(THEME)) {
        Ok(tokens) => nsh_core::element::set_tokens(tokens),
        Err(error) => log::error!("theme: {error}"),
    }
}

/// The page behind the app's content.
pub fn background() -> AdaptiveColor {
    theme().background
}

/// Body text: a card's text, a control's label.
pub fn foreground() -> AdaptiveColor {
    theme().foreground
}

/// A card, a tile, a surface holding one object.
pub fn card() -> AdaptiveColor {
    theme().card
}

/// Secondary text: a caption, a metadata line, a hint.
pub fn muted_foreground() -> AdaptiveColor {
    theme().muted_foreground
}

/// A hairline: a surface's border, a rule between rows.
pub fn border() -> AdaptiveColor {
    theme().border
}

/// The wash a hovered or a highlighted row takes.
pub fn hover() -> AdaptiveColor {
    theme().accent
}

/// The fill of a selected row, a chosen tab, a pressed control.
pub fn selection() -> AdaptiveColor {
    theme().primary
}

/// Text and glyphs drawn on [`selection`].
pub fn selection_foreground() -> AdaptiveColor {
    theme().primary_foreground
}

/// A focused surface's ring.
pub fn ring() -> AdaptiveColor {
    theme().ring
}

/// A destructive action and its text.
pub fn destructive() -> AdaptiveColor {
    theme().destructive
}

/// The canvas a review is read on: the page dimmed a step, so the card sits above it.
///
/// A misspelled token falls back to the page, which shows as a card with no canvas rather than
/// silently looking plausible.
pub fn review_backdrop() -> AdaptiveColor {
    theme().color("review-backdrop").unwrap_or(theme().background)
}

/// The radius of a card surface and of a small control inside one.
pub fn radius() -> Points {
    theme().radius
}

/// The width of the sidebar: a layout measurement the screens take rather than write.
pub fn sidebar_width() -> Points {
    px(220.0)
}

/// The fill a row takes when it is the one in force: the hover wash, which is visibly a fill and
/// never transparent. An unselected row takes the page's background, so the two always differ.
pub fn row_fill(selected: bool) -> AdaptiveColor {
    if selected { hover() } else { background() }
}


/// The height a card tile is estimated at before it is measured: the list's scroll bar uses it.
pub fn tile_estimate() -> Points {
    px(96.0)
}

/// The narrowest a card tile may be, in points: a line of tiles wraps before a tile gets narrower.
pub const TILE_MIN_WIDTH: f32 = 240.0;

/// How many tiles fit across a window `width` points wide, one gap apart, inside the page's padding.
/// Never fewer than one: a narrow window shows one tile per line rather than none.
pub fn grid_columns(width: f32) -> u32 {
    let gap = 8.0;
    let padding = 32.0;
    let usable = (width - padding).max(0.0);
    ((usable + gap) / (TILE_MIN_WIDTH + gap)).floor().max(1.0) as u32
}

/// Roughly how many points a character of tag text takes, and a chip's own padding.
const CHAR_WIDTH: f32 = 7.0;
const CHIP_PADDING: f32 = 16.0;

/// How many of `tags` fit across a tile `width` points wide, in the order they were set; the rest
/// are counted. A chip is estimated from its text, since a tile's text is measured only when drawn.
pub fn tags_that_fit(width: f32, tags: &[&str]) -> usize {
    let mut used = 0.0;
    let mut fit = 0;
    for tag in tags {
        let chip = tag.chars().count() as f32 * CHAR_WIDTH + CHIP_PADDING;
        if used + chip > width {
            break;
        }
        used += chip + 4.0;
        fit += 1;
    }
    fit
}

/// No inset: a layer that covers its parent exactly.
pub fn flush() -> nsh_core::prelude::Points {
    nsh_core::prelude::px(0.0)
}
