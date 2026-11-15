use super::*;
use rune::Env;

/// The heights and offsets of the three rows drawn below, keyed by row.
fn drawn() -> Vec<(String, [f32; 4], [f32; 4])> {
    let mut runtime = rune::Runtime::new(
        (),
        |_: &(), _: &mut rune::Cx<()>| {
            col()
                .px(INSET)
                .w(WIDTH)
                .child(row::<()>(
                    &Row::new("row-all", "All cards"),
                    |_: &mut ()| {},
                ))
                .child(row::<()>(
                    &Row::new("row-spanish", "Spanish").selected(),
                    |_: &mut ()| {},
                ))
                .child(row::<()>(
                    &Row::new("row-import", "Import").disabled(),
                    |_: &mut ()| {},
                ))
        },
        (WIDTH + 40.0, 200.0),
        Env::new(true, 1.0),
    );
    runtime.frame(0.0);
    runtime.frame(0.0);
    let elements = runtime.elements();
    let mut rows = Vec::new();
    for name in ["row-all", "row-spanish", "row-import"] {
        let row = elements
            .iter()
            .find(|element| element.key.as_deref() == Some(name));
        let wanted = format!("{name}-label");
        let label = elements
            .iter()
            .find(|element| element.key.as_deref() == Some(wanted.as_str()));
        match (row, label) {
            (Some(row), Some(label)) => rows.push((name.to_string(), row.rect, label.rect)),
            _ => panic!("{name} was not drawn"),
        }
    }
    rows
}

#[test]
fn rows_share_one_label_offset_and_height() {
    for (name, row, label) in drawn() {
        assert!(
            (row[3] - ROW_HEIGHT).abs() < 0.5,
            "{name} is {} points tall",
            row[3]
        );
        assert!(
            (label[0] - row[0] - LABEL_X).abs() < 0.5,
            "{name} starts its label {} points in, not {LABEL_X}",
            label[0] - row[0]
        );
    }
}

#[test]
fn every_row_starts_at_the_same_inset() {
    let offsets: Vec<f32> = drawn().iter().map(|(_, row, _)| row[0]).collect();
    assert!(
        offsets
            .windows(2)
            .all(|pair| (pair[0] - pair[1]).abs() < 0.5),
        "rows start at {offsets:?}"
    );
}

#[test]
fn two_decks_take_two_tints_and_one_deck_keeps_its_own() {
    assert_eq!(tint("Spanish"), tint("Spanish"));
    assert_ne!(
        tint("Spanish"),
        tint("Japanese"),
        "the tint is chosen by name, not by position"
    );
}

#[test]
fn an_icon_draws_one_letter() {
    assert_eq!(monogram(Kind::Deck, "spanish"), "S");
    assert_eq!(monogram(Kind::All, "All cards"), "A");
    assert_eq!(monogram(Kind::Tag, "animals"), "#");
    assert_eq!(monogram(Kind::Tag, "#animals"), "#");
    assert_eq!(monogram(Kind::Deck, "  "), "?");
}

#[test]
fn the_icon_slot_is_the_same_width_whatever_the_letter() {
    let mut runtime = rune::Runtime::new(
        Row::new("row-deck", "Spanish"),
        |item: &Row, _: &mut rune::Cx<Row>| {
            col()
                .px(INSET)
                .w(WIDTH)
                .child(row::<Row>(item, |_: &mut Row| {}))
        },
        (WIDTH + 40.0, 120.0),
        Env::new(true, 1.0),
    );
    runtime.frame(0.0);
    runtime.frame(0.0);
    let elements = runtime.elements();
    let icon = elements
        .iter()
        .find(|element| element.key.as_deref() == Some("icon-Spanish"))
        .expect("the icon");
    assert!((icon.rect[2] - ICON).abs() < 0.5, "the icon is {icon:?}");
    assert!((icon.rect[3] - ICON).abs() < 0.5, "the icon is {icon:?}");
}

/// Counts its presses, so a row that should not answer can be told apart from one that does.
#[derive(Default)]
struct Counted {
    pressed: u32,
}

