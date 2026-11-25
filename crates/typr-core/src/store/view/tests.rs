//! The orders a list can be shown in, and the filter a view saves.

use super::*;
use crate::library_view::ViewMode;
use crate::model::{CardMode, Deck, Id, Snapshot};

/// A library of `count` cards in one deck, each told apart by its preview only.
fn library(count: i64) -> Snapshot {
    let deck = Deck {
        id: 1,
        parent: None,
        name: "Biology".to_string(),
        archived: false,
        reverse: None,
        template: None,
        view: ViewMode::Grid,
    };
    Snapshot {
        decks: vec![deck],
        cards: (1..=count)
            .map(|id| CardSummary {
                id,
                deck: 1,
                preview: format!("Card {id}"),
                updated_at: id,
                ..CardSummary::default()
            })
            .collect(),
        ..Snapshot::default()
    }
}

fn ids(cards: &[&CardSummary]) -> Vec<Id> {
    cards.iter().map(|card| card.id).collect()
}

#[test]
fn each_order_runs_both_ways() {
    let mut snapshot = library(3);
    snapshot.cards[0].created_at = 30;
    snapshot.cards[1].created_at = 10;
    snapshot.cards[2].created_at = 20;
    snapshot.cards[0].reviews = 5;
    snapshot.cards[1].reviews = 1;
    snapshot.cards[2].reviews = 9;
    snapshot.cards[0].preview = "Zebra".to_string();
    snapshot.cards[1].preview = "apple".to_string();
    snapshot.cards[2].preview = "Mango".to_string();

    let all = |sort: Sort, dir: SortDir| ids(&sort.order(snapshot.cards.iter().collect(), dir));

    assert_eq!(all(Sort::LastEdited, SortDir::Ascending), vec![1, 2, 3]);
    assert_eq!(all(Sort::LastEdited, SortDir::Descending), vec![3, 2, 1]);
    assert_eq!(all(Sort::Created, SortDir::Ascending), vec![2, 3, 1]);
    assert_eq!(all(Sort::Created, SortDir::Descending), vec![1, 3, 2]);
    assert_eq!(all(Sort::Reviews, SortDir::Ascending), vec![2, 1, 3]);
    assert_eq!(all(Sort::Reviews, SortDir::Descending), vec![3, 1, 2]);
    // Folded, so "apple" sorts before "Mango" whichever case it was written in.
    assert_eq!(all(Sort::FirstSide, SortDir::Ascending), vec![2, 3, 1]);
    assert_eq!(all(Sort::FirstSide, SortDir::Descending), vec![1, 3, 2]);
}

#[test]
fn a_card_with_nothing_scheduled_comes_last_either_way() {
    let mut snapshot = library(4);
    snapshot.cards[0].due_at = None;
    snapshot.cards[1].due_at = Some(300);
    snapshot.cards[2].due_at = None;
    snapshot.cards[3].due_at = Some(100);

    let soonest = ids(&Sort::DueDate.order(snapshot.cards.iter().collect(), SortDir::Ascending));
    assert_eq!(soonest, vec![4, 2, 1, 3], "earliest first, undated last");

    let latest = ids(&Sort::DueDate.order(snapshot.cards.iter().collect(), SortDir::Descending));
    assert_eq!(latest, vec![2, 4, 1, 3], "latest first, undated still last");
}

#[test]
fn a_shuffle_only_reorders() {
    let snapshot = library(20);
    let cards: Vec<&CardSummary> = snapshot.cards.iter().collect();
    let mixed = shuffled(cards.clone(), 42);
    let mut before: Vec<Id> = ids(&cards);
    let mut after: Vec<Id> = ids(&mixed);
    assert_ne!(
        before, after,
        "a seed that leaves the list alone proves nothing"
    );
    before.sort_unstable();
    after.sort_unstable();
    assert_eq!(before, after, "a shuffle moves cards, it does not drop any");
    assert_eq!(
        ids(&super::shuffled(cards.clone(), 42)),
        ids(&super::shuffled(cards.clone(), 42)),
        "the same seed gives the same order, so a redraw does not jump"
    );
}

#[test]
fn a_spec_keeps_the_cards_it_asks_for() {
    let mut snapshot = library(4);
    snapshot.cards[0].tags = vec!["exam".to_string(), "biology".to_string()];
    snapshot.cards[1].tags = vec!["exam".to_string()];
    snapshot.cards[2].pinned = true;
    snapshot.cards[3].archived = true;

    let spec = ViewSpec {
        tags: vec!["exam".to_string(), "biology".to_string()],
        ..ViewSpec::default()
    };
    assert_eq!(
        ids(&snapshot.cards_matching(&spec)),
        vec![1],
        "every tag must match"
    );

    let pinned = ViewSpec {
        pinned_only: true,
        ..ViewSpec::default()
    };
    assert_eq!(ids(&snapshot.cards_matching(&pinned)), vec![3]);

    let archived = ViewSpec {
        mode: CardMode::Archived,
        ..ViewSpec::default()
    };
    assert_eq!(ids(&snapshot.cards_matching(&archived)), vec![4]);
}

#[test]
fn a_scope_no_deck_carries_falls_back_to_every_card() {
    let snapshot = library(2);
    let missing = ViewSpec {
        deck: Some(99),
        ..ViewSpec::default()
    };
    assert_eq!(
        ids(&snapshot.cards_matching(&missing)),
        vec![1, 2],
        "a view whose deck was deleted shows the library, not an empty list"
    );
    let view = SavedView {
        id: 1,
        name: "Gone".to_string(),
        spec: missing,
    };
    assert!(view.scope_missing(&snapshot));
    assert!(
        !SavedView {
            spec: ViewSpec::default(),
            ..view
        }
        .scope_missing(&snapshot)
    );
}

#[test]
fn every_order_survives_a_round_trip() {
    for sort in Sort::ALL {
        assert_eq!(Sort::parse(sort.as_str()), Some(sort));
    }
    assert_eq!(
        Sort::parse("shuffled"),
        None,
        "a shuffle is not a saved sort"
    );
    for dir in [SortDir::Ascending, SortDir::Descending] {
        assert_eq!(SortDir::parse(dir.as_str()), Some(dir));
    }
    assert_eq!(SortDir::Ascending.flipped(), SortDir::Descending);
}
