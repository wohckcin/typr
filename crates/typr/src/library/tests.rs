use std::sync::atomic::Ordering;

use super::{GAP, Library, WRAP_BELOW, columns_for, deck_tree, tile_width, view};
use typr_core::store::{CardMode, Deck};

#[test]
fn two_tiles_fit_at_482_points() {
    assert_eq!(columns_for(482.0), 2, "two shares of 233 and the gap");
    assert_eq!(columns_for(767.0), 3);
    assert_eq!(columns_for(1485.0), 6);
    assert_eq!(
        columns_for(WRAP_BELOW * 2.0 + GAP),
        2,
        "the narrowest two-column grid is one tile over the wrap width"
    );
    assert!(
        (tile_width(767.0, 3) - 245.0).abs() < 0.5,
        "three shares of 767 are 245 each, got {}",
        tile_width(767.0, 3)
    );
    assert!(
        (tile_width(482.0, 2) - 233.0).abs() < 0.5,
        "two shares of 482 are 233 each, got {}",
        tile_width(482.0, 2)
    );
}

#[test]
fn one_tile_fits_when_nothing_else_does() {
    assert_eq!(columns_for(0.0), 1, "an unmeasured grid still draws a tile");
    assert_eq!(
        columns_for(200.0),
        1,
        "narrower than one share is still one"
    );
    assert_eq!(columns_for(WRAP_BELOW), 1, "exactly one share is one");
    assert!(
        (tile_width(200.0, 1) - 200.0).abs() < 0.5,
        "one column takes the whole width"
    );
}

