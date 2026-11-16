//! The shell over a library snapshot: the sidebar's rows come from the snapshot, and a library
//! that did not open is said so.

use nsh_core::prelude::*;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;
use typr_core::library_view::ViewMode;
use typr_core::store::{CardSummary, Deck, Snapshot};

fn deck(id: i64, parent: Option<i64>, name: &str) -> Deck {
    Deck {
        id,
        parent,
        name: name.to_owned(),
        archived: false,
        reverse: None,
        template: None,
        view: ViewMode::default(),
    }
}

fn fixture() -> Snapshot {
    let spanish = deck(1, None, "Spanish");
    let verbs = deck(2, Some(1), "Verbs");
    let biology = deck(3, None, "Biology");
    let due_card = CardSummary { id: 10, deck: 2, due_at: Some(100), ..CardSummary::default() };
    Snapshot { decks: vec![spanish, verbs, biology], cards: vec![due_card], ..Snapshot::default() }
}

#[component]
fn Probe() -> impl IntoElement {
    typr::ui::tokens::install();
    typr::shell::shell(fixture(), true)
}

fn start() -> Headless {
    Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap()
}

#[test]
fn the_decks_of_the_library_are_listed_with_their_due_counts() -> Result<(), String> {
    let mut app = start();
    app.wait_for("Spanish")?;
    assert!(app.screen().shows("Biology"));
    assert!(app.screen().shows("Verbs"));
    // The due card is in Verbs, so Spanish above it counts it too.
    app.wait_for("text:/^1$/")?;
    Ok(())
}

#[test]
fn an_empty_library_says_it_is_closed() -> Result<(), String> {
    #[component]
    fn Closed() -> impl IntoElement {
        typr::ui::tokens::install();
        typr::shell::shell(Snapshot::default(), false)
    }
    let mut app = Headless::start(EngineConfig::new(Root::of::<Closed>())).unwrap();
    app.wait_for("Library closed")?;
    Ok(())
}

#[test]
fn a_tag_shows_under_tags_with_its_name_only() -> Result<(), String> {
    #[component]
    fn Tagged() -> impl IntoElement {
        typr::ui::tokens::install();
        let snapshot = Snapshot {
            cards: vec![CardSummary { id: 1, tags: vec!["spanish/verbs".to_owned()], ..CardSummary::default() }],
            ..Snapshot::default()
        };
        typr::shell::shell(snapshot, true)
    }
    let mut app = Headless::start(EngineConfig::new(Root::of::<Tagged>())).unwrap();
    app.wait_for("Tags")?;
    app.wait_for("verbs")?;
    Ok(())
}

#[test]
fn the_settings_row_opens_settings_and_escape_returns() -> Result<(), String> {
    let mut app = start();
    app.wait_for("Settings")?;
    app.press("Settings")?;
    app.wait_for("Show review counts")?;
    app.press_key("escape")?;
    app.wait_for_gone("Show review counts", std::time::Duration::from_secs(5))?;
    app.wait_for("Spanish")?;
    Ok(())
}

