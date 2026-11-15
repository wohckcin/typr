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
