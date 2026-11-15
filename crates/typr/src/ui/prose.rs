//! Markdown content: a card's text, rendered as the app draws prose.

use nsh_base::text_view;
use nsh_core::prelude::*;

use super::tokens;

/// The markdown `source` drawn as prose, in the foreground colour, at the body size.
pub fn prose(source: impl Fn() -> SharedString + 'static) -> impl IntoElement {
    text_view()
        .markdown(move || Some(source()))
        .text_sm()
        .text_color(move || tokens::foreground())
}
