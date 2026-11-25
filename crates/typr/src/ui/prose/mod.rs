//! Markdown content: a card's text, rendered as the app draws prose.

use nsh_base::text_view;

mod extensions;
use nsh_core::element::Prop;
use nsh_core::prelude::*;

use super::tokens;

/// A prose block's props.
#[derive(Clone, PartialEq, Props)]
#[non_exhaustive]
pub struct ProseProps {
    /// The markdown it draws.
    pub source: Prop<SharedString>,
}

/// The markdown `source` drawn as prose, in the foreground colour, at the body size.
#[component]
pub fn Prose(props: ProseProps) -> impl IntoElement {
    text_view()
        .markdown(move || Some(props.source.get()))
        .extensions(extensions::extensions())
        .render_custom(extensions::render)
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
        .extensions(extensions::extensions())
        .render_custom(extensions::render)
        .gap_3()
        .text_sm()
        .text_color(tokens::foreground)
        .render_code_block(move |code: &CodeBlock| block(code))
}
