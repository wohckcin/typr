//! The app's single-line text field: one border, one focus ring, the same padding as its label.

use nsh_base::input;
use nsh_core::prelude::*;

use super::tokens;

/// A text field bound to `value`; typing writes the whole text back through `on_change`.
pub fn field(placeholder: &'static str, value: Signal<String>) -> impl IntoElement {
    input()
        .value(move || value.get())
        .on_change(move |text: &str| value.set(text.to_owned()))
        .placeholder(placeholder)
        .h(px(32.0))
        .px_3()
        .text_sm()
        .rounded(tokens::radius)
        .border_1()
        .border_color(move || tokens::border())
        .focus(|s| s.border_color(move || tokens::ring()))
}

/// A text field that runs `on_submit` when Return is pressed in it, for a field that commits a
/// command (the palette) rather than editing a value that saves elsewhere.
pub fn submit_field(
    placeholder: &'static str,
    value: Signal<String>,
    on_submit: impl Fn(&str) + 'static,
) -> impl IntoElement {
    input()
        .value(move || value.get())
        .on_change(move |text: &str| value.set(text.to_owned()))
        .submit_behavior(SubmitBehavior::Submit)
        .on_submit(on_submit)
        .placeholder(placeholder)
        .h(px(32.0))
        .px_3()
        .text_sm()
        .rounded(tokens::radius)
        .border_1()
        .border_color(move || tokens::border())
        .focus(|s| s.border_color(move || tokens::ring()))
}
