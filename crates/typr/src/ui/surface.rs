//! A card: one bordered surface with a radius, holding everything it shows.

use nsh_core::prelude::*;

use super::tokens;

/// A surface holding `content`, padded from the scale and filled with the card colour.
pub fn card(content: impl IntoElement) -> impl IntoElement {
    view()
        .p_4()
        .gap_2()
        .rounded(tokens::radius)
        .bg(move || tokens::card())
        .border_1()
        .border_color(move || tokens::border())
        .child(content)
}

/// A hairline between rows or sections: one device pixel, drawn once by its owner.
pub fn separator() -> impl IntoElement {
    view().h(px(1.0)).w_full().bg(move || tokens::border())
}
