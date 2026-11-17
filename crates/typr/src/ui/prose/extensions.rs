//! The card syntax the renderer does not read: `==highlight==`, `20^th^` and `{base}(reading)`.
//!
//! Each is a `MarkdownExtension::pattern`, so the parser matches it in text outside code and links, and the
//! nodes it makes are drawn here. Everything else — CommonMark, GFM, subscript, math — is the renderer's.

use nsh_base::text_view::{CustomNode, MarkdownExtension};
use nsh_core::prelude::*;

use super::tokens;

/// The node name the highlight pattern makes.
pub const HIGHLIGHT: &str = "typr-highlight";
/// The node name the superscript pattern makes.
pub const SUPERSCRIPT: &str = "typr-superscript";
/// The node name the ruby pattern makes.
pub const RUBY: &str = "typr-ruby";

/// The patterns, in the order the parser tries them.
pub fn extensions() -> Vec<MarkdownExtension> {
    vec![
        MarkdownExtension::pattern(HIGHLIGHT, r"==([^=\n]+)=="),
        MarkdownExtension::pattern(SUPERSCRIPT, r"\^([^\^\n]+)\^"),
        MarkdownExtension::pattern(RUBY, r"\{([^}\n]*)\}\(([^)\n]*)\)"),
    ]
}

/// Draws one of the nodes the patterns make. A name this does not know keeps its text as written.
pub fn render(node: &CustomNode) -> AnyElement {
    let group =
        |at: &str| -> SharedString { node.attributes.get(at).cloned().unwrap_or_default().into() };
    match node.name.as_str() {
        HIGHLIGHT => view()
            .rounded(px(2.0))
            .bg(tokens::selection())
            .child(text(group("1")).text_color(tokens::selection_foreground()))
            .into_element(),
        SUPERSCRIPT => view()
            .mt(px(-3.0))
            .child(text(group("1")).text_xs().text_color(tokens::foreground))
            .into_element(),
        RUBY => view()
            .flex_col()
            .items_center()
            .child(
                text(group("2"))
                    .text_xs()
                    .text_color(tokens::muted_foreground),
            )
            .child(text(group("1")).text_color(tokens::foreground))
            .into_element(),
        _ => text(node.text.clone()).into_element(),
    }
}
