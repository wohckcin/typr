use std::sync::atomic::Ordering;

use super::{GAP, Host, Library, WRAP_BELOW, columns_for, deck_tree, tile_width, view};
use typr_core::keymap::Surface;
use typr_core::library_view::ViewMode;
use typr_core::store::{CardMode, Deck, Id};

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
        view: ViewMode::Grid,
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
        view: ViewMode::Grid,
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
fn the_sort_control_offers_every_order() {
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

#[test]
fn the_list_is_ordered_by_the_chosen_key() {
    use typr_core::store::{Sort, SortDir};

    let mut library = Library::seeded(3);
    {
        let snapshot = std::sync::Arc::make_mut(&mut library.snapshot);
        let edited = [3, 2, 1];
        let first = ["cabra", "abeja", "burro"];
        let due = [None, Some(10), Some(5)];
        for (index, card) in snapshot.cards.iter_mut().enumerate() {
            card.updated_at = edited[index];
            card.preview = first[index].to_string();
            card.due_at = due[index];
            card.prompts = u32::from(due[index].is_some());
        }
    }
    let ids = |library: &Library| {
        library
            .cards()
            .iter()
            .map(|card| card.id)
            .collect::<Vec<_>>()
    };

    library.refresh();
    assert_eq!(
        ids(&library),
        vec![1, 2, 3],
        "an untouched list opens newest first, because editing is the default order"
    );

    library.set_sort(Sort::FirstSide);
    assert_eq!(library.dir, SortDir::Ascending, "a name reads A first");
    assert_eq!(ids(&library), vec![2, 3, 1], "abeja, burro, cabra");

    library.set_sort(Sort::DueDate);
    assert_eq!(
        ids(&library),
        vec![3, 2, 1],
        "the soonest due first, and the card with nothing scheduled last"
    );

    library.set_sort(Sort::Reviews);
    assert_eq!(
        library.dir,
        SortDir::Descending,
        "a count reads largest first"
    );
    assert_eq!(
        ids(&library),
        vec![1, 2, 3],
        "every review count is equal, so the tie keeps the snapshot's order"
    );
}

#[test]
fn a_shuffle_is_not_saved_with_the_view() {
    let mut library = Library::seeded(8);
    let spec = typr_core::store::ViewSpec {
        sort: typr_core::store::Sort::FirstSide,
        ..typr_core::store::ViewSpec::default()
    };
    std::sync::Arc::make_mut(&mut library.snapshot)
        .views
        .push(typr_core::store::SavedView {
            id: 4,
            name: "A-Z".to_string(),
            spec,
        });
    library.select_view(4);
    let saved = library.cards().iter().map(|c| c.id).collect::<Vec<_>>();

    library.shuffle_now();
    assert!(library.shuffled());

    library.select_view(4);
    assert!(
        !library.shuffled(),
        "coming back to a view drops the shuffle"
    );
    assert_eq!(
        library.cards().iter().map(|c| c.id).collect::<Vec<_>>(),
        saved,
        "the view's own order is what the list shows"
    );
}

#[test]
fn the_pinned_only_control_filters_the_list() {
    let mut library = Library::seeded(3);
    library.pin(1);
    let mut runtime = rune::Runtime::new(
        library,
        |s: &Library, _: &mut rune::Cx<Library>| view::<Library>(s),
        (1200.0, 700.0),
        rune::Env::new(true, 1.0),
    );
    runtime.frame(0.0);
    assert_eq!(
        runtime.state.cards().len(),
        3,
        "every card before the press"
    );

    let (x, y) = runtime
        .find("pinned-only")
        .expect("the pinned-only control");
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.0);

    assert!(runtime.state.pinned_only, "the list asks for pinned cards");
    let shown: Vec<Id> = runtime.state.cards().iter().map(|card| card.id).collect();
    assert_eq!(shown, vec![1], "only the pinned card is left");

    let (x, y) = runtime
        .find("pinned-only")
        .expect("the pinned-only control");
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.0);
    assert_eq!(
        runtime.state.cards().len(),
        3,
        "pressing it again shows all"
    );
}

