//! Card embeds: `![[Name]]` in a card's text draws that card's body where it stands, set apart from
//! the text around it. An embed is one level deep: an embedded body is drawn as written, and a card
//! that embeds itself draws a marker instead of repeating its own text.
//!
//! The rewrite is typr-core's (`markdown_ext::expand`); this screen only draws the blocks it leaves.

use nsh_core::prelude::*;
use typr_core::markdown_ext::{INFO, MISSING, expand};

use crate::ui::prose::{CodeBlock, prose, prose_with_blocks};
use crate::ui::{surface, tokens};

/// The name and body of every card the embeds may name.
pub type Cards = Vec<(String, String)>;

fn body_of<'a>(cards: &'a Cards, name: &str) -> Option<&'a str> {
    cards
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, body)| body.as_str())
}

/// The card's text with its embeds drawn. `this` is the card's own name, which an embed of itself
/// is checked against.
pub fn embedded(markdown: &str, cards: Cards, this: String) -> impl IntoElement {
    let source = expand(markdown, |name| body_of(&cards, name).map(str::to_owned));
    let render_cards = cards.clone();
    let render_this = this.clone();
    prose_with_blocks(source.into(), move |block: &CodeBlock| {
        let name = block.code.trim().to_owned();
        match block.language.as_deref() {
            Some(INFO) if name == render_this => surface::card(
                text(format!("“{name}” embeds itself"))
                    .text_sm()
                    .text_color(tokens::muted_foreground),
            )
            .into_element(),
            Some(INFO) => {
                let body = body_of(&render_cards, &name).unwrap_or("").to_owned();
                // One level only: a body that embeds another card is named, not drawn.
                if !typr_core::markdown_ext::embeds(&body).is_empty() {
                    return surface::card(
                        text(format!("“{name}” embeds another card"))
                            .text_sm()
                            .text_color(tokens::muted_foreground),
                    )
                    .into_element();
                }
                surface::card(prose(move || body.clone().into())).into_element()
            }
            Some(MISSING) => surface::card(
                text(format!("No card named {name}"))
                    .text_sm()
                    .text_color(tokens::destructive),
            )
            .into_element(),
            _ => view().child(text(block.code.clone())).into_element(),
        }
    })
}