/// A library in a temporary folder: one deck with one card, so `App` opens it from the environment.
fn library_on_disk(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("typr-shell-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let db = typr_core::store::Db::open(&root.join("typr.sqlite3")).unwrap();
    let deck = match db.apply(
        &typr_core::store::Command::CreateDeck { parent: None, name: "Spanish".to_owned() },
        0,
    ) {
        Ok(typr_core::store::Outcome::Deck(id)) => id,
        other => panic!("deck: {other:?}"),
    };
    db.apply(
        &typr_core::store::Command::CreateCard { deck, markdown: "gato\n---\ncat".to_owned() },
        0,
    )
    .unwrap();
    root
}

/// The environment is process-wide, so the tests that set it take turns.
static ENV: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn a_tile_opens_the_card_and_escape_returns_to_the_grid() -> Result<(), String> {
    let _turn = ENV.lock().unwrap_or_else(|poison| poison.into_inner());
    let root = library_on_disk("open");
    // `App` opens the library the environment names, which the engine thread shares.
    unsafe { std::env::set_var("TYPR_DATA_DIR", &root) };
    let mut app = Headless::start(EngineConfig::new(Root::of::<typr::shell::App>())).unwrap();
    app.wait_for("gato")?;
    app.press("role:button[gato]")?;
    app.wait_for("Attach image")?;
    app.press_key("escape")?;
    app.wait_for_gone("Attach image", std::time::Duration::from_secs(5))?;
    Ok(())
}

/// A library with one diagram card, whose picture is named and captioned.
fn diagram_on_disk(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("typr-shell-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let db = typr_core::store::Db::open(&root.join("typr.sqlite3")).unwrap();
    let deck = match db.apply(
        &typr_core::store::Command::CreateDeck { parent: None, name: "Biology".to_owned() },
        0,
    ) {
        Ok(typr_core::store::Outcome::Deck(id)) => id,
        other => panic!("deck: {other:?}"),
    };
    db.apply(
        &typr_core::store::Command::CreateDiagram {
            deck,
            image: "cell.png".to_owned(),
            caption: "the cell".to_owned(),
        },
        0,
    )
    .unwrap();
    root
}

#[test]
fn a_drag_on_a_diagram_card_writes_its_rectangle_to_the_library() -> Result<(), String> {
    let _turn = ENV.lock().unwrap_or_else(|poison| poison.into_inner());
    let root = diagram_on_disk("mask");
    unsafe { std::env::set_var("TYPR_DATA_DIR", &root) };
    let mut app = Headless::start(EngineConfig::new(Root::of::<typr::shell::App>())).unwrap();
    app.wait_for("the cell")?;
    app.press("role:button[the cell]")?;
    app.wait_for("diagram")?;
    let by = Point { x: px(80.0), y: px(60.0) };
    app.drag("text:diagram", by)?;
    app.wait_for("group 1")?;
    // The editor writes through the store, so a second look at the library file sees the rectangle.
    let reader = typr_core::store::Db::open(&root.join("typr.sqlite3")).unwrap();
    let mut masks = Vec::new();
    for _ in 0..40 {
        let cards = reader.snapshot().unwrap().cards;
        masks = cards.first().map(|card| card.masks.clone()).unwrap_or_default();
        if !masks.is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(!masks.is_empty(), "the rectangle reached the library");
    Ok(())
}

#[test]
fn clearing_a_diagram_turns_the_card_into_text_in_the_library() -> Result<(), String> {
    let _turn = ENV.lock().unwrap_or_else(|poison| poison.into_inner());
    let root = diagram_on_disk("clear");
    unsafe { std::env::set_var("TYPR_DATA_DIR", &root) };
    let mut app = Headless::start(EngineConfig::new(Root::of::<typr::shell::App>())).unwrap();
    app.wait_for("the cell")?;
    app.press("role:button[the cell]")?;
    app.press("role:button[Clear the diagram]")?;
    app.wait_for("Clear this diagram?")?;
    app.press("role:button[Clear]")?;
    // The card is text again, so the editor offers its source and the diagram's tag is gone.
    app.wait_for("Save")?;
    app.wait_for_gone("Clear the diagram", std::time::Duration::from_secs(5))?;
    let reader = typr_core::store::Db::open(&root.join("typr.sqlite3")).unwrap();
    let card = reader.snapshot().unwrap().cards.first().cloned().expect("the card");
    assert_eq!(card.kind, typr_core::store::CardKind::Text, "the library holds a text card now");
    assert!(card.image.is_none() && card.masks.is_empty(), "the picture and rectangles are gone");
    Ok(())
}

#[test]
fn a_draft_survives_a_restart_until_it_is_discarded() -> Result<(), String> {
    let _turn = ENV.lock().unwrap_or_else(|poison| poison.into_inner());
    let root = library_on_disk("draft");
    unsafe { std::env::set_var("TYPR_DATA_DIR", &root) };
    // Type into the card, then stay, so the draft is written down.
    let mut app = Headless::start(EngineConfig::new(Root::of::<typr::shell::App>())).unwrap();
    app.wait_for("gato")?;
    app.press("role:button[gato]")?;
    app.wait_for("Card source")?;
    app.type_text("label:Card source", "!")?;
    // The focused source keeps Escape for itself; the header's way back asks instead.
    app.press("role:button[Back]")?;
    app.wait_for("Save this card before leaving?")?;
    app.press("role:button[Stay]")?;
    app.wait_for_gone("Save this card before leaving?", std::time::Duration::from_secs(5))?;
    drop(app);

    // A second launch opens on the draft it left behind, marked unsaved.
    let mut again = Headless::start(EngineConfig::new(Root::of::<typr::shell::App>())).unwrap();
    again.wait_for("gato")?;
    again.press("role:button[gato]")?;
    again.wait_for("Unsaved")?;

    // Discarding throws it away: the next launch opens the saved card again.
    again.press("role:button[Back]")?;
    again.wait_for("Save this card before leaving?")?;
    again.press("role:button[Discard]")?;
    again.wait_for_gone("Attach image", std::time::Duration::from_secs(5))?;
    drop(again);

    let mut once_more = Headless::start(EngineConfig::new(Root::of::<typr::shell::App>())).unwrap();
    once_more.wait_for("gato")?;
    once_more.press("role:button[gato]")?;
    once_more.wait_for("Save")?;
    assert!(
        !once_more.screen().shows("Unsaved"),
        "the draft was discarded, so the card opens saved"
    );
    Ok(())
}

#[test]
fn the_diagnostics_row_opens_the_screen_with_both_transfer_entry_points() -> Result<(), String> {
    let _turn = ENV.lock().unwrap_or_else(|poison| poison.into_inner());
    let root = library_on_disk("diagnostics");
    unsafe { std::env::set_var("TYPR_DATA_DIR", &root) };
    let mut app = Headless::start(EngineConfig::new(Root::of::<typr::shell::App>())).unwrap();
    app.wait_for("Settings")?;
    app.press("Diagnostics")?;
    app.wait_for("Slow commands")?;
    app.wait_for("No slow commands recorded")?;
    app.wait_for("Import")?;
    app.wait_for("Export")?;
    app.press_key("escape")?;
    app.wait_for_gone("No slow commands recorded", std::time::Duration::from_secs(5))?;
    Ok(())
}

#[test]
fn an_export_opens_as_a_library_and_a_foreign_file_is_refused_by_name() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let db = typr_core::store::Db::open(&dir.path().join("typr.sqlite3")).expect("the library");
    let deck = match db
        .apply(&typr_core::store::Command::CreateDeck { parent: None, name: "Spanish".to_owned() }, 0)
        .expect("deck")
    {
        typr_core::store::Outcome::Deck(id) => id,
        other => panic!("deck: {other:?}"),
    };
    db.apply(
        &typr_core::store::Command::CreateCard { deck, markdown: "gato\n---\ncat".to_owned() },
        0,
    )
    .expect("card");

    let target = dir.path().join("export.sqlite3");
    let line = typr::shell::export_to(&db, &target);
    assert_eq!(line, format!("Exported to {}", target.display()));
    assert_eq!(
        typr_core::store::Db::open(&target).expect("the export opens").snapshot().expect("snapshot").cards.len(),
        1,
        "the export reopens as a library with the card in it"
    );
    // Exporting again to the same path is refused rather than overwriting the earlier export.
    assert!(typr::shell::export_to(&db, &target).contains("already exists") || typr::shell::export_to(&db, &target).contains("exists"));

    // A file the transfer cannot recognise is refused, and the line names it.
    let foreign = dir.path().join("holiday.zip");
    std::fs::write(&foreign, b"not an archive").expect("the file");
    let refused = typr::shell::import_from(&db, &foreign, dir.path());
    assert!(refused.contains("holiday.zip"), "the line names the file: {refused}");
    assert_eq!(db.snapshot().expect("snapshot").cards.len(), 1, "nothing was written");
}
