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
            tags: if id == 0 { vec!["verbs".into()] } else { Vec::new() },
        })
        .collect()
}

#[component]
fn Probe() -> impl IntoElement {
    typr::ui::tokens::install();
    let data = signal(tiles(200));
    view().flex_1().child(grid(data, || {}))
}

#[component]
fn Empty() -> impl IntoElement {
    typr::ui::tokens::install();
    let data = signal(Vec::<Tile>::new());
    view().flex_1().child(grid(data, || {}))
}

#[test]
fn a_tile_shows_its_first_side_its_hidden_sides_and_its_tags() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
    app.wait_for("card 0")?;
    app.wait_for("2 hidden sides")?;
    app.wait_for("verbs")?;
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