#[test]
fn the_views_scene_shows_a_pinned_card_under_its_view() {
    use typr_core::store::{SavedView, Sort, SortDir, ViewSpec};
    let mut library = Library::seeded_nested();
    library.seed_views(vec![SavedView {
        id: 1,
        name: "Exam week".to_string(),
        spec: ViewSpec {
            pinned_only: true,
            sort: Sort::FirstSide,
            dir: SortDir::Ascending,
            ..ViewSpec::default()
        },
    }]);
    library.pin(2);
    library.select_view(1);

    let shown: Vec<Id> = library.cards().iter().map(|card| card.id).collect();
    assert_eq!(
        shown,
        vec![2],
        "the scene's view leaves the pinned card alone"
    );
    assert!(library.pinned_only, "the pinned-only row reads as chosen");
}

/// A library of `count` cards in a deck that is read `view` ways, with the first card selected.
fn in_view(count: usize, view: ViewMode) -> Library {
    let mut library = Library::seeded(count);
    let deck = typr_core::store::Deck {
        id: 1,
        parent: None,
        name: "Seeded".to_string(),
        archived: false,
        reverse: None,
        template: None,
        view,
    };
    {
        let snapshot = std::sync::Arc::make_mut(&mut library.snapshot);
        snapshot.decks = vec![deck];
    }
    library.selected_deck = Some(1);
    library.selected_card = Some(1);
    library.refresh();
    library
}

fn draw(library: Library) -> rune::Runtime<Library> {
    let mut runtime = rune::Runtime::new(
        library,
        |s: &Library, _: &mut rune::Cx<Library>| view::<Library>(s),
        (1200.0, 800.0),
        rune::Env::new(true, 1.0),
    );
    runtime.frame(0.0);
    runtime.frame(0.0);
    runtime
}

#[test]
fn each_view_draws_the_same_cards() {
    let grid = draw(in_view(3, ViewMode::Grid));
    let list = draw(in_view(3, ViewMode::List));
    let notebook = draw(in_view(3, ViewMode::Notebook));
    let column = draw(in_view(3, ViewMode::Column));

    for id in 1..=3 {
        assert!(
            grid.find(&format!("tile-{id}")).is_some(),
            "the grid lost card {id}"
        );
        assert!(
            list.find(&format!("list-row-{id}")).is_some(),
            "the list lost card {id}"
        );
        assert!(
            notebook.find(&format!("notebook-{id}")).is_some(),
            "the notebook lost card {id}"
        );
    }
    assert!(
        column.find("column-card").is_some(),
        "the column view draws no card at all"
    );
    assert!(
        column.find("column-name").is_some(),
        "the column view does not name the card it reads"
    );
    // The column view is one card at reading width, so it cannot draw the other two, and saying so
    // is the point: it is the only view that shows less than the filter holds.
    assert!(column.find("notebook-1").is_none());
}

#[test]
fn the_notebook_reads_in_the_order_the_deck_holds() {
    let mut library = in_view(3, ViewMode::Notebook);
    {
        let snapshot = std::sync::Arc::make_mut(&mut library.snapshot);
        // Deliberately not the id order: the deck holds card 3 first.
        snapshot.cards[0].position = 2;
        snapshot.cards[1].position = 0;
        snapshot.cards[2].position = 1;
    }
    // `visible` is an owned copy of the cards, so an order changed behind `refresh` would not be
    // the order the view reads.
    library.refresh();
    let runtime = draw(library);
    let names: Vec<String> = runtime
        .elements()
        .iter()
        .filter_map(|element| element.key.clone())
        .filter(|key| {
            key.strip_prefix("notebook-")
                .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()))
        })
        .collect();
    let mut positions: Vec<(String, f32)> = names
        .into_iter()
        .filter_map(|key| {
            let rect = runtime
                .elements()
                .iter()
                .find(|each| each.key.as_deref() == Some(key.as_str()) && !each.exiting)?
                .rect;
            Some((key, rect[1]))
        })
        .collect();
    positions.sort_by(|left, right| left.1.total_cmp(&right.1));
    let order: Vec<&str> = positions.iter().map(|(key, _)| key.as_str()).collect();
    assert_eq!(
        order,
        vec!["notebook-2", "notebook-3", "notebook-1"],
        "the notebook stacks the deck's order, not the ids"
    );
}

