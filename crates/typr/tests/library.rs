//! The library screen: the selected deck's cards, the search over them, and pinned cards first.

use nsh_core::prelude::*;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;
use typr::screens::library::{cards_in, tiles_for};
use typr_core::library_view::ViewMode;
use typr_core::store::{CardSummary, Deck, Snapshot};

fn card(id: i64, deck: i64, preview: &str, pinned: bool, position: i64) -> CardSummary {
    CardSummary {
        id,
        deck,
        preview: preview.to_owned(),
        markdown: preview.to_owned(),
        pinned,
        position,
        ..CardSummary::default()
    }
}

fn fixture() -> Snapshot {
    let deck = |id, name: &str| Deck {
        id,
        parent: None,
        name: name.to_owned(),
        archived: false,
        reverse: None,
        template: None,
        view: ViewMode::default(),
    };
    Snapshot {
        decks: vec![deck(1, "Spanish"), deck(2, "Biology")],
        cards: vec![
            card(10, 1, "hola", false, 0),
            card(11, 2, "cells", false, 0),
            card(12, 1, "adios", true, 1),
        ],
        ..Snapshot::default()
    }
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
fn every_live_card_is_shown_by_default() -> Result<(), String> {
    let mut app = start();
    app.wait_for("hola")?;
    assert!(app.screen().shows("cells"));
    Ok(())
}

#[test]
fn choosing_a_deck_shows_only_its_cards() -> Result<(), String> {
    let mut app = start();
    app.wait_for("cells")?;
    app.press("Biology")?;
    app.wait_for_gone("hola", std::time::Duration::from_secs(2))?;
    assert!(app.screen().shows("cells"));
    Ok(())
}

#[test]
fn search_narrows_the_grid_to_matching_cards() -> Result<(), String> {
    let mut app = start();
    app.wait_for("hola")?;
    app.type_text("text:Search cards", "cells")?;
    app.wait_for_gone("hola", std::time::Duration::from_secs(2))?;
    assert!(app.screen().shows("cells"));
    Ok(())
}

#[test]
fn pinned_cards_come_first_and_archived_cards_are_not_listed() {
    let mut archived = card(13, 1, "hidden", false, 0);
    archived.archived = true;
    let mut cards = fixture().cards;
    cards.push(archived);
    let tiles = tiles_for(
        &cards_in(
            &Snapshot {
                cards: cards.clone(),
                ..Snapshot::default()
            },
            Some("deck:1"),
        ),
        "",
        0,
    );
    let order: Vec<&str> = tiles.iter().map(|t| t.preview.as_ref()).collect();
    assert_eq!(order, vec!["adios", "hola"]);
}
