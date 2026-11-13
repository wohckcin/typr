use std::sync::atomic::Ordering;

use super::{Library, ROW_HEIGHT, view};
use crate::store::CardMode;

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
    let on_screen = (640.0 / ROW_HEIGHT).ceil() as usize;
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
    let second = crate::store::Deck {
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
