//! A card: one bordered surface with a radius, holding everything it shows.

use nsh_core::prelude::*;

use super::tokens;

/// A surface holding `content`, padded from the scale and filled with the card colour.
pub fn card(content: impl IntoElement) -> impl IntoElement {
    view()
        .p_4()
        .gap_2()
        .rounded(tokens::radius)
        .bg(tokens::card)
        .border_1()
        .border_color(tokens::border)
        .child(content)
}

/// A hairline between rows or sections: one device pixel, drawn once by its owner.
pub fn separator() -> impl IntoElement {
    view().h(px(1.0)).w_full().bg(tokens::border)
}

/// A tile-shaped placeholder for a slot with nothing in it yet: a dashed border, not a filled card.
pub fn placeholder(content: impl IntoElement) -> impl IntoElement {
    view()
        .p_4()
        .gap_2()
        .rounded(tokens::radius)
        .border_1()
        .border_dashed()
        .border_color(tokens::border)
        .child(content)
}
