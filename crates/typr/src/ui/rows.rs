//! Settings as rows in one card: a label at the leading edge, its control at the trailing edge.
//!
//! The card is the border colour with a hairline gap between rows, and each row is the card colour,
//! so every line between two rows is drawn once, by the card, and a row hidden by a search leaves
//! neither a double line nor a stray one.

use nsh_core::prelude::*;

use super::{size::Size, tokens};

/// The card holding `rows`, drawn by the caller as [`setting_row`]s.
pub fn rows() -> View {
    view()
        .rounded(tokens::radius)
        .overflow_hidden()
        .border_1()
        .border_color(tokens::border)
        .bg(tokens::border)
        .gap(px(1.0))
}

/// One setting: `label` at the start, `control` at the end. A control that does not fit wraps under
/// the label rather than leaving the card.
pub fn setting_row(label: &'static str, control: impl IntoElement) -> impl IntoElement {
    view()
        .flex_row()
        .flex_wrap()
        .items_center()
        .justify_between()
        .gap_3()
        .px_4()
        .py_3()
        .min_h(Size::Lg.min_height())
        .bg(tokens::card)
        .child(text(label).text_sm().text_color(tokens::foreground))
        .child(control)
}
