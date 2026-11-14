//! What a name resolves to, one level down.
//!
//! The drawing itself is proven where it can be seen (`crates/typr/src/tests/embeds.rs`); what is
//! proven here is the resolution behind it, because a name answered as missing is drawn in the
//! same tone a card that really is missing gets, and no colour can be read back out of the runtime.

use super::*;

/// A library standing in for the store: the name each card is called, and its body.
fn library<'a>(cards: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
    move |name| {
        cards
            .iter()
            .find(|(own, _)| *own == name)
            .map(|(_, body)| body.to_string())
    }
}

/// B is drawn, so the name B's body embeds is asked for as well — answering it keeps a card that
/// exists from being painted as one that does not.
#[test]
fn a_name_inside_an_embedded_body_is_answered() {
    let resolve = library(&[
        ("A", "![[B]]"),
        ("B", "B body text\n\n![[C]]"),
        ("C", "C body"),
    ]);
    let embeds = Embeds::of("Start:\n\n![[B]]", resolve);

    assert_eq!(
        embeds.body("B"),
        Some("B body text\n\n![[C]]".to_string()),
        "the card the text names was not resolved"
    );
    assert_eq!(
        embeds.body("C"),
        Some("C body".to_string()),
        "a name reached only through an embedded body was left unanswered"
    );
}

/// The names a body embeds are added, not followed: one level is what is drawn, so nothing past it
/// is asked for.
#[test]
fn the_chain_is_not_followed_past_one_level() {
    let resolve = library(&[("B", "![[C]]"), ("C", "![[D]]"), ("D", "D body")]);
    let embeds = Embeds::of("![[B]]", resolve);

    assert_eq!(embeds.body("C"), Some("![[D]]".to_string()));
    assert_eq!(embeds.body("D"), None, "the chain went two levels down");
}

/// A body that embeds itself adds no second entry, so the digest does not depend on how deep the
/// same name was met.
#[test]
fn a_name_met_twice_is_answered_once() {
    let embeds = Embeds::of("![[A]]", library(&[("A", "![[A]]\n\nA body")]));

    assert_eq!(embeds.body("A"), Some("![[A]]\n\nA body".to_string()));
}

/// A nested name nobody holds stays unanswered, which is what draws the danger tone.
#[test]
fn a_name_nobody_holds_stays_unanswered() {
    let embeds = Embeds::of("![[B]]", library(&[("B", "B body\n\n![[Gone]]")]));

    assert_eq!(embeds.body("Gone"), None);
    assert_ne!(embeds.digest(), 0, "the text embedded nothing at all");
}
