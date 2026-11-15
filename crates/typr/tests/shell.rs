//! The shell over a library snapshot: the sidebar's rows come from the snapshot, and a library
//! that did not open is said so.

use nsh_core::prelude::*;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;
use typr_core::library_view::ViewMode;
use typr_core::store::{CardSummary, Deck, Snapshot};

fn deck(id: i64, parent: Option<i64>, name: &str) -> Deck {
    Deck {
        id,
        parent,
        name: name.to_owned(),
        archived: false,
        reverse: None,
        template: None,
        view: ViewMode::default(),
    }
}

fn fixture() -> Snapshot {
    let spanish = deck(1, None, "Spanish");
    let verbs = deck(2, Some(1), "Verbs");
    let biology = deck(3, None, "Biology");
    let due_card = CardSummary { id: 10, deck: 2, due_at: Some(100), ..CardSummary::default() };
    Snapshot { decks: vec![spanish, verbs, biology], cards: vec![due_card], ..Snapshot::default() }
}

#[component]
fn Probe() -> impl IntoElement {
    typr::ui::tokens::install();
    typr::shell::shell(fixture(), true)
}

fn start() -> Headless {
    Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap()
}

#[test]
fn the_decks_of_the_library_are_listed_with_their_due_counts() -> Result<(), String> {
    let mut app = start();
    app.wait_for("Spanish")?;
    assert!(app.screen().shows("Biology"));
    assert!(app.screen().shows("Verbs"));
    // The due card is in Verbs, so Spanish above it counts it too.
    app.wait_for("text:/^1$/")?;
    Ok(())
}

#[test]
fn an_empty_library_says_it_is_closed() -> Result<(), String> {
    #[component]
    fn Closed() -> impl IntoElement {
        typr::ui::tokens::install();
        typr::shell::shell(Snapshot::default(), false)
    }
    let mut app = Headless::start(EngineConfig::new(Root::of::<Closed>())).unwrap();
    app.wait_for("Library closed")?;
    Ok(())
}
