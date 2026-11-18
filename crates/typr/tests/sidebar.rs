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
        item("inbox", "Inbox", Group::Navigation),
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
    assert!(at("Inbox") < at("Decks"), "{texts:?}");
    assert!(at("Decks") < at("Settings"), "{texts:?}");
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
    // A deck's count rides in its label: the base tree gives a row no trailing slot.
    app.wait_for("Spanish 1")?;
    assert!(!app.screen().texts().contains(&"0"));
    Ok(())
}

#[test]
fn folding_a_deck_hides_its_children_and_keeps_its_count() -> Result<(), String> {
    let mut app = start();
    app.wait_for("Verbs")?;
    app.press("Spanish 1")?;
    app.press_key("left")?;
    app.wait_for_gone("Verbs", std::time::Duration::from_secs(2))?;
    assert!(app.screen().shows("Spanish 1"));
    Ok(())
}

#[test]
fn a_fold_survives_a_refresh() -> Result<(), String> {
    let mut app = start();
    app.wait_for("Verbs")?;
    app.press("Spanish 1")?;
    app.press_key("left")?;
    app.wait_for_gone("Verbs", std::time::Duration::from_secs(2))?;
    app.press("Refresh")?;
    app.wait_for("Spanish 1")?;
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
fn a_deck_folds_on_its_own_press_and_selects_itself() -> Result<(), String> {
    let mut app = start();
    app.wait_for("Verbs")?;
    // A folder folds on its own press — the base element's model — and the row it folded is the
    // row it selected.
    app.press("Spanish 1")?;
    app.wait_for_gone("Verbs", std::time::Duration::from_secs(2))?;
    app.wait_for("selected=spanish")?;
    // The same press opens it again.
    app.press("Spanish 1")?;
    app.wait_for("Verbs")?;
    // A leaf row selects and folds nothing.
    app.press("Biology")?;
    app.wait_for("selected=biology")?;
    Ok(())
}

#[test]
fn the_deck_rows_are_a_tree() -> Result<(), String> {
    let mut app = start();
    // The rows are one tree, not a list of buttons: the base element reports each row's level and
    // whether it is expanded, and the nested deck sits under its parent.
    app.wait_for("role:treeitem[Spanish 1]")?;
    app.wait_for("role:treeitem[Biology]")?;
    let texts: Vec<String> = app.screen().texts().iter().map(|t| t.to_string()).collect();
    let at = |label: &str| texts.iter().position(|t| t == label).unwrap_or(usize::MAX);
    assert!(at("Spanish 1") < at("Verbs"), "{texts:?}");
    assert!(at("Verbs") < at("Biology"), "{texts:?}");
    Ok(())
}

#[test]
fn the_state_row_is_below_every_group() -> Result<(), String> {
    let mut app = start();
    app.wait_for("Library closed")?;
    let texts: Vec<String> = app.screen().texts().iter().map(|t| t.to_string()).collect();
    let at = |label: &str| texts.iter().position(|t| t == label).unwrap_or(usize::MAX);
    assert!(at("Settings") < at("Library closed"), "{texts:?}");
    assert!(at("Biology") < at("Library closed"), "{texts:?}");
    Ok(())
}

#[test]
fn the_groups_follow_the_search_row() -> Result<(), String> {
    let mut app = start();
    app.wait_for("Library closed")?;
    let texts: Vec<String> = app.screen().texts().iter().map(|t| t.to_string()).collect();
    let at = |label: &str| texts.iter().position(|t| t == label).unwrap_or(usize::MAX);
    let between = &texts[at("Search") + 1..at("Inbox")];
    assert!(
        !between.iter().any(|t| t.starts_with("Library")),
        "the state sits between the search row and the groups: {texts:?}"
    );
    Ok(())
}

#[component]
fn OpenProbe() -> impl IntoElement {
    typr::ui::tokens::install();
    let state = Sidebar::new(fixture(), true, typr::screens::sidebar::search_chord());
    sidebar(state, move |key: &SharedString| state.select(key))
}

fn dot_colour(app: &Headless) -> Option<u32> {
    app.screen()
        .view_style("id:library-dot")
        .expect("one dot")
        .expect("the dot is drawn")
        .background_color
}

#[test]
fn an_open_library_has_a_success_dot_and_a_closed_one_a_different_dot() -> Result<(), String> {
    let mut open = Headless::start(EngineConfig::new(Root::of::<OpenProbe>())).unwrap();
    open.wait_for("Library open")?;
    let mut closed = start();
    closed.wait_for("Library closed")?;
    let (open_dot, closed_dot) = (dot_colour(&open), dot_colour(&closed));
    assert!(
        open_dot.is_some() && closed_dot.is_some(),
        "a dot with no fill"
    );
    assert_ne!(
        open_dot, closed_dot,
        "the dot is the same colour open and closed"
    );
    Ok(())
}

// ---- the reference layout, driven from a real snapshot ----

use typr_core::library_view::ViewMode;
use typr_core::store::{CardSummary, Deck, Snapshot};

const NOW: i64 = 1_000_000;

fn deck(id: i64, name: &str) -> Deck {
    Deck {
        id,
        parent: None,
        name: name.to_owned(),
        archived: false,
        reverse: None,
        template: None,
        view: ViewMode::default(),
    }
}

fn snapshot_with(cards: Vec<CardSummary>) -> Snapshot {
    Snapshot {
        decks: vec![deck(1, "Flashcards"), deck(2, "Notes")],
        cards,
        ..Snapshot::default()
    }
}

fn labels(items: &[Item]) -> Vec<String> {
    items.iter().map(|item| item.label.to_string()).collect()
}

#[test]
fn a_new_library_reads_as_the_reference_layout() {
    let snapshot = snapshot_with(Vec::new());
    let due = typr::screens::sidebar::due_counts(&snapshot, NOW);
    let items = typr::screens::sidebar::items_from(&snapshot, &due, NOW);
    assert_eq!(
        labels(&items),
        [
            "Dashboard",
            "Inbox",
            "Due today",
            "New cards",
            "Flashcards",
            "Notes",
            "Import",
            "Settings",
            "Trash"
        ]
    );
    assert!(
        items.iter().all(|item| item.icon.is_some()),
        "every row has a glyph"
    );
    assert!(items[0].disabled, "the dashboard is not available yet");
}

#[test]
fn the_inbox_rows_count_their_cards() {
    let card = |id, due_at, unscheduled| CardSummary {
        id,
        deck: 1,
        prompts: 1,
        unscheduled,
        due_at,
        ..CardSummary::default()
    };
    // Two due, three new, none both.
    let snapshot = snapshot_with(vec![
        card(1, Some(NOW - 1), 0),
        card(2, Some(NOW - 1), 0),
        card(3, None, 1),
        card(4, None, 1),
        card(5, None, 1),
    ]);
    let due = typr::screens::sidebar::due_counts(&snapshot, NOW);
    let items = typr::screens::sidebar::items_from(&snapshot, &due, NOW);
    let count = |label: &str| items.iter().find(|i| i.label == label).unwrap().due;
    assert_eq!(
        (count("Inbox"), count("Due today"), count("New cards")),
        (5, 2, 3)
    );

    let quiet =
        typr::screens::sidebar::items_from(&snapshot_with(Vec::new()), &Default::default(), NOW);
    assert!(
        quiet
            .iter()
            .filter(|i| i.group == Group::Navigation)
            .all(|i| i.due == 0)
    );
}

#[component]
fn Layout() -> impl IntoElement {
    typr::ui::tokens::install();
    let snapshot = snapshot_with(vec![CardSummary {
        id: 1,
        deck: 1,
        prompts: 1,
        unscheduled: 1,
        ..CardSummary::default()
    }]);
    let due = typr::screens::sidebar::due_counts(&snapshot, NOW);
    let items = typr::screens::sidebar::items_from(&snapshot, &due, NOW);
    let state = Sidebar::new(items, true, typr::screens::sidebar::search_chord());
    view()
        .flex_row()
        .child(sidebar(state, move |key: &SharedString| {
            if key.as_ref() != "search" && !key.starts_with("new-deck:") {
                state.select(key);
            }
        }))
        .child(text(move || format!("selected={}", state.selected_text())))
}

fn open_layout() -> Headless {
    Headless::start(EngineConfig::new(Root::of::<Layout>())).unwrap()
}

#[test]
fn the_inbox_is_selected_first_and_folding_it_hides_its_halves_but_not_its_count()
-> Result<(), String> {
    let mut app = open_layout();
    app.wait_for("selected=inbox")?;
    app.wait_for("Due today")?;
    app.press("Inbox 1")?;
    app.wait_for_gone("Due today", std::time::Duration::from_secs(2))?;
    assert!(app.screen().shows("Inbox 1"));
    Ok(())
}

#[test]
fn pressing_the_dashboard_changes_nothing() -> Result<(), String> {
    let mut app = open_layout();
    app.wait_for("Dashboard (coming soon)")?;
    let _ = app.press("Dashboard (coming soon)");
    app.wait_for("selected=inbox")?;
    Ok(())
}