#[test]
fn a_disabled_row_swallows_the_press() {
    let mut runtime = rune::Runtime::new(
        Counted::default(),
        |_: &Counted, _: &mut rune::Cx<Counted>| {
            col()
                .px(INSET)
                .w(WIDTH)
                .child(row::<Counted>(
                    &Row::new("row-live", "Live"),
                    |s: &mut Counted| s.pressed += 1,
                ))
                .child(row::<Counted>(
                    &Row::new("row-off", "Off").disabled(),
                    |s: &mut Counted| s.pressed += 1,
                ))
        },
        (WIDTH + 40.0, 200.0),
        Env::new(true, 1.0),
    );
    runtime.frame(0.0);
    runtime.frame(0.0);
    let centre = |runtime: &rune::Runtime<Counted>, key: &str| {
        runtime
            .elements()
            .iter()
            .find(|element| element.key.as_deref() == Some(key))
            .map(|element| {
                (
                    element.rect[0] + element.rect[2] / 2.0,
                    element.rect[1] + element.rect[3] / 2.0,
                )
            })
            .expect("the row")
    };
    let (off_x, off_y) = centre(&runtime, "row-off");
    runtime.pointer_down(off_x, off_y);
    runtime.pointer_up(off_x, off_y);
    runtime.frame(0.0);
    assert_eq!(runtime.state.pressed, 0, "a disabled row answers nothing");
    let (live_x, live_y) = centre(&runtime, "row-live");
    runtime.pointer_down(live_x, live_y);
    runtime.pointer_up(live_x, live_y);
    runtime.frame(0.0);
    assert_eq!(runtime.state.pressed, 1, "a live row answers its press");
}

/// A deck with just enough filled in for the counts.
fn deck(id: Id, parent: Option<Id>, archived: bool) -> typr_core::store::Deck {
    typr_core::store::Deck {
        id,
        parent,
        name: format!("deck {id}"),
        archived,
        reverse: None,
        template: None,
        view: typr_core::library_view::ViewMode::Grid,
    }
}

/// A card with a due time, or without one.
fn card(id: Id, deck: Id, due_at: Option<i64>) -> typr_core::store::CardSummary {
    typr_core::store::CardSummary {
        id,
        deck,
        due_at,
        ..typr_core::store::CardSummary::default()
    }
}

#[test]
fn a_count_credits_the_deck_and_every_deck_above_it() {
    let snapshot = Snapshot {
        decks: vec![deck(1, None, false), deck(2, Some(1), false)],
        cards: vec![card(10, 2, Some(100)), card(11, 2, Some(500))],
        ..Snapshot::default()
    };
    let counts = deck_due_counts(&snapshot, 200);
    assert_eq!(counts.get(&2), Some(&1), "the card's own deck");
    assert_eq!(counts.get(&1), Some(&1), "its parent");
    assert_eq!(counts.len(), 2);
}

#[test]
fn an_archived_deck_and_a_card_in_the_trash_count_for_nothing() {
    let mut trashed = card(12, 2, Some(100));
    trashed.trashed_at = Some(1);
    let mut archived_card = card(13, 2, Some(100));
    archived_card.archived = true;
    let snapshot = Snapshot {
        decks: vec![
            deck(1, None, false),
            deck(2, Some(1), false),
            deck(3, Some(1), true),
        ],
        cards: vec![
            card(10, 2, Some(100)),
            trashed,
            archived_card,
            card(14, 3, Some(100)),
        ],
        ..Snapshot::default()
    };
    let counts = deck_due_counts(&snapshot, 200);
    assert_eq!(counts.get(&2), Some(&1));
    assert_eq!(
        counts.get(&1),
        Some(&1),
        "only the card that counts is credited"
    );
    assert_eq!(counts.get(&3), None, "an archived deck counts for nothing");
}

#[test]
fn a_card_whose_deck_is_missing_is_not_counted() {
    let snapshot = Snapshot {
        decks: vec![deck(1, None, false)],
        cards: vec![card(10, 404, Some(100))],
        ..Snapshot::default()
    };
    assert!(deck_due_counts(&snapshot, 200).is_empty());
}

#[test]
fn the_selected_row_differs_from_its_sibling_in_fill_and_in_label() {
    assert_ne!(
        row_fill(true, false),
        row_fill(false, false),
        "the selected row draws the selection surface"
    );
    assert_ne!(
        label_color(true, false),
        label_color(false, false),
        "the selected row's label is full contrast"
    );
    assert_eq!(label_color(true, false), rune::Color::WHITE);
    assert_ne!(
        row_fill(false, true),
        row_fill(false, false),
        "a move target keeps its own tint"
    );
}
