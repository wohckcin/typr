//! A card: one bordered surface with a radius, holding everything it shows.

use nsh_core::prelude::*;

use super::tokens;

/// A surface's props.
#[derive(Clone, PartialEq, Props)]
pub struct SurfaceProps {
    /// What it holds.
    pub children: Children,
}

/// A surface holding its children, padded from the scale and filled with the card colour.
#[component]
pub fn Surface(props: SurfaceProps) -> impl IntoElement {
    view()
        .p_4()
        .gap_2()
        .rounded(tokens::radius)
        .bg(tokens::card)
        .border_1()
        .border_color(tokens::border)
        .child(props.children)
}

/// A placeholder's props.
#[derive(Clone, PartialEq, Props)]
pub struct PlaceholderProps {
    /// What it stands in for.
    pub children: Children,
}

/// A tile-shaped placeholder for a slot with nothing in it yet: a dashed border, not a filled card.
#[component]
pub fn Placeholder(props: PlaceholderProps) -> impl IntoElement {
    view()
        .p_4()
        .gap_2()
        .rounded(tokens::radius)
        .border_1()
        .border_dashed()
        .border_color(tokens::border)
        .child(props.children)
}
