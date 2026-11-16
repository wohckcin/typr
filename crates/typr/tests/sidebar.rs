//! The NSH sidebar: groups in order, the rows the user can press, counts, folds and the library
//! state, driven through the screen.

use nsh_core::prelude::*;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;
use typr::screens::sidebar::{Group, Item, Sidebar, sidebar};

fn item(key: &'static str, label: &'static str, group: Group) -> Item {
    Item::new(key, label, group)
}

fn fixture() -> Vec<Item> {
    let mut spanish = item("spanish", "Spanish", Group::Decks);
    spanish.due = 1;
    spanish.has_children = true;
    let mut verbs = item("verbs", "Verbs", Group::Decks);
    verbs.parent = Some("spanish".into());
    let biology = item("biology", "Biology", Group::Decks);
    vec![
        item("all", "All cards", Group::Navigation),
        spanish,
        verbs,
        biology,
        item("settings", "Settings", Group::Utilities),
    ]
}

#[component]
fn Probe() -> impl IntoElement {
    typr::ui::tokens::install();
    let state = Sidebar::new(fixture(), false, typr::screens::sidebar::search_chord());
    view()
        .flex_row()
        .child(sidebar(state, move |key: &SharedString| state.select(key)))
        .child(text(move || format!("selected={}", state.selected_text())))
        .child(button_refresh(state))
}

fn button_refresh(state: Sidebar) -> impl IntoElement {
    typr::ui::button::button()
        .variant(typr::ui::button::Variant::Ghost)
        .label("Refresh")
        .on_press(move || state.refresh(fixture()))
}

fn start() -> Headless {
    Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap()
}

#[test]
fn groups_draw_in_order_and_an_empty_group_is_absent() -> Result<(), String> {
    let mut app = start();
    app.wait_for("Decks")?;
    let texts: Vec<String> = app.screen().texts().iter().map(|t| t.to_string()).collect();
    let at = |label: &str| texts.iter().position(|t| t == label).unwrap_or(usize::MAX);
    assert!(at("Navigation") < at("Decks"), "{texts:?}");
    assert!(at("Decks") < at("Utilities"), "{texts:?}");
    assert!(!app.screen().shows("Tags"), "{texts:?}");
    Ok(())
}

#[test]
fn pressing_a_row_selects_it() -> Result<(), String> {
    let mut app = start();
    app.wait_for("Biology")?;
    app.press("Biology")?;
    app.wait_for("text:/selected=biology/")?;
    Ok(())
}

#[test]
fn a_deck_with_work_shows_its_count_and_a_quiet_one_does_not() -> Result<(), String> {
    let mut app = start();
    app.wait_for("Spanish")?;
    assert!(app.screen().shows("1"));
    assert!(!app.screen().texts().contains(&"0"));
    Ok(())
}

#[test]
fn folding_a_deck_hides_its_children_and_keeps_its_count() -> Result<(), String> {
    let mut app = start();
    app.wait_for("Verbs")?;
    app.press("role:button[Collapse Spanish]")?;
    app.wait_for_gone("Verbs", std::time::Duration::from_secs(2))?;
    assert!(app.screen().shows("Spanish"));
    Ok(())
}

#[test]
fn a_fold_survives_a_refresh() -> Result<(), String> {
    let mut app = start();
    app.wait_for("Verbs")?;
    app.press("role:button[Collapse Spanish]")?;
    app.wait_for_gone("Verbs", std::time::Duration::from_secs(2))?;
    app.press("Refresh")?;
    app.wait_for("Spanish")?;
    assert!(!app.screen().shows("Verbs"));
    Ok(())
}

#[test]
fn the_search_row_shows_the_chord_the_keymap_binds() -> Result<(), String> {
    let chord = typr::screens::sidebar::search_chord();
    assert!(!chord.is_empty(), "the library binds no search chord");
    let mut app = start();
    app.wait_for(format!("text:{chord}").as_str())?;
    Ok(())
}

#[test]
fn a_closed_library_is_stated() -> Result<(), String> {
    let mut app = start();
    app.wait_for("Library closed")?;
    Ok(())
}

#[test]
fn the_fold_chevron_names_what_it_does_and_what_it_folds() -> Result<(), String> {
    let mut app = start();
    app.wait_for("role:button[Collapse Spanish]")?;
    app.press("role:button[Collapse Spanish]")?;
    app.wait_for("role:button[Expand Spanish]")?;
    Ok(())
}
