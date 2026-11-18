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
        // A new prompt, so the Inbox the app opens on holds the card.
        prompts: 1,
        unscheduled: 1,
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
            0,
        ),
        "",
        0,
    );
    let order: Vec<&str> = tiles.iter().map(|t| t.preview.as_ref()).collect();
    assert_eq!(order, vec!["adios", "hola"]);
}

const NOW: i64 = 1_000_000;

fn studyable(id: i64, deck: i64, due_at: Option<i64>, unscheduled: u32) -> CardSummary {
    CardSummary {
        id,
        deck,
        prompts: 1,
        unscheduled,
        due_at,
        ..CardSummary::default()
    }
}

fn deck_of(id: i64, parent: Option<i64>, archived: bool) -> Deck {
    Deck {
        id,
        parent,
        name: format!("d{id}"),
        archived,
        reverse: None,
        template: None,
        view: ViewMode::default(),
    }
}

fn ids(cards: Vec<CardSummary>) -> Vec<i64> {
    cards.into_iter().map(|card| card.id).collect()
}

fn inbox_fixture() -> Snapshot {
    let mut trashed = studyable(5, 1, Some(0), 0);
    trashed.trashed_at = Some(1);
    let mut note = studyable(6, 1, Some(0), 1);
    note.prompts = 0;
    let mut archived_card = studyable(7, 1, Some(0), 0);
    archived_card.archived = true;
    Snapshot {
        decks: vec![
            deck_of(1, None, false),
            deck_of(2, None, true),
            deck_of(3, Some(2), false),
        ],
        cards: vec![
            studyable(1, 1, Some(NOW - 1), 0), // due
            studyable(2, 1, None, 1),          // new
            studyable(3, 1, Some(NOW + 7), 0), // due next week
            studyable(4, 1, Some(NOW - 1), 1), // due and new
            trashed,
            note,
            archived_card,
            studyable(8, 2, Some(0), 0), // in an archived deck
            studyable(9, 3, Some(0), 1), // under an archived deck
        ],
        ..Snapshot::default()
    }
}

#[test]
fn the_inbox_holds_due_and_new_cards_once_each() {
    let snapshot = inbox_fixture();
    assert_eq!(ids(cards_in(&snapshot, Some("inbox"), NOW)), vec![1, 2, 4]);
    assert_eq!(ids(cards_in(&snapshot, Some("inbox:due"), NOW)), vec![1, 4]);
    assert_eq!(ids(cards_in(&snapshot, Some("inbox:new"), NOW)), vec![2, 4]);
}

#[test]
fn the_inbox_leaves_out_what_is_not_studyable() {
    let snapshot = inbox_fixture();
    let shown = ids(cards_in(&snapshot, Some("inbox"), NOW));
    for gone in [5, 6, 7, 8, 9] {
        assert!(!shown.contains(&gone), "card {gone} is not studyable");
    }
}
