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
