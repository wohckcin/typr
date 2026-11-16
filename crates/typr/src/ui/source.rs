//! The markdown source editor: a card's text, written in the monospaced code editor.

use nsh_base::editor;
use nsh_core::prelude::*;

use super::tokens;

/// A markdown source bound to `value`; each edit writes the whole text back.
pub fn source(value: Signal<String>) -> impl IntoElement {
    editor()
        .value(move || Some(value.get().into()))
        .language("markdown")
        .on_change(move |text: &SharedString| value.set(text.to_string()))
        .flex_1()
        .border_1()
        .border_color(move || tokens::border())
}

/// A markdown source that reports each edit, for a draft that must be written down as it is typed.
pub fn source_with(
    value: Signal<String>,
    on_change: impl Fn(&str) + 'static,
) -> impl IntoElement {
    editor()
        .value(move || Some(value.get().into()))
        .language("markdown")
        .accessibility_label("Card source")
        .on_change(move |text: &SharedString| {
            value.set(text.to_string());
            on_change(text);
        })
        .flex_1()
        .border_1()
        .border_color(move || tokens::border())
}
