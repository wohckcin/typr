//! The visual language: the one place that names the spacing, the type, the radii and the surfaces
//! a screen draws with, so a screen cannot invent a number of its own.
//!
//! Spacing is a 4-point scale ([`space`]); a box's width or height is the same scale ([`size`]).
//! Two dimensions sit off the scale on purpose: [`HAIRLINE`] for a border, which is one device
//! pixel rather than a step of rhythm, and [`TIGHT`] for the gap between the rows of one control
//! group, which is closer than one step allows. Type comes from the rune type scale and radii,
//! elevation and the focus ring from `Kit`, so switching the theme moves no dimension — only the
//! colours.

use rune::el::Shadow;
use rune::tok::{self, type_};
use rune::{El, TextStyle, col, text};
use rune_kit::Kit;
use std::sync::LazyLock;

/// One device pixel: a border, a rule, a marker bar.
pub const HAIRLINE: f32 = 1.0;

/// The gap between the rows of one control group: closer than the 4-point scale allows.
pub const TIGHT: f32 = 2.0;

/// The step of the spacing scale, in points.
pub const UNIT: f32 = 4.0;

/// The width of the ring a focused surface draws, just inside its border.
pub const FOCUS_WIDTH: f32 = TIGHT;

static KIT: LazyLock<Kit> = LazyLock::new(|| Kit::neutral(true));

/// `steps` of the 4-point scale, for spacing: padding, gaps and margins.
pub fn space(steps: u8) -> f32 {
    f32::from(steps) * UNIT
}

/// The same scale, named for a box's width or height rather than for distance between boxes.
pub fn size(steps: u8) -> f32 {
    space(steps)
}

/// Body copy: what a card's text and a control's label are drawn in.
pub fn type_body() -> TextStyle {
    type_::BODY
}

/// A heading: the same size as body copy at the heavy weight, so a title leads by weight.
pub fn type_title() -> TextStyle {
    type_::TITLE
}

/// Secondary text: a caption, a metadata line, a hint.
pub fn type_caption() -> TextStyle {
    type_::MONO_CAPTION
}

/// A caption on a tight line, for a label beside a control.
pub fn type_caption_tight() -> TextStyle {
    type_::MONO_CAPTION.lh(18.0)
}

/// Body copy inside a one-line control, where the line height is the control's own.
pub fn type_input() -> TextStyle {
    type_::BODY.lh(20.0)
}

/// The text an editor writes in: body copy on a line tall enough to read a paragraph.
pub fn type_editor() -> TextStyle {
    type_::BODY.lh(22.0)
}

/// The radius of a card surface and of a row's selection fill.
pub fn radius() -> f32 {
    KIT.radius.md
}

/// The radius of a small control inside a card: a chip, a badge.
pub fn radius_small() -> f32 {
    KIT.radius.sm
}

/// The radius of a large surface: a dialog, a review card.
pub fn radius_large() -> f32 {
    KIT.radius.lg
}

/// The colour a surface's border is drawn in.
pub fn surface_border() -> rune::Color {
    KIT.c.border_base
}

/// The colour a focused surface's ring is drawn in, inside its bounds.
pub fn focus_color() -> rune::Color {
    KIT.c.border_focus
}

/// One step of elevation: the shadow a card surface casts on the page.
pub fn elevation() -> Vec<Shadow> {
    KIT.elevation.raised.clone()
}

/// The canvas a review is read on: the page dimmed a step, so the card sits above it and the eye
/// goes to the card rather than to the screen.
pub fn review_backdrop() -> rune::Color {
    tok::theme(true).bg.mix(rune::Color::BLACK, 0.35)
}

/// The fill a card surface is painted in.
pub fn surface_fill() -> rune::Color {
    tok::theme(true).card
}

/// The overlay a hover paints on a surface: colour only, so the surface does not move.
pub fn hover_color() -> rune::Color {
    tok::theme(true).hover
}

/// A card: one bordered surface with a radius and one step of elevation, holding everything it
/// shows. `focused` adds the ring inside the border, which takes no layout space, so focus never
/// moves the card.
pub fn surface_card(children: Vec<El>, focused: bool) -> El {
    surface_card_on(children, focused, surface_fill())
}

/// [`surface_card`] painted in `fill`, for a surface that says something about its own state — a
/// selected card reads as chosen rather than as a different kind of object.
pub fn surface_card_on(children: Vec<El>, focused: bool, fill: rune::Color) -> El {
    let mut card = col()
        .radius(radius())
        .padding(space(4))
        .gap(space(1))
        .fill(fill)
        .border(HAIRLINE, surface_border());
    for layer in elevation() {
        card = card.box_shadow(layer);
    }
    if focused {
        card = card.box_shadow(Shadow {
            spread: FOCUS_WIDTH,
            color: focus_color(),
            inset: true,
            ..Shadow::default()
        });
    }
    card.children(children)
}

/// A label for the region a surface holds: secondary, above the content, never a rule.
pub fn section_label(name: &str) -> El {
    text(name)
        .font(type_caption())
        .color(tok::theme(true).text_secondary)
}

#[cfg(test)]
mod tests;
