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