#[test]
fn a_long_deck_builds_only_the_tile_rows_on_screen() {
    let mut runtime = rune::Runtime::new(
        Library::seeded(5000),
        |s: &Library, _: &mut rune::Cx<Library>| view::<Library>(s),
        (960.0, 640.0),
        rune::Env::new(true, 1.0),
    );
    // The width arrives one frame after the first layout, and the counter accumulates every
    // commit, so take the largest single frame's build.
    let mut busiest = 0;
    for _ in 0..6 {
        let before = runtime.state.built_rows.load(Ordering::Relaxed);
        runtime.frame(0.1);
        let built = runtime.state.built_rows.load(Ordering::Relaxed) - before;
        busiest = busiest.max(built);
    }

    let width = runtime.state.grid_width;
    assert_eq!(width, 760.0, "the grid is the window less the sidebar");
    let columns = columns_for(width);
    // Six lines of 116 points fit in a 640-point viewport.
    let on_screen = 6;
    assert_eq!(
        runtime.state.visible.len(),
        5000,
        "the filter keeps every card"
    );
    assert!(
        busiest <= on_screen * 4,
        "built {busiest} lines in one frame for a {on_screen}-line window at {columns} columns"
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
        reverse: None,
        template: None,
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
        reverse: None,
        template: None,
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

#[test]
fn the_grid_is_measured_from_the_window_not_from_its_own_tiles() {
    let mut runtime = rune::Runtime::new(
        Library::seeded(9),
        |s: &Library, _: &mut rune::Cx<Library>| view::<Library>(s),
        (960.0, 640.0),
        rune::Env::new(true, 1.0),
    );
    for _ in 0..3 {
        runtime.frame(0.1);
    }
    assert_eq!(
        runtime.state.grid_width, 760.0,
        "the window less the sidebar, got {}",
        runtime.state.grid_width
    );
    let grid = columns_for(runtime.state.grid_width);
    assert_eq!(grid, 3, "three columns of 242.67");
    let tile = runtime
        .elements()
        .into_iter()
        .find(|element| element.key.as_deref() == Some("tile-1"))
        .expect("the first tile");
    assert!(
        (tile.rect[2] - 242.667).abs() < 1.0,
        "the tile is 242.67 wide, got {}",
        tile.rect[2]
    );
}
#[test]
fn an_empty_deck_offers_a_tile_to_fill() {
    let library = Library::seeded(0);
    let mut runtime = rune::Runtime::new(
        library,
        |s: &Library, _: &mut rune::Cx<Library>| view::<Library>(s),
        (960.0, 640.0),
        rune::Env::new(true, 1.0),
    );
    for _ in 0..3 {
        runtime.frame(0.1);
    }
    let placeholders: Vec<_> = runtime
        .elements()
        .into_iter()
        .filter(|element| element.key.as_deref() == Some("new-card"))
        .collect();
    assert_eq!(
        placeholders.len(),
        1,
        "an empty deck draws exactly one tile that starts a card"
    );
    let rect = placeholders[0].rect;
    assert!(
        (rect[2] - 242.667).abs() < 1.0,
        "the placeholder is one grid share wide, got {rect:?}"
    );
    assert!(
        (rect[3] - 94.0).abs() < 1.0,
        "the placeholder is as tall as a tile, got {rect:?}"
    );
}

#[test]
fn the_sidebar_edits_a_deck_template_with_a_preview() {
    let mut library = Library::seeded_nested();
    library.select_deck(Some(1));
    let mut runtime = rune::Runtime::new(
        library,
        |s: &Library, _: &mut rune::Cx<Library>| view::<Library>(s),
        (960.0, 640.0),
        rune::Env::new(true, 1.0),
    );
    runtime.frame(0.0);

    let (x, y) = runtime.find("edit-template").expect("the template button");
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.1);

    assert_eq!(runtime.state.templating, Some(1), "the field is open");
    assert!(runtime.find("template-field").is_some());
    assert!(runtime.find("template-save").is_some());
    assert!(runtime.find("template-clear").is_some());

    for letter in ["<", "<", "W", "o", "r", "d", ">", ">"] {
        runtime.key(rune::KeyEvent {
            key: rune::Key::Text(letter.to_string()),
            mods: rune::Mods::default(),
        });
        runtime.frame(0.1);
    }
    assert_eq!(runtime.state.template_text().text, "<<Word>>");
    assert!(
        runtime.find("template-preview").is_some(),
        "one of the deck's cards is rendered beside the field"
    );
}

#[test]
fn an_emptied_template_field_clears_the_deck() {
    let mut library = Library::seeded_nested();
    let mut deck = library.snapshot().decks[0].clone();
    deck.template = Some("<<Word>>".to_string());
    std::sync::Arc::make_mut(&mut library.snapshot).decks[0] = deck;

    library.start_template(1);
    library.template_field().replace_text("   ".to_string());
    assert_eq!(
        library.take_template(),
        Some((1, None)),
        "whitespace is a cleared template, not a template of nothing"
    );
    assert_eq!(library.templating, None, "taking it closes the field");
}

#[test]
fn the_sidebar_lists_saved_views_and_selects_one() {
    let mut library = Library::seeded_nested();
    let spec = typr_core::store::ViewSpec {
        deck: Some(2),
        query: "verb".to_string(),
        ..typr_core::store::ViewSpec::default()
    };
    std::sync::Arc::make_mut(&mut library.snapshot)
        .views
        .push(typr_core::store::SavedView {
            id: 9,
            name: "Verb drills".to_string(),
            spec,
        });
    let mut runtime = rune::Runtime::new(
        library,
        |s: &Library, _: &mut rune::Cx<Library>| view::<Library>(s),
        (960.0, 640.0),
        rune::Env::new(true, 1.0),
    );
    runtime.frame(0.0);

    let row = runtime.find("view-9").expect("the view's row");
    runtime.pointer_down(row.0, row.1);
    runtime.pointer_up(row.0, row.1);
    runtime.frame(0.1);

    assert_eq!(runtime.state.selected_view, Some(9), "the view is selected");
    assert_eq!(runtime.state.selected_deck, None, "and no deck is");
    assert_eq!(runtime.state.query.text, "verb", "its search is applied");
    assert!(
        runtime.find("rename-view").is_some() && runtime.find("delete-view").is_some(),
        "a selected view can be renamed and deleted"
    );
}

#[test]
fn the_sort_control_offers_every_order_and_the_shuffle_changes_none() {
    let library = Library::seeded(10);
    let mut runtime = rune::Runtime::new(
        library,
        |s: &Library, _: &mut rune::Cx<Library>| view::<Library>(s),
        (1200.0, 700.0),
        rune::Env::new(true, 1.0),
    );
    runtime.frame(0.0);

    for key in [
        "sort-edited",
        "sort-created",
        "sort-first-side",
        "sort-due",
        "sort-reviews",
        "sort-shuffle",
    ] {
        assert!(runtime.find(key).is_some(), "{key} is missing");
    }

    let press = |runtime: &mut rune::Runtime<Library>, key: &str| {
        let (x, y) = runtime.find(key).unwrap_or_else(|| panic!("{key}"));
        runtime.pointer_down(x, y);
        runtime.pointer_up(x, y);
        runtime.frame(0.1);
    };

    press(&mut runtime, "sort-due");
    assert_eq!(runtime.state.sort, typr_core::store::Sort::DueDate);
    assert!(
        !runtime.state.shuffled(),
        "choosing an order ends a shuffle"
    );

    let order: Vec<i64> = runtime.state.cards().iter().map(|card| card.id).collect();
    press(&mut runtime, "sort-shuffle");
    assert!(runtime.state.shuffled(), "the shuffle is in force");
    assert_eq!(
        runtime.state.sort,
        typr_core::store::Sort::DueDate,
        "and it leaves the view's order alone"
    );
    assert_eq!(runtime.state.shuffle, Some(1), "the first hand");
    assert_ne!(
        runtime
            .state
            .cards()
            .iter()
            .map(|card| card.id)
            .collect::<Vec<_>>(),
        order,
        "the shuffle dealt another order"
    );

    press(&mut runtime, "sort-shuffle");
    assert_eq!(
        runtime.state.shuffle,
        Some(2),
        "pressing it again deals again"
    );
    assert!(
        runtime.state.snapshot().views.is_empty(),
        "no shuffle was written anywhere"
    );
}
