//! Markdown content: a card's text, rendered as the app draws prose.

use nsh_base::text_view;
use nsh_core::prelude::*;

use super::tokens;

/// The markdown `source` drawn as prose, in the foreground colour, at the body size.
pub fn prose(source: impl Fn() -> SharedString + 'static) -> impl IntoElement {
    text_view()
        .markdown(move || Some(source()))
        .text_sm()
        .text_color(tokens::foreground)
}

pub use nsh_base::text_view::CodeBlock;

/// Markdown whose fenced blocks are drawn by `block`, for content that carries app blocks such as
/// card embeds. Every other part of the markdown is drawn as [`prose`] draws it.
pub fn prose_with_blocks(
    source: SharedString,
    block: impl Fn(&CodeBlock) -> AnyElement + 'static,
) -> impl IntoElement {
    text_view()
        .markdown(move || Some(source.clone()))
        .gap_3()
        .text_sm()
        .text_color(tokens::foreground)
        .render_code_block(move |code: &CodeBlock| block(code))
}
