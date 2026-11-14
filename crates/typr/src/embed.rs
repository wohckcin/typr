//! Drawing an embed: the card a name resolves to, and the inset its block becomes.
//!
//! Stored text keeps `![[Name]]`. The pass in [`typr_core::markdown_ext`] writes each one as a
//! `typr-embed` fence whose body is the name; this module answers that name with a card body and
//! draws it inside a bordered block, at the caption's size, so the source card stays recognisable
//! while the embedded text is still the card's own markdown.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::sync::LazyLock;

use rune::tok::{self, type_};
use rune::{El, col, text};
use rune_kit::{CodeFrame, Kit, Markdown, MarkdownStyle};

use typr_core::markdown_ext;
use typr_core::store::{CardSummary, Id, Snapshot, folded};

static KIT: LazyLock<Kit> = LazyLock::new(|| Kit::neutral(true));

/// What the caption under an embed that names nothing says.
pub const NO_SUCH_CARD: &str = "no such card";

/// The body of the card `name` names, if the library holds one.
///
/// The order is the store's own: the referring card's deck, then the decks above it, then the
/// whole library; the oldest card wins a tie, and a trashed card is not there at all. Rendering
/// reads the snapshot the store published, so the drawn card is the card a reference would reach.
pub fn lookup(snapshot: &Snapshot, from: Option<Id>, name: &str) -> Option<String> {
    let wanted = folded(name);
    if wanted.is_empty() {
        return None;
    }
    let mut visited = Vec::new();
    let mut deck = from
        .and_then(|id| snapshot.cards.iter().find(|card| card.id == id))
        .map(|card| card.deck);
    while let Some(id) = deck {
        // A deck tree the store would not publish would still not loop forever here.
        if visited.contains(&id) {
            break;
        }
        visited.push(id);
        if let Some(card) = oldest(snapshot, Some(id), &wanted) {
            return Some(card.markdown.clone());
        }
        deck = snapshot
            .decks
            .iter()
            .find(|deck| deck.id == id)
            .and_then(|deck| deck.parent);
    }
    oldest(snapshot, None, &wanted).map(|card| card.markdown.clone())
}

/// The oldest card called `wanted` on `deck`, or anywhere when the deck is the whole library.
fn oldest<'a>(snapshot: &'a Snapshot, deck: Option<Id>, wanted: &str) -> Option<&'a CardSummary> {
    snapshot
        .cards
        .iter()
        .filter(|card| {
            card.trashed_at.is_none()
                && !card.name.is_empty()
                && folded(&card.name) == wanted
                && deck.is_none_or(|id| card.deck == id)
        })
        .min_by_key(|card| card.id)
}

/// The bodies one drawn text's embeds resolved to, read once when the view is built.
///
/// The renderer's frame callback is `'static`, so it cannot ask the app anything while drawing;
/// the answers are taken before the frame and kept here.
pub(crate) struct Embeds {
    entries: Vec<(String, Option<String>)>,
}

impl Embeds {
    /// Resolve every name `text` embeds, in the order they were written.
    pub(crate) fn of(text: &str, resolve: impl Fn(&str) -> Option<String>) -> Embeds {
        Embeds {
            entries: markdown_ext::embeds(text)
                .into_iter()
                .map(|name| {
                    let body = resolve(&name);
                    (name, body)
                })
                .collect(),
        }
    }

    /// The body a name resolved to, compared the way the store compares names.
    pub(crate) fn body(&self, name: &str) -> Option<String> {
        let wanted = folded(name);
        self.entries
            .iter()
            .find(|(own, _)| folded(own) == wanted)
            .and_then(|(_, body)| body.clone())
    }

    /// A key for the preview's memo: the same names with changed bodies are a different drawing.
    pub(crate) fn digest(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        for (name, body) in &self.entries {
            name.hash(&mut hasher);
            body.hash(&mut hasher);
        }
        hasher.finish()
    }
}

/// The text to hand the renderer: every embed written as the fence it is drawn from.
pub(crate) fn expanded(text: &str, embeds: &Embeds) -> String {
    markdown_ext::expand(text, |name| embeds.body(name))
}

/// Replace a fenced block with what it draws, and leave every other fence alone.
pub(crate) fn frame(card: El, frame: &CodeFrame<'_>, embeds: &Rc<Embeds>) -> El {
    let name = frame.source.trim();
    match frame.language {
        Some(markdown_ext::INFO) => match embeds.body(name) {
            Some(body) => inset(frame.index, name, &body, Rc::clone(embeds)),
            // The target went away between the pass and this draw.
            None => missing(frame.index, name),
        },
        Some(markdown_ext::MISSING) => missing(frame.index, name),
        _ => card,
    }
}

/// One embed: the source card's text, then a caption naming the card it came from.
fn inset(index: usize, name: &str, body: &str, embeds: Rc<Embeds>) -> El {
    let t = tok::theme(true);
    let drawn = expanded(body, &embeds);
    let nested = Rc::clone(&embeds);
    col()
        .key(format!("embed-{index}"))
        .w_full()
        .gap(6.0)
        .padding(12.0)
        .radius(6.0)
        .fill(t.bg)
        .border(1.0, t.card_border)
        .child(
            Markdown::new(drawn)
                .code_frame(move |card, frame| nested_frame(card, frame, &nested))
                .render(&MarkdownStyle::neutral(true)),
        )
        .child(
            text(name.to_string())
                .key(format!("embed-caption-{index}"))
                .font(type_::LABEL)
                .color(t.text_secondary),
        )
}

/// One level is the whole depth: an embed inside an embedded card says what it names and stops.
fn nested_frame(card: El, frame: &CodeFrame<'_>, embeds: &Rc<Embeds>) -> El {
    let embed =
        frame.language == Some(markdown_ext::INFO) || frame.language == Some(markdown_ext::MISSING);
    if !embed {
        return card;
    }
    let name = frame.source.trim();
    let missing = frame.language == Some(markdown_ext::MISSING) || embeds.body(name).is_none();
    marker(name, missing)
}

/// The name drawn where an embed was nested, so a reader sees a card was meant and not its text.
fn marker(name: &str, missing: bool) -> El {
    let t = tok::theme(true);
    text(format!("{name} (embedded card)"))
        .font(type_::LABEL)
        .color(if missing {
            KIT.c.danger.fg
        } else {
            t.text_secondary
        })
}

/// An embed that names a card the library does not hold: still named, and said to be missing.
fn missing(index: usize, name: &str) -> El {
    let t = tok::theme(true);
    col()
        .key(format!("embed-{index}"))
        .w_full()
        .gap(4.0)
        .padding(12.0)
        .radius(6.0)
        .fill(t.bg)
        .border(1.0, KIT.c.danger.fg.alpha(0.5))
        .child(
            text(name.to_string())
                .key(format!("embed-missing-{index}"))
                .font(type_::BODY)
                .color(KIT.c.danger.fg),
        )
        .child(
            text(NO_SUCH_CARD)
                .key(format!("embed-caption-{index}"))
                .font(type_::LABEL)
                .color(KIT.c.danger.fg.alpha(0.75)),
        )
}
