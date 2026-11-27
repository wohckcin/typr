//! Every colour and radius a screen draws with, by role.
//!
//! A screen names the role (`tokens::card()`), never a token string or a colour value, so one edit
//! here restyles every screen an it follows the appearance on its own: each accessor returns the
//! theme's `AdaptiveColor`, which resolves per commit.

use nsh_base::{theme, tokens_of};
use nsh_core::prelude::{AdaptiveColor, Points, Styled, px};

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

/// A surface that floats over the page: a menu, a popover, a dialog.
pub fn popover() -> AdaptiveColor {
    theme().popover
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

/// A measured layout length in points, for runtime geometry such as diagram mask overlays.
pub fn points(value: f32) -> Points {
    px(value)
}

/// A focused surface's ring.
pub fn ring() -> AdaptiveColor {
    theme().ring
}

/// The keyboard focus outline shared across every interactive control.
pub fn focus_ring<S: Styled>(style: S) -> S {
    style.outline(px(2.0)).outline_color(ring)
}

/// A destructive action and its text.
pub fn destructive() -> AdaptiveColor {
    theme().destructive
}

/// Something that is fine: an open library's dot. Never the only cue; a word names the state too.
///
/// A misspelled token falls back to the foreground, which shows as a dot that is not green.
pub fn success() -> AdaptiveColor {
    theme().color("success").unwrap_or(theme().foreground)
}

/// The canvas a review is read on: the page dimmed a step, so the card sits above it.
///
/// A misspelled token falls back to the page, which shows as a card with no canvas rather than
/// silently looking plausible.
pub fn review_backdrop() -> AdaptiveColor {
    theme()
        .color("review-backdrop")
        .unwrap_or(theme().background)
}

/// The radius of a card surface and of a small control inside one.
pub fn radius() -> Points {
    theme().radius
}

/// The width of the sidebar: a layout measurement the screens take rather than write.
pub fn sidebar_width() -> Points {
    px(220.0)
}

/// The height of the app's own title bar strip, the system's window controls centred in it.
pub fn title_bar_height() -> Points {
    px(32.0)
}

/// The main window's content size when it first opens.
pub fn window_size() -> nsh_core::prelude::Size<Points> {
    nsh_core::prelude::size(px(960.0), px(1280.0))
}

/// The fill a row takes when it is the one in force: the hover wash, which is visibly a fill and
/// never transparent. An unselected row takes the page's background, so the two always differ.
pub fn row_fill(selected: bool) -> AdaptiveColor {
    if selected { hover() } else { background() }
}

/// The height a card tile is estimated at before it is measured: the list's scroll bar uses it.
pub fn tile_estimate() -> Points {
    px(106.0)
}

/// The narrowest a card tile may be, in points: a line of tiles wraps before a tile gets narrower.
pub const TILE_MIN_WIDTH: f32 = 240.0;

/// The space between card tiles, across and down: under a tile's own padding, so tiles read as
/// separate cards without the grid looking sparse.
pub const GRID_GAP: f32 = 12.0;

/// [`GRID_GAP`] as a length.
pub fn grid_gap() -> Points {
    px(GRID_GAP)
}

/// How many tiles fit across a window `width` points wide, one gap apart, inside the page's padding.
/// Never fewer than one: a narrow window shows one tile per line rather than none.
pub fn grid_columns(width: f32) -> u32 {
    let padding = 32.0;
    let usable = (width - padding).max(0.0);
    let columns = ((usable + GRID_GAP) / (TILE_MIN_WIDTH + GRID_GAP))
        .floor()
        .max(1.0);
    // A window wider than four billion tiles is not worth a cast lint: saturate at the maximum.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let whole = columns.min(u32::MAX as f32) as u32;
    whole
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
