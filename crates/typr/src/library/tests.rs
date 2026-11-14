use std::sync::atomic::Ordering;

use super::{Library, ROW_HEIGHT_PX, deck_tree, view};
use typr_core::store::{CardMode, Deck};

#[test]
fn a_long_library_builds_only_the_rows_on_screen() {
    let mut runtime = rune::Runtime::new(
        Library::seeded(5000),
        |s: &Library, _: &mut rune::Cx<Library>| view::<Library>(s),
        (960.0, 640.0),
        rune::Env::new(true, 1.0),
    );
    runtime.frame(0.0);

    let built = runtime.state.built_rows.load(Ordering::Relaxed);
    let on_screen = (640usize).div_ceil(ROW_HEIGHT_PX as usize);
    assert!(built > 0, "nothing was built at all");
    assert!(
        built < on_screen * 4,
        "built {built} rows for a {on_screen}-row window of 5000"
    );
    assert_eq!(
        runtime.state.visible.len(),
        5000,
        "the filter keeps every card"
    );
}

#[test]
fn the_search_box_narrows_the_list() {
    let mut library = Library::seeded(5000);
    library.query.replace_text("card 42");
    library.refresh();
    let ids: Vec<_> = library.visible.iter().map(|card| card.id).collect();
    assert!(ids.contains(&42), "exact card found: {ids:?}");
    assert!(
        !ids.contains(&43),
        "a neighbour that does not contain the number stays out"
    );
    assert!(
        ids.len() < 500,
        "only numbers containing 42 match, got {}",
        ids.len()
    );
}

#[test]
fn a_tag_filter_and_a_deck_filter_compose() {
    let mut library = Library::seeded(10);
    let second = typr_core::store::Deck {
        id: 2,
        parent: None,
        name: "Other".to_string(),
        archived: false,
    };
    let snapshot = std::sync::Arc::make_mut(&mut library.snapshot);
    snapshot.decks.push(second);
    snapshot.cards[0].tags = vec!["Grammar".to_string()];
    snapshot.cards[1].tags = vec!["verbs".to_string()];
    snapshot.cards[2].tags = vec!["Grammar".to_string()];
    snapshot.cards[2].deck = 2;

    library.refresh();
    assert_eq!(library.visible.len(), 10);

    library.tag = Some("grammar".to_string());
    library.refresh();
    assert_eq!(
        library
            .visible
            .iter()
            .map(|card| card.id)
            .collect::<Vec<_>>(),
        vec![1, 3]
    );

    library.selected_deck = Some(2);
    library.refresh();
    assert_eq!(
        library
            .visible
            .iter()
            .map(|card| card.id)
            .collect::<Vec<_>>(),
        vec![3]
    );

    library.mode = CardMode::Trash;
    library.tag = None;
    library.selected_deck = None;
    library.refresh();
    assert!(library.visible.is_empty(), "nothing is trashed");
}

fn named(id: i64, parent: Option<i64>, name: &str) -> Deck {
    Deck {
        id,
        parent,
        name: name.to_string(),
        archived: false,
    }
}

#[test]
fn a_deck_is_drawn_under_the_deck_that_owns_it() {
    // Deliberately out of order, the way a snapshot returns them after a move.
    let decks = vec![
        named(3, Some(1), "Cell biology"),
        named(1, None, "Biology"),
        named(4, Some(3), "Mitosis"),
        named(2, None, "Chemistry"),
    ];
    let drawn: Vec<(&str, usize)> = deck_tree(&decks)
        .into_iter()
        .map(|(deck, depth)| (deck.name.as_str(), depth))
        .collect();
    assert_eq!(
        drawn,
        vec![
            ("Biology", 0),
            ("Cell biology", 1),
            ("Mitosis", 2),
            ("Chemistry", 0),
        ]
    );
}

#[test]
fn a_deck_whose_parent_is_gone_is_drawn_on_its_own() {
    let decks = vec![named(7, Some(404), "Orphan")];
    let drawn: Vec<(&str, usize)> = deck_tree(&decks)
        .into_iter()
        .map(|(deck, depth)| (deck.name.as_str(), depth))
        .collect();
    assert_eq!(drawn, vec![("Orphan", 0)]);
}

#[test]
fn a_loop_of_decks_is_still_drawn_once() {
    let decks = vec![named(1, Some(2), "One"), named(2, Some(1), "Two")];
    let drawn: Vec<(&str, usize)> = deck_tree(&decks)
        .into_iter()
        .map(|(deck, depth)| (deck.name.as_str(), depth))
        .collect();
    assert_eq!(drawn, vec![("One", 0), ("Two", 1)]);
}

#[test]
fn the_rename_field_takes_what_is_typed_straight_after_the_press() {
    let mut library = Library::seeded(1);
    library.select_deck(Some(1));
    let mut runtime = rune::Runtime::new(
        library,
        |s: &Library, _: &mut rune::Cx<Library>| view::<Library>(s),
        (960.0, 640.0),
        rune::Env::new(true, 1.0),
    );
    runtime.frame(0.0);

    let (x, y) = runtime.find("rename-deck").expect("the rename button");
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.1);
    assert_eq!(
        runtime.state.renaming,
        Some(1),
        "the press did not open the field"
    );

    runtime.paste("Zoology");
    runtime.frame(0.1);
    assert_eq!(
        runtime.state.rename_buffer.text, "Zoology",
        "typing after the press should replace the old name"
    );
}

#[test]
fn the_rename_field_keeps_each_keystroke() {
    let mut library = Library::seeded(1);
    library.select_deck(Some(1));
    let mut runtime = rune::Runtime::new(
        library,
        |s: &Library, _: &mut rune::Cx<Library>| view::<Library>(s),
        (960.0, 640.0),
        rune::Env::new(true, 1.0),
    );
    runtime.frame(0.0);

    let (x, y) = runtime.find("rename-deck").expect("the rename button");
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.1);

    for letter in ["Z", "o", "o"] {
        runtime.key(rune::KeyEvent {
            key: rune::Key::Text(letter.to_string()),
            mods: rune::Mods::default(),
        });
        runtime.frame(0.1);
    }
    assert_eq!(runtime.state.rename_buffer.text, "Zoo");
}

#[test]
fn selecting_another_deck_abandons_the_name_field() {
    let mut library = Library::seeded_nested();
    library.select_deck(Some(1));
    let mut runtime = rune::Runtime::new(
        library,
        |s: &Library, _: &mut rune::Cx<Library>| view::<Library>(s),
        (960.0, 640.0),
        rune::Env::new(true, 1.0),
    );
    runtime.frame(0.0);

    let (x, y) = runtime.find("rename-deck").expect("the rename button");
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.1);
    assert_eq!(runtime.state.renaming, Some(1));

    let (x, y) = runtime.find("deck-2").expect("the second deck");
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.1);
    assert_eq!(runtime.state.renaming, None, "the field stayed open");
    assert_eq!(runtime.state.snapshot.decks[0].name, "Biology");
}