#[test]
fn the_notebook_view_draws_a_whole_card() {
    let mut library = in_view(2, ViewMode::Notebook);
    {
        let snapshot = std::sync::Arc::make_mut(&mut library.snapshot);
        snapshot.cards[0].markdown = "gato\n---\ncat".to_string();
        snapshot.cards[0].preview = "gato".to_string();
        snapshot.cards[0].name = "gato".to_string();
        snapshot.cards[0].tags = vec!["animals".to_string()];
    }
    library.refresh();
    let runtime = draw(library);
    let keys: Vec<String> = runtime
        .elements()
        .iter()
        .filter(|each| !each.exiting)
        .filter_map(|each| each.key.clone())
        .collect();
    assert!(
        keys.iter().any(|key| key == "notebook-1"),
        "the card is drawn"
    );
    assert!(
        keys.iter().any(|key| key == "notebook-edit-1"),
        "the notebook offers a way into the editor, got {keys:?}"
    );
    let elements = runtime.elements();
    let drawn: Vec<&str> = elements
        .iter()
        .filter(|each| !each.exiting)
        .filter_map(|each| each.text.as_deref())
        .collect();
    assert!(
        drawn.iter().any(|line| line.contains("cat")),
        "the notebook shows the back of the card, drew {drawn:?}"
    );
    assert!(
        drawn.iter().any(|line| line.contains("animals")),
        "the notebook shows the card's tags"
    );
}

/// Press a keyed control and settle the frame it queued.
fn press(runtime: &mut rune::Runtime<Library>, key: &str) {
    let (x, y) = runtime.find(key).expect("the control is drawn");
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.0);
}

#[test]
fn the_column_view_finds_the_second_match() {
    let mut library = in_view(2, ViewMode::Column);
    {
        let snapshot = std::sync::Arc::make_mut(&mut library.snapshot);
        snapshot.cards[0].markdown = "gato\n\nel gato duerme\n\nun gato mas\n---\ncat".to_string();
    }
    library.refresh();
    library.selected_card = Some(1);
    library.find.replace_text("GATO");
    library.find_in_current();
    assert_eq!(
        library.find_hits,
        vec![0, 2, 4],
        "three lines hold the find"
    );
    assert_eq!(library.find_label(), "1 of 3");

    let mut runtime = draw(library);
    let keys: Vec<String> = runtime
        .elements()
        .iter()
        .filter(|each| !each.exiting)
        .filter_map(|each| each.key.clone())
        .collect();
    for wanted in [
        "column-find",
        "find",
        "find-count",
        "find-next",
        "find-previous",
    ] {
        assert!(
            keys.iter().any(|key| key == wanted),
            "the column view has no {wanted}"
        );
    }

    press(&mut runtime, "find-next");
    assert_eq!(runtime.state.find_at, 1, "the next hit is the second one");
    assert_eq!(runtime.state.find_label(), "2 of 3");
    assert_eq!(runtime.state.find_line(), Some("el gato duerme"));

    press(&mut runtime, "find-next");
    press(&mut runtime, "find-next");
    assert_eq!(runtime.state.find_at, 0, "the hits wrap");
}

#[test]
fn a_find_that_finds_nothing_says_so() {
    let mut library = in_view(1, ViewMode::Column);
    library.find.replace_text("perro");
    library.find_in_current();
    assert!(library.find_hits.is_empty());
    assert_eq!(library.find_label(), "No match");
    library.find_next();
    assert_eq!(library.find_at, 0, "a step with no hits changes nothing");

    let runtime = draw(library);
    let keys: Vec<String> = runtime
        .elements()
        .iter()
        .filter(|each| !each.exiting)
        .filter_map(|each| each.key.clone())
        .collect();
    assert!(
        keys.iter().any(|key| key == "find-none"),
        "the column view says there is no match, got {keys:?}"
    );
}

