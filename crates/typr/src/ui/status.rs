//! Small status pieces: a badge for a short state, an empty state that names the next action.

use nsh_core::prelude::*;

use super::tokens;

/// A short state or count, in the muted colours: neutral unless a caller needs a stronger one.
pub fn badge(label: impl IntoReactive<SharedString>) -> impl IntoElement {
    view()
        .px_2()
        .rounded(tokens::radius)
        .border_1()
        .border_color(tokens::border)
        .child(text(label).text_xs().text_color(tokens::muted_foreground))
}

/// What a screen shows when it has nothing: what is missing, and the action that fills it.
pub fn empty(message: &'static str, action: impl IntoElement) -> impl IntoElement {
    view()
        .p_6()
        .gap_3()
        .items_center()
        .child(text(message).text_color(tokens::muted_foreground))
        .child(action)
}

/// A pulsing placeholder of a known shape while content loads.
pub fn skeleton() -> impl IntoElement {
    view()
        .h(px(16.0))
        .w(px(160.0))
        .rounded(tokens::radius)
        .bg(tokens::hover)
}

/// Reserved for an indeterminate wait: a control that is working, not a decoration.
pub fn spinner_label(label: &'static str) -> impl IntoElement {
    text(label).text_sm().text_color(tokens::muted_foreground)
}

/// A key's name, drawn as a small keycap.
pub fn keycap(chord: impl IntoReactive<SharedString>) -> impl IntoElement {
    view()
        .px_2()
        .rounded(tokens::radius)
        .border_1()
        .border_color(tokens::border)
        .child(text(chord).text_xs().text_color(tokens::muted_foreground))
}
