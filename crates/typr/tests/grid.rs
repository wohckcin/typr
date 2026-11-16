//! The card grid: tiles show their first side and hidden sides, an empty deck offers a card, and a
//! long deck scrolls to rows that show their own cards, not the ones the cells held before.

use nsh_core::prelude::*;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;
use typr::screens::grid::{Tile, grid};

fn tiles(count: i64) -> Vec<Tile> {
    (0..count)
        .map(|id| Tile {
            id,
            preview: format!("card {id}").into(),
            hidden_sides: if id == 0 { 2 } else { 0 },
            tags: if id == 0 { vec!["verbs".into(), "a".into(), "b".into(), "c".into()] } else { Vec::new() },
            schedule: typr::screens::grid::Schedule::New,
            pinned: false,
        })
        .collect()
}

#[component]
fn Probe() -> impl IntoElement {
    typr::ui::tokens::install();
    let data = signal(tiles(200));
    let memo = computed(move || data.get());
    view().flex_1().child(grid(memo, signal(0), || {}, |_| {}))
}

#[component]
fn Empty() -> impl IntoElement {
    typr::ui::tokens::install();
    let data = signal(Vec::<Tile>::new());
    let memo = computed(move || data.get());
    view().flex_1().child(grid(memo, signal(0), || {}, |_| {}))
}

#[test]
fn a_tile_shows_its_first_side_its_hidden_sides_and_its_tags() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
    app.wait_for("card 0")?;
    app.wait_for("2 hidden sides")?;
    app.wait_for("verbs")?;
    app.wait_for("New")?;
    Ok(())
}

#[test]
fn an_empty_deck_offers_a_new_card() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Empty>())).unwrap();
    app.wait_for("This deck has no cards yet")?;
    app.wait_for("role:button[New card]")?;
    Ok(())
}

#[test]
fn a_recycled_cell_shows_the_row_it_is_given() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
    app.wait_for("card 0")?;
    app.scroll("card 0", px(20000.0))?;
    app.wait_for("card 199")?;
    assert!(!app.screen().shows("card 0"), "the first row is still on screen after scrolling away");
    Ok(())
}

#[test]
fn the_column_count_follows_the_window_width_and_never_drops_to_none() {
    use typr::ui::tokens::grid_columns;
    assert_eq!(grid_columns(0.0), 1);
    assert_eq!(grid_columns(260.0), 1);
    assert!(grid_columns(1200.0) > grid_columns(600.0));
    assert!(grid_columns(600.0) >= grid_columns(300.0));
}

#[test]
fn a_new_card_is_new_and_a_studied_card_states_its_interval() {
    use typr::screens::grid::{Schedule, schedule_of};
    let day = 86_400_000;
    assert_eq!(schedule_of(0, None, 1000), Schedule::New);
    assert_eq!(schedule_of(1, Some(1000 + 10 * day), 1000), Schedule::Interval(10));
    assert_eq!(schedule_of(1, Some(500), 1000), Schedule::Due);
}

#[test]
fn a_tile_never_shows_a_hidden_answer() {
    use typr::screens::grid::first_side;
    let shown = first_side("The {{hormone}} is made by the {{pancreas}}.");
    assert!(!shown.contains("hormone"), "{shown}");
    assert!(!shown.contains("pancreas"), "{shown}");
    assert!(shown.contains("…"), "{shown}");
}

#[test]
fn the_empty_deck_shows_the_binding_that_starts_a_card() {
    let chord = typr_core::keymap::chord_of(typr_core::keymap::Surface::Library, typr_core::keymap::Command::NewCard);
    assert!(chord.is_some(), "the library binds no key to a new card");
}

#[test]
fn tags_past_the_tile_width_are_counted_not_drawn() {
    use typr::ui::tokens::tags_that_fit;
    assert_eq!(tags_that_fit(10_000.0, &["a", "b", "c"]), 3);
    assert_eq!(tags_that_fit(40.0, &["verbs", "a"]), 0);
    let long = ["spanish-verbs", "irregular", "x"];
    let fit = tags_that_fit(200.0, &long);
    assert!(fit >= 1 && fit < 3, "{fit}");
}