#[test]
fn switching_views_keeps_the_selection() {
    let mut library = in_view(3, ViewMode::Grid);
    library.selected_card = Some(2);
    library.refresh();
    let mut runtime = draw(library);
    assert_eq!(runtime.state.view_mode(), ViewMode::Grid);
    assert!(
        runtime.find("tile-2").is_some(),
        "the grid draws the selected card"
    );

    press(&mut runtime, "view-list");

    assert_eq!(
        runtime.state.view_mode(),
        ViewMode::List,
        "the switcher changes the deck's view"
    );
    assert_eq!(
        runtime.state.selected_card,
        Some(2),
        "the reader stays on the card they were reading"
    );
    assert!(
        runtime.find("list-row-2").is_some(),
        "the list draws the same card"
    );
    assert!(
        runtime.find("tile-2").is_none(),
        "the grid is gone, not stacked under the list"
    );
}

#[test]
fn the_whole_library_has_no_view_to_choose() {
    let mut library = in_view(2, ViewMode::Grid);
    library.selected_deck = None;
    library.refresh();
    let runtime = draw(library);
    assert!(
        runtime.find("view-grid").is_none(),
        "there is no deck to choose a view for"
    );
}

/// A key press as the frame loop sees it, so the test proves the binding as well as the move.
fn key_event(key: rune::Key) -> rune::KeyEvent {
    rune::KeyEvent {
        key,
        mods: rune::Mods::default(),
    }
}

/// The command the table binds on the library surface, and the move it makes.
fn press_on_library(library: &mut Library, key: rune::Key) -> bool {
    let Some(command) = crate::keymap::command_for(Surface::Library, &key_event(key)) else {
        return false;
    };
    match command {
        typr_core::keymap::Command::SelectionNext => {
            library.select_next();
            true
        }
        typr_core::keymap::Command::SelectionPrevious => {
            library.select_previous();
            true
        }
        _ => false,
    }
}

#[test]
fn the_selection_stops_at_the_first_and_last_card() {
    let mut library = in_view(3, ViewMode::List);
    library.selected_card = Some(1);

    press_on_library(&mut library, rune::Key::Text("k".to_string()));
    assert_eq!(
        library.selected_card,
        Some(1),
        "there is nothing above the first card"
    );

    press_on_library(&mut library, rune::Key::Down);
    press_on_library(&mut library, rune::Key::Down);
    assert_eq!(
        library.selected_card,
        Some(3),
        "down moves through the list"
    );
    press_on_library(&mut library, rune::Key::Down);
    assert_eq!(
        library.selected_card,
        Some(3),
        "there is nothing below the last card"
    );
    press_on_library(&mut library, rune::Key::Text("j".to_string()));
    assert_eq!(library.selected_card, Some(3), "`j` is the same move");

    press_on_library(&mut library, rune::Key::Up);
    assert_eq!(library.selected_card, Some(2), "up moves back");
}

#[test]
fn a_selection_with_no_card_in_it_lands_on_the_first() {
    let mut library = in_view(2, ViewMode::Grid);
    library.selected_card = None;
    press_on_library(&mut library, rune::Key::Down);
    assert_eq!(
        library.selected_card,
        Some(1),
        "the first press starts the walk"
    );
}

#[test]
fn moving_selection_scrolls_it_into_view() {
    let mut library = in_view(40, ViewMode::Notebook);
    library.selected_card = Some(1);
    library.refresh();
    let mut runtime = draw(library);
    // The app's own handler, so this proves the key, the request and the reveal together.
    runtime.on_key =
        Some(|s: &mut Library, event: &rune::KeyEvent, tx: &mut rune::Tx| super::key(s, event, tx));
    let first = rect_of(&runtime, "notebook-1").expect("the first card is drawn");
    assert!(first[1] < 800.0, "the first card starts in view");

    for _ in 0..30 {
        runtime.key(key_event(rune::Key::Down));
        runtime.frame(0.0);
    }
    assert_eq!(runtime.state.selected_card, Some(31));
    let shown = rect_of(&runtime, "notebook-31").expect("the selected card is drawn");
    assert!(
        (0.0..800.0).contains(&shown[1]),
        "the card the selection landed on is inside the window, at y {}",
        shown[1]
    );
}

