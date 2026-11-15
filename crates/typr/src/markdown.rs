//! The one place a card's markdown becomes elements.
//!
//! `rune-kit` draws the syntax its parser has events for, and cannot be extended from outside: its
//! style fields are `pub(super)` and its events are laid out inside a paragraph, so no other crate can
//! add an inline background, a raised run or a gloss. What is left is to write those constructs in
//! forms the parser does see before it reads them — [`typr_core::markdown_ext`] — and to say, in one
//! list, what each construct renders as.
//!
//! Every card is drawn here, so the editor's preview, the review, the tile and an embedded card read
//! the same text.

use rune::El;
use rune_kit::{Markdown, MarkdownStyle};

/// The text the renderer is handed: the constructs it cannot see, written in forms it can.
pub fn text(source: &str) -> String {
    typr_core::markdown_ext::rewrite(source).into_owned()
}

/// A card's markdown as elements, in the neutral style a card reads with.
pub fn body(source: &str) -> El {
    Markdown::new(text(source)).render(&MarkdownStyle::neutral(true))
}

/// One card that writes every construct in `SUPPORTED`, as Mochi's shortcut panel spells it.
///
/// The degraded constructs come first: the kitchen-sink scene is one window tall, and those rewrites are
/// the ones a reviewer must see on the picture.
///
/// The inventory test compares these names with the list, and the render tests read the "renders as"
/// column, so a construct cannot be answered for in one place and left unproven in the other.
pub const CASES: &[(&str, &str)] = &[
    ("image", "![plate](plate.png)"),
    ("highlight", "==the Krebs cycle=="),
    ("ruby", "{漢字}(かんじ)"),
    ("superscript", "20^th^ century"),
    ("subscript", "H~2~0"),
    ("definition-list", "Krebs cycle\n: in the matrix"),
    ("inline-math", "the area is $\\pi r^2$"),
    ("display-math", "$$\n\\int_0^1 x\\,dx\n$$"),
    ("strong", "**the Krebs cycle**"),
    ("emphasis", "*the Krebs cycle*"),
    ("strikethrough", "~~the Krebs cycle~~"),
    ("inline-code", "`the Krebs cycle`"),
    ("code-fence", "```text\nthe Krebs cycle\n```"),
    ("table", "| stage | where |\n|---|---|\n| Krebs | matrix |"),
    ("task-list", "- [x] the Krebs cycle"),
    ("link", "[the Krebs cycle](https://example.com)"),
    ("heading", "## the Krebs cycle"),
    ("list", "- the Krebs cycle"),
    ("quote", "> the Krebs cycle"),
    ("raw-html", "<b>the Krebs cycle</b>"),
    ("footnote", "the Krebs cycle[^1]"),
    ("latex", "\\frac{a}{b}"),
];

#[cfg(test)]
#[path = "markdown/tests.rs"]
mod tests;
