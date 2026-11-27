//! The glyphs a row leads with: outline icons drawn from inline SVG, tinted by the text colour of the
//! element they sit in (`currentColor`), so they follow the theme with no asset to load.
//!
//! The shapes follow the Lucide set (ISC licence, <https://lucide.dev>). An icon carries no name of its
//! own: it decorates a label that a screen reader already reads.

use nsh_core::element::{ImageSrc, PropEnum};
use nsh_core::prelude::*;

/// A glyph of the fixed set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PropEnum)]
pub enum Icon {
    Search,
    Inbox,
    CircleCheck,
    CircleDashed,
    Book,
    Archive,
    Plus,
    Download,
    Gear,
    Trash,
}

impl Icon {
    /// Every glyph, so a check can draw them all.
    pub const ALL: [Icon; 10] = [
        Icon::Search,
        Icon::Inbox,
        Icon::CircleCheck,
        Icon::CircleDashed,
        Icon::Book,
        Icon::Archive,
        Icon::Plus,
        Icon::Download,
        Icon::Gear,
        Icon::Trash,
    ];

    fn shapes(self) -> &'static str {
        match self {
            Icon::Search => r#"<circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/>"#,
            Icon::Inbox => {
                r#"<polyline points="22 12 16 12 14 15 10 15 8 12 2 12"/><path d="M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z"/>"#
            }
            Icon::CircleCheck => r#"<circle cx="12" cy="12" r="10"/><path d="m9 12 2 2 4-4"/>"#,
            Icon::CircleDashed => r#"<circle cx="12" cy="12" r="10" stroke-dasharray="3 3"/>"#,
            Icon::Book => {
                r#"<path d="M4 19.5v-15A2.5 2.5 0 0 1 6.5 2H19a1 1 0 0 1 1 1v18a1 1 0 0 1-1 1H6.5a1 1 0 0 1 0-5H20"/>"#
            }
            Icon::Archive => {
                r#"<rect width="20" height="5" x="2" y="3" rx="1"/><path d="M4 8v11a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8"/><path d="M10 12h4"/>"#
            }
            Icon::Plus => r#"<path d="M5 12h14"/><path d="M12 5v14"/>"#,
            Icon::Download => {
                r#"<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" x2="12" y1="15" y2="3"/>"#
            }
            Icon::Gear => {
                r#"<circle cx="12" cy="12" r="3"/><path d="M12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9 7 7M17 17l2.1 2.1M4.9 19.1 7 17M17 7l2.1-2.1"/>"#
            }
            Icon::Trash => {
                r#"<path d="M3 6h18"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"/><path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>"#
            }
        }
    }

    pub fn markup(self) -> String {
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">{}</svg>"#,
            self.shapes()
        )
    }
}

/// The glyph, 16 points square, in the colour of the text around it.
pub fn icon(glyph: Icon) -> impl IntoElement {
    svg(ImageSrc::svg_markup(&glyph.markup()))
        .w(px(16.0))
        .h(px(16.0))
        .flex_shrink_0()
}
