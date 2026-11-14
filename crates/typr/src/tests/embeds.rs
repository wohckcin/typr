//! What an embedding card draws: the inset in the preview and on a review face.
//!
//! The fixtures publish a snapshot directly instead of going through a store, because this is
//! about the drawing: which card a name reaches is the store's own resolution, proven where it
//! lives, and what is proven here is that the reached body is what the frame shows.

use super::*;
use typr_core::store::{CardSummary, Deck, Snapshot};

/// A deck so a card's home is in the snapshot, the way a published library always has it.
fn deck(id: Id, name: &str) -> Deck {
    Deck {
        id,
        parent: None,
        name: name.to_string(),
        archived: false,
        reverse: None,
        template: None,
    }
}

/// A card holding just what drawing reads.
fn named(card: Id, name: &str, markdown: &str) -> CardSummary {
    CardSummary {
        id: card,
        deck: 1,
        name: name.to_string(),
        markdown: markdown.to_string(),
        ..CardSummary::default()
    }
}

/// The library a test draws from: the card on screen, and the one it embeds.
fn embedding_library() -> Vec<CardSummary> {
    vec![
        named(
            10,
            "Cell cycle",
            "Look at this:\n\n![[Mitosis]]\n---\nCell division.",
        ),
        named(
            11,
            "Mitosis",
            "Mitosis is how one cell becomes two identical cells.",
        ),
    ]
}

/// A snapshot holding those cards, published as the library the screens read.
fn snapshot(cards: Vec<CardSummary>) -> std::sync::Arc<Snapshot> {
    std::sync::Arc::new(Snapshot {
        decks: vec![deck(1, "Biology")],
        cards,
        ..Snapshot::default()
    })
}

/// An editor on card 10, with the embed's target one name away.
fn editor_state(text: &str) -> State {
    let mut editor = Editor::new(Some(10), text.to_string());
    editor.settle();
    let mut state = State {
        screen: Screen::Editor,
        editor,
        ..State::default()
    };
    state.library.set_snapshot(snapshot(embedding_library()));
    state
}

/// A session whose prompt holds the text, revealed to its last face.
fn reviewing(prompt: &str) -> Session {
    let mut session = Session::new(
        Queue::Due,
        vec![Entry::sides(10, [prompt, "Cell division."], 0)],
    );
    while session.next_side().is_some() {
        session.reveal();
    }
    session
}

/// The texts a frame of the app draws, after layout has had its frames.
fn drawn_text(state: State) -> Vec<String> {
    let mut runtime = rune::Runtime::new(state, view, (900.0, 600.0), rune::Env::new(true, 1.0));
    for _ in 0..4 {
        runtime.frame(1.0 / 60.0);
    }
    runtime
        .elements()
        .into_iter()
        .filter_map(|element| element.text)
        .collect()
}

/// The preview draws the named card's text inside an inset, captioned with the card it names.
#[test]
fn an_embed_draws_as_an_inset_in_the_preview() {
    let drawn = drawn_text(editor_state(
        "Look at this:\n\n![[Mitosis]]\n---\nCell division.",
    ));
    assert!(
        drawn
            .iter()
            .any(|text| text.contains("Mitosis is how one cell becomes two")),
        "the embedded card's body is not drawn: {drawn:?}"
    );
    assert!(
        drawn.iter().any(|text| text == "Mitosis"),
        "the caption naming the source card is not drawn: {drawn:?}"
    );
    assert!(
        drawn.iter().any(|text| text.contains("Look at this:")),
        "the embedding card's own text is not drawn beside the inset: {drawn:?}"
    );
}

/// The same block draws on a review face, so a side is not the only place an embed can be read.
#[test]
fn an_embed_draws_on_a_review_face() {
    let mut state = State {
        screen: Screen::Session,
        session: reviewing("Recite:\n\n![[Mitosis]]"),
        ..State::default()
    };
    state.library.set_snapshot(snapshot(embedding_library()));
    let drawn = drawn_text(state);
    assert!(
        drawn
            .iter()
            .any(|text| text.contains("Mitosis is how one cell becomes two")),
        "the review face does not draw the embedded card's body: {drawn:?}"
    );
    assert!(
        drawn.iter().any(|text| text == "Mitosis"),
        "the review face does not caption the inset: {drawn:?}"
    );
}
