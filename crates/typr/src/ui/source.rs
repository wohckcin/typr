//! The markdown source editor: a card's text, written in the monospaced code editor.

use nsh_base::editor;
use nsh_base::editor::{Editor, EditorApi};
use nsh_core::prelude::*;

use super::tokens;

/// A handle on a source editor, so a screen can type into it at its caret.
pub fn source_handle() -> ComponentRef<Editor> {
    component_ref()
}

/// Types `text` at the editor's caret (in place of its selection), the way a paste would, and says
/// whether the editor took it: `false` while the editor is not mounted.
pub fn insert_at_caret(handle: &ComponentRef<Editor>, text: &str) -> bool {
    match handle.api::<EditorApi>() {
        Some(api) => {
            api.insert(text.to_owned());
            true
        }
        None => false,
    }
}

/// A markdown source that reports each edit, for a draft that must be written down as it is typed.
/// `handle`, when given, is where the editor is reached so a reference can be typed at its caret.
pub fn source_with(
    value: Signal<String>,
    on_change: impl Fn(&str) + 'static,
    handle: Option<&ComponentRef<Editor>>,
) -> impl IntoElement {
    let source = editor()
        .value(move || Some(value.get().into()))
        .language("markdown")
        .accessibility_label("Card source")
        .on_change(move |text: &SharedString| {
            value.set(text.to_string());
            on_change(text);
        })
        .flex_1()
        .border_1()
        .border_color(tokens::border);
    match handle {
        Some(handle) => source.component_ref(handle).into_element(),
        None => source.into_element(),
    }
}