#[test]
fn pressing_down_in_the_list_keeps_the_selection_on_screen() {
    let mut library = in_view(40, ViewMode::List);
    library.selected_card = Some(1);
    library.refresh();
    let mut runtime = draw(library);
    runtime.on_key =
        Some(|s: &mut Library, event: &rune::KeyEvent, tx: &mut rune::Tx| super::key(s, event, tx));

    for _ in 0..30 {
        runtime.key(key_event(rune::Key::Down));
        runtime.frame(0.0);
    }
    assert_eq!(runtime.state.selected_card, Some(31));
    // The list is drawn only where it is scrolled to: the row holding the card has to be built.
    let row = rect_of(&runtime, "line-30").expect("the row holding the selected card is drawn");
    let cards = rect_of(&runtime, "cards").expect("the list is drawn");
    assert!(
        row[1] >= cards[1] && row[1] + row[3] <= cards[1] + cards[3] + 1.0,
        "the selected row sits inside the list, at y {} in {}..{}",
        row[1],
        cards[1],
        cards[1] + cards[3]
    );
}

/// A host that records what the keys asked for, since the library itself only draws.
#[derive(Default)]
struct Recorder {
    library: Library,
    opened: Vec<Id>,
    trashed: Vec<Id>,
    made: Vec<(Id, u32)>,
}

impl Host for Recorder {
    fn library(&self) -> &Library {
        &self.library
    }

    fn library_mut(&mut self) -> &mut Library {
        &mut self.library
    }

    fn open_card(&mut self, card: Id) {
        self.opened.push(card);
    }

    fn trash_card(&mut self, card: Id) {
        self.trashed.push(card);
    }

    fn new_card_at(&mut self, deck: Id, position: u32) {
        self.made.push((deck, position));
    }
}

fn press_host(host: &mut Recorder, key: rune::Key) -> bool {
    super::key(host, &key_event(key), &mut rune::Tx::default())
}

#[test]
fn every_view_edits_previews_and_deletes() {
    for view in ViewMode::ALL {
        let mut host = Recorder {
            library: in_view(3, view),
            ..Recorder::default()
        };
        host.library.selected_card = Some(2);

        assert!(press_host(&mut host, rune::Key::Enter), "{view:?} edit");
        assert_eq!(host.opened, vec![2], "{view:?} opens the card it selected");

        assert!(
            press_host(&mut host, rune::Key::Text("p".to_string())),
            "{view:?} preview"
        );
        assert_eq!(host.library.peek, Some(2), "{view:?} previews it");
        assert!(
            press_host(&mut host, rune::Key::Text("p".to_string())),
            "{view:?} close"
        );
        assert_eq!(host.library.peek, None, "{view:?} puts the preview away");

        assert!(
            press_host(&mut host, rune::Key::Backspace),
            "{view:?} delete"
        );
        assert_eq!(host.trashed, vec![2], "{view:?} trashes it");
    }
}

#[test]
fn the_notebook_view_adds_a_card_below() {
    let mut host = Recorder {
        library: in_view(3, ViewMode::Notebook),
        ..Recorder::default()
    };
    host.library.selected_card = Some(2);
    assert!(press_host(&mut host, rune::Key::Text("o".to_string())));
    assert_eq!(
        host.made,
        vec![(1, 2)],
        "the new card belongs right after the second"
    );
}

/// The box a keyed element was laid out in, or `None` when it is not drawn.
fn rect_of(runtime: &rune::Runtime<Library>, key: &str) -> Option<[f32; 4]> {
    runtime
        .elements()
        .iter()
        .find(|each| !each.exiting && each.key.as_deref() == Some(key))
        .map(|each| each.rect)
}
