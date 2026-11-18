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
    let due_card = CardSummary {
        id: 10,
        deck: 2,
        due_at: Some(100),
        ..CardSummary::default()
    };
    Snapshot {
        decks: vec![spanish, verbs, biology],
        cards: vec![due_card],
        ..Snapshot::default()
    }
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
    // The decks are one tree, and a row's due count rides in its label: the base element gives a
    // row no trailing slot.
    app.wait_for("Spanish 1")?;
    assert!(app.screen().shows("Biology"));
    // The due card is in Verbs, so Spanish above it counts it too.
    assert!(app.screen().shows("Verbs 1"));
    Ok(())
}

#[test]
fn a_parent_deck_counts_the_cards_due_under_it() -> Result<(), String> {
    #[component]
    fn TwoDue() -> impl IntoElement {
        typr::ui::tokens::install();
        let mut snapshot = fixture();
        snapshot.cards.push(CardSummary {
            id: 11,
            deck: 2,
            due_at: Some(100),
            ..CardSummary::default()
        });
        typr::shell::shell(snapshot, true)
    }
    let mut app = Headless::start(EngineConfig::new(Root::of::<TwoDue>())).unwrap();
    // The count rides in the row's label, and a parent counts what sits under it.
    app.wait_for("Spanish 2")?;
    app.wait_for("Verbs 2")?;
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
fn a_tag_draws_no_sidebar_row() -> Result<(), String> {
    #[component]
    fn Tagged() -> impl IntoElement {
        typr::ui::tokens::install();
        let snapshot = Snapshot {
            cards: vec![CardSummary {
                id: 1,
                tags: vec!["spanish/verbs".to_owned()],
                ..CardSummary::default()
            }],
            ..Snapshot::default()
        };
        typr::shell::shell(snapshot, true)
    }
    let mut app = Headless::start(EngineConfig::new(Root::of::<Tagged>())).unwrap();
    app.wait_for("Inbox")?;
    for gone in ["Tags", "verbs", "All cards", "Diagnostics"] {
        assert!(!app.screen().shows(gone), "{gone} is not a sidebar row");
    }
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
    app.wait_for("Spanish 1")?;
    Ok(())
}

/// A library in a temporary folder: one deck with one card, so `App` opens it from the environment.
fn library_on_disk(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("typr-shell-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let db = typr_core::store::Db::open(&root.join("typr.sqlite3")).unwrap();
    let deck = match db.apply(
        &typr_core::store::Command::CreateDeck {
            parent: None,
            name: "Spanish".to_owned(),
        },
        0,
    ) {
        Ok(typr_core::store::Outcome::Deck(id)) => id,
        other => panic!("deck: {other:?}"),
    };
    db.apply(
        &typr_core::store::Command::CreateCard {
            deck,
            markdown: "gato\n---\ncat".to_owned(),
        },
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
    app.wait_for("Attach image…")?;
    app.press_key("escape")?;
    app.wait_for_gone("Attach image…", std::time::Duration::from_secs(5))?;
    Ok(())
}

/// A library with one diagram card, whose picture is named and captioned.
fn diagram_on_disk(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("typr-shell-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let db = typr_core::store::Db::open(&root.join("typr.sqlite3")).unwrap();
    let deck = match db.apply(
        &typr_core::store::Command::CreateDeck {
            parent: None,
            name: "Biology".to_owned(),
        },
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
    // A diagram with no rectangle has no prompt, so the Inbox does not hold it: open its deck.
    app.wait_for("Biology")?;
    app.press("Biology")?;
    app.wait_for("the cell")?;
    app.press("role:button[the cell]")?;
    app.wait_for("diagram")?;
    let by = Point {
        x: px(80.0),
        y: px(60.0),
    };
    app.drag("text:diagram", by)?;
    app.wait_for("group 1")?;
    // The editor writes through the store, so a second look at the library file sees the rectangle.
    let reader = typr_core::store::Db::open(&root.join("typr.sqlite3")).unwrap();
    let mut masks = Vec::new();
    for _ in 0..40 {
        let cards = reader.snapshot().unwrap().cards;
        masks = cards
            .first()
            .map(|card| card.masks.clone())
            .unwrap_or_default();
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
    // A diagram with no rectangle has no prompt, so the Inbox does not hold it: open its deck.
    app.wait_for("Biology")?;
    app.press("Biology")?;
    app.wait_for("the cell")?;
    app.press("role:button[the cell]")?;
    app.press("role:button[Clear the diagram…]")?;
    app.wait_for("Clear this diagram?")?;
    app.press("role:button[Clear]")?;
    // The card is text again, so the editor offers its source and the diagram's tag is gone.
    app.wait_for("Save")?;
    app.wait_for_gone("Clear the diagram…", std::time::Duration::from_secs(5))?;
    let reader = typr_core::store::Db::open(&root.join("typr.sqlite3")).unwrap();
    let card = reader
        .snapshot()
        .unwrap()
        .cards
        .first()
        .cloned()
        .expect("the card");
    assert_eq!(
        card.kind,
        typr_core::store::CardKind::Text,
        "the library holds a text card now"
    );
    assert!(
        card.image.is_none() && card.masks.is_empty(),
        "the picture and rectangles are gone"
    );
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
    app.wait_for_gone(
        "Save this card before leaving?",
        std::time::Duration::from_secs(5),
    )?;
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
    again.wait_for_gone("Attach image…", std::time::Duration::from_secs(5))?;
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
    app.press("Settings")?;
    // The last settings row sits below the fold of the default window.
    app.set_viewport(Size {
        width: px(1000.0),
        height: px(1600.0),
    })?;
    app.wait_for("role:button[Open diagnostics]")?;
    app.press("role:button[Open diagnostics]")?;
    app.wait_for("Slow commands")?;
    app.wait_for("No slow commands recorded")?;
    app.wait_for("Import")?;
    app.wait_for("Export")?;
    app.press_key("escape")?;
    app.wait_for_gone(
        "No slow commands recorded",
        std::time::Duration::from_secs(5),
    )?;
    Ok(())
}

#[test]
fn an_export_opens_as_a_library_and_a_foreign_file_is_refused_by_name() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let db = typr_core::store::Db::open(&dir.path().join("typr.sqlite3")).expect("the library");
    let deck = match db
        .apply(
            &typr_core::store::Command::CreateDeck {
                parent: None,
                name: "Spanish".to_owned(),
            },
            0,
        )
        .expect("deck")
    {
        typr_core::store::Outcome::Deck(id) => id,
        other => panic!("deck: {other:?}"),
    };
    db.apply(
        &typr_core::store::Command::CreateCard {
            deck,
            markdown: "gato\n---\ncat".to_owned(),
        },
        0,
    )
    .expect("card");

    let target = dir.path().join("export.sqlite3");
    let line = typr::shell::export_to(&db, &target);
    assert_eq!(line, format!("Exported to {}", target.display()));
    assert_eq!(
        typr_core::store::Db::open(&target)
            .expect("the export opens")
            .snapshot()
            .expect("snapshot")
            .cards
            .len(),
        1,
        "the export reopens as a library with the card in it"
    );
    // Exporting again to the same path is refused rather than overwriting the earlier export.
    assert!(
        typr::shell::export_to(&db, &target).contains("already exists")
            || typr::shell::export_to(&db, &target).contains("exists")
    );

    // A file the transfer cannot recognise is refused, and the line names it.
    let foreign = dir.path().join("holiday.zip");
    std::fs::write(&foreign, b"not an archive").expect("the file");
    let refused = typr::shell::import_from(&db, &foreign, dir.path());
    assert!(
        refused.contains("holiday.zip"),
        "the line names the file: {refused}"
    );
    assert_eq!(
        db.snapshot().expect("snapshot").cards.len(),
        1,
        "nothing was written"
    );
}

#[test]
fn the_keyboard_walks_the_library_and_opens_the_chosen_card() -> Result<(), String> {
    let _turn = ENV.lock().unwrap_or_else(|poison| poison.into_inner());
    let root = library_on_disk("keys");
    unsafe { std::env::set_var("TYPR_DATA_DIR", &root) };
    let mut app = Headless::start(EngineConfig::new(Root::of::<typr::shell::App>())).unwrap();
    app.wait_for("role:button[gato]")?;
    // Down walks the grid; Enter edits the card in force, which opens the editor on it.
    app.press_key("down")?;
    app.press_key("enter")?;
    app.wait_for("Attach image…")?;
    Ok(())
}

#[test]
fn a_due_session_opens_from_the_library_and_grades_back_to_the_store() -> Result<(), String> {
    let _turn = ENV.lock().unwrap_or_else(|poison| poison.into_inner());
    let root = library_on_disk("session");
    // A card with a prompt that is due now: the library's `s` serves it.
    {
        let db = typr_core::store::Db::open(&root.join("typr.sqlite3")).unwrap();
        let cards = db.snapshot().unwrap().cards;
        let card = cards.first().expect("the card").id;
        db.apply(
            &typr_core::store::Command::Grade {
                card,
                key: "fwd".to_owned(),
                grade: typr_core::store::Grade::Remembered,
                duration_ms: None,
            },
            86_400_000,
        )
        .expect("a review that schedules the card");
    }
    unsafe { std::env::set_var("TYPR_DATA_DIR", &root) };
    let mut app = Headless::start(EngineConfig::new(Root::of::<typr::shell::App>())).unwrap();
    app.wait_for("role:button[gato]")?;
    app.press_key("s")?;
    // The session asks the card's first side; revealing offers the grades, and grading completes it.
    app.wait_for("role:button[Show answer]")?;
    app.press("role:button[Show answer]")?;
    app.press("role:button[Remembered]")?;
    app.wait_for("text:/Session complete · 1 reviewed/")?;
    // The grade reached the store: the card has a review recorded through the shell.
    let reader = typr_core::store::Db::open(&root.join("typr.sqlite3")).unwrap();
    let card = reader
        .snapshot()
        .unwrap()
        .cards
        .first()
        .cloned()
        .expect("the card");
    let reviews = reader.review_count(card.id).expect("the reviews");
    assert!(
        reviews >= 1,
        "the session's grade was written through the shell: {reviews}"
    );
    Ok(())
}

#[test]
fn a_slow_grade_is_recorded_on_the_diagnostics_screen() -> Result<(), String> {
    let _turn = ENV.lock().unwrap_or_else(|poison| poison.into_inner());
    let root = library_on_disk("slow-grade");
    let path = root.join("typr.sqlite3");
    {
        let db = typr_core::store::Db::open(&path).unwrap();
        let card = db.snapshot().unwrap().cards.first().expect("the card").id;
        db.apply(
            &typr_core::store::Command::Grade {
                card,
                key: "fwd".to_owned(),
                grade: typr_core::store::Grade::Remembered,
                duration_ms: None,
            },
            86_400_000,
        )
        .expect("a review that schedules the card");
    }
    // Every review now takes well over the slow-command threshold, so the grade is slow.
    typr_core::store::Db::open(&path)
        .unwrap()
        .slow_down_reviews()
        .unwrap();
    unsafe { std::env::set_var("TYPR_DATA_DIR", &root) };
    let mut app = Headless::start(EngineConfig::new(Root::of::<typr::shell::App>())).unwrap();
    app.wait_for("role:button[gato]")?;
    app.press_key("s")?;
    app.wait_for("role:button[Show answer]")?;
    app.press("role:button[Show answer]")?;
    app.press("role:button[Remembered]")?;
    app.wait_for("text:/Session complete · 1 reviewed/")?;
    app.press("Back to the library")?;
    app.press("Settings")?;
    // The last settings row sits below the fold of the default window.
    app.set_viewport(Size {
        width: px(1000.0),
        height: px(1600.0),
    })?;
    app.wait_for("role:button[Open diagnostics]")?;
    app.press("role:button[Open diagnostics]")?;
    app.wait_for("text:/Grade · \\d+ ms/")?;
    Ok(())
}

#[test]
fn the_librarys_keys_do_nothing_while_a_card_is_open() -> Result<(), String> {
    let _turn = ENV.lock().unwrap_or_else(|poison| poison.into_inner());
    let root = library_on_disk("guarded");
    unsafe { std::env::set_var("TYPR_DATA_DIR", &root) };
    let mut app = Headless::start(EngineConfig::new(Root::of::<typr::shell::App>())).unwrap();
    app.wait_for("role:button[gato]")?;
    app.press("role:button[gato]")?;
    app.wait_for("Attach image…")?;
    // `c` makes a card on the library screen; here it must make nothing and leave the editor alone.
    app.press_key("c")?;
    app.wait_for("Attach image…")?;
    let reader = typr_core::store::Db::open(&root.join("typr.sqlite3")).unwrap();
    let cards = reader.snapshot().unwrap().cards.len();
    assert_eq!(cards, 1, "no card was made while the editor was open");
    Ok(())
}

#[test]
fn choosing_a_deck_leaves_the_open_card() -> Result<(), String> {
    let _turn = ENV.lock().unwrap_or_else(|poison| poison.into_inner());
    let root = library_on_disk("deck-pick");
    unsafe { std::env::set_var("TYPR_DATA_DIR", &root) };
    let mut app = Headless::start(EngineConfig::new(Root::of::<typr::shell::App>())).unwrap();
    app.wait_for("role:button[gato]")?;
    app.press("role:button[gato]")?;
    app.wait_for("Attach image…")?;
    // A row in the sidebar opens the grid for that row, so the card on screen goes away.
    app.press("role:treeitem[Spanish]")?;
    app.wait_for("Search cards")?;
    let editor_rows = app.screen().count("Attach image…")?;
    assert_eq!(editor_rows, 0, "the editor is gone");
    Ok(())
}

/// A library of two decks with one card each, so a row change repaints a different card first.
fn two_decks_on_disk(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("typr-shell-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let db = typr_core::store::Db::open(&root.join("typr.sqlite3")).unwrap();
    for (deck_name, markdown) in [
        ("Spanish", "gato\n---\ncat"),
        ("Biology", "perro\n---\ndog"),
    ] {
        let deck = match db.apply(
            &typr_core::store::Command::CreateDeck {
                parent: None,
                name: deck_name.to_owned(),
            },
            0,
        ) {
            Ok(typr_core::store::Outcome::Deck(id)) => id,
            other => panic!("deck: {other:?}"),
        };
        db.apply(
            &typr_core::store::Command::CreateCard {
                deck,
                markdown: markdown.to_owned(),
            },
            0,
        )
        .unwrap();
    }
    root
}

#[test]
fn a_filtered_out_card_does_not_leave_the_next_one_selected() -> Result<(), String> {
    let _turn = ENV.lock().unwrap_or_else(|poison| poison.into_inner());
    let root = two_decks_on_disk("filtered");
    unsafe { std::env::set_var("TYPR_DATA_DIR", &root) };
    let mut app = Headless::start(EngineConfig::new(Root::of::<typr::shell::App>())).unwrap();
    app.wait_for("gato")?;
    // Select the first card, then filter it out. Nothing is in force afterwards, so Enter opens
    // nothing rather than the card that took its place.
    app.press_key("down")?;
    app.type_text("text:Search cards", "perro")?;
    app.wait_for_gone("gato", std::time::Duration::from_secs(5))?;
    // Escape leaves the field and shows the whole library again, which is where Enter acts.
    app.press_key("escape")?;
    app.press_key("enter")?;
    assert!(
        !app.screen().shows("Attach image…"),
        "no card opened, because the card that was selected had left the list"
    );
    Ok(())
}

#[test]
fn a_row_change_keeps_the_selected_card_when_it_still_shows() -> Result<(), String> {
    let _turn = ENV.lock().unwrap_or_else(|poison| poison.into_inner());
    let root = two_decks_on_disk("kept");
    unsafe { std::env::set_var("TYPR_DATA_DIR", &root) };
    let mut app = Headless::start(EngineConfig::new(Root::of::<typr::shell::App>())).unwrap();
    app.wait_for("perro")?;
    // Select the second card, then choose the deck that holds only it: it stays in force, and Enter
    // opens it rather than acting on a position that no longer exists.
    app.press_key("down")?;
    app.press_key("down")?;
    app.press("role:treeitem[Biology]")?;
    app.press_key("enter")?;
    app.wait_for("Attach image…")?;
    app.wait_for("text:perro")?;
    Ok(())
}

#[test]
fn the_arrow_keys_step_the_grid_from_the_selected_card() -> Result<(), String> {
    let _turn = ENV.lock().unwrap_or_else(|poison| poison.into_inner());
    let root = two_decks_on_disk("arrows");
    unsafe { std::env::set_var("TYPR_DATA_DIR", &root) };
    let mut app = Headless::start(EngineConfig::new(Root::of::<typr::shell::App>())).unwrap();
    app.wait_for("perro")?;
    // Right selects the first card, Right again the second, and Left steps back to the first.
    app.press_key("right")?;
    app.press_key("right")?;
    app.press_key("left")?;
    app.press_key("enter")?;
    app.wait_for("Attach image…")?;
    app.wait_for("text:gato")?;
    Ok(())
}

#[test]
fn home_and_end_reach_the_ends_of_the_grid() -> Result<(), String> {
    let _turn = ENV.lock().unwrap_or_else(|poison| poison.into_inner());
    let root = two_decks_on_disk("ends");
    unsafe { std::env::set_var("TYPR_DATA_DIR", &root) };
    let mut app = Headless::start(EngineConfig::new(Root::of::<typr::shell::App>())).unwrap();
    app.wait_for("perro")?;
    app.press_key("end")?;
    app.press_key("enter")?;
    app.wait_for("Attach image…")?;
    app.wait_for("text:perro")?;
    app.press_key("escape")?;
    app.wait_for_gone("Attach image…", std::time::Duration::from_secs(5))?;
    app.press_key("home")?;
    app.press_key("enter")?;
    app.wait_for("Attach image…")?;
    app.wait_for("text:gato")?;
    Ok(())
}

#[test]
fn delete_trashes_the_highlighted_card_and_no_other() -> Result<(), String> {
    let _turn = ENV.lock().unwrap_or_else(|poison| poison.into_inner());
    let root = two_decks_on_disk("delete-selected");
    unsafe { std::env::set_var("TYPR_DATA_DIR", &root) };
    let mut app = Headless::start(EngineConfig::new(Root::of::<typr::shell::App>())).unwrap();
    app.wait_for("perro")?;
    // Select the second card and trash it. The card left in the list is the first one.
    app.press_key("down")?;
    app.press_key("down")?;
    app.press_key("backspace")?;
    app.wait_for_gone("perro", std::time::Duration::from_secs(5))?;
    let reader = typr_core::store::Db::open(&root.join("typr.sqlite3")).unwrap();
    let cards = reader.snapshot().unwrap().cards;
    assert!(
        cards.iter().any(|card| card.markdown.starts_with("gato")),
        "the pair of the selected card is still in the library: {cards:?}"
    );
    Ok(())
}

/// A library in a temporary folder with one deck and no cards: the empty grid's button has a deck
/// to put its card in.
fn empty_library_on_disk(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("typr-shell-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let db = typr_core::store::Db::open(&root.join("typr.sqlite3")).unwrap();
    match db.apply(
        &typr_core::store::Command::CreateDeck {
            parent: None,
            name: "Spanish".to_owned(),
        },
        0,
    ) {
        Ok(typr_core::store::Outcome::Deck(_)) => {}
        other => panic!("deck: {other:?}"),
    }
    root
}

#[test]
fn an_empty_deck_offers_a_new_card_that_opens_its_editor() -> Result<(), String> {
    let _turn = ENV.lock().unwrap_or_else(|poison| poison.into_inner());
    let root = empty_library_on_disk("empty");
    unsafe { std::env::set_var("TYPR_DATA_DIR", &root) };
    let mut app = Headless::start(EngineConfig::new(Root::of::<typr::shell::App>())).unwrap();
    app.wait_for("This deck has no cards yet")?;
    app.press("role:button[New card]")?;
    app.wait_for("Attach image…")?;
    // The card is in the library, not only on the screen: the button writes through the store.
    let reader = typr_core::store::Db::open(&root.join("typr.sqlite3")).unwrap();
    assert_eq!(reader.snapshot().unwrap().cards.len(), 1);
    Ok(())
}

#[test]
fn the_library_opens_on_the_inbox_and_escape_returns_to_it() -> Result<(), String> {
    let _turn = ENV.lock().unwrap_or_else(|poison| poison.into_inner());
    // The only card is a diagram with no rectangle: it has no prompt, so the Inbox does not hold it
    // and its deck does.
    let root = diagram_on_disk("inbox-default");
    unsafe { std::env::set_var("TYPR_DATA_DIR", &root) };
    let mut app = Headless::start(EngineConfig::new(Root::of::<typr::shell::App>())).unwrap();
    app.wait_for("Biology")?;
    assert!(
        !app.screen().shows("the cell"),
        "the Inbox opened, not the whole library"
    );
    app.press("Biology")?;
    app.wait_for("the cell")?;
    app.press("Settings")?;
    app.wait_for("Show review counts")?;
    app.press_key("escape")?;
    app.wait_for_gone("Show review counts", std::time::Duration::from_secs(5))?;
    assert!(
        !app.screen().shows("the cell"),
        "Escape went back to the Inbox"
    );
    Ok(())
}

#[test]
fn the_search_row_shows_the_library_from_another_screen() -> Result<(), String> {
    let mut app = start();
    app.wait_for("Settings")?;
    app.press("Settings")?;
    app.wait_for("Show review counts")?;
    app.press("Search")?;
    app.wait_for_gone("Show review counts", std::time::Duration::from_secs(5))?;
    Ok(())
}

#[test]
fn the_trash_row_lists_a_trashed_card_and_restore_brings_it_back() -> Result<(), String> {
    let _turn = ENV.lock().unwrap_or_else(|poison| poison.into_inner());
    let root = library_on_disk("trash");
    {
        let db = typr_core::store::Db::open(&root.join("typr.sqlite3")).unwrap();
        let card = db.snapshot().unwrap().cards[0].id;
        db.apply(&typr_core::store::Command::TrashCard { id: card }, 1)
            .unwrap();
    }
    unsafe { std::env::set_var("TYPR_DATA_DIR", &root) };
    let mut app = Headless::start(EngineConfig::new(Root::of::<typr::shell::App>())).unwrap();
    app.wait_for("Trash")?;
    assert!(
        !app.screen().shows("gato"),
        "a trashed card is not in the Inbox"
    );
    app.press("Trash")?;
    app.wait_for("gato")?;
    app.press("role:button[Restore]")?;
    app.wait_for("The trash is empty")?;
    app.press_key("escape")?;
    app.wait_for("role:button[gato]")?;
    Ok(())
}

#[test]
fn a_named_deck_is_created_and_selected() -> Result<(), String> {
    let _turn = ENV.lock().unwrap_or_else(|poison| poison.into_inner());
    let root = library_on_disk("new-deck");
    unsafe { std::env::set_var("TYPR_DATA_DIR", &root) };
    let mut app = Headless::start(EngineConfig::new(Root::of::<typr::shell::App>())).unwrap();
    app.wait_for("New Deck")?;
    app.press("New Deck")?;
    app.type_text("text:Deck name", " Maths ")?;
    app.press_key("enter")?;
    app.wait_for("role:treeitem[Maths]")?;
    let reader = typr_core::store::Db::open(&root.join("typr.sqlite3")).unwrap();
    let decks = reader.snapshot().unwrap().decks;
    assert!(
        decks
            .iter()
            .any(|d| d.name == "Maths" && d.parent.is_none()),
        "{decks:?}"
    );
    Ok(())
}

#[test]
fn escape_cancels_naming_a_deck() -> Result<(), String> {
    let mut app = start();
    app.wait_for("New Deck")?;
    app.press("New Deck")?;
    app.type_text("text:Deck name", "Spa")?;
    app.press_key("escape")?;
    app.wait_for("New Deck")?;
    assert!(!app.screen().shows("Spa"));
    Ok(())
}

#[test]
fn a_new_card_from_the_inbox_skips_an_archived_first_deck() -> Result<(), String> {
    let _turn = ENV.lock().unwrap_or_else(|poison| poison.into_inner());
    let root = two_decks_on_disk("archived-first");
    let db = typr_core::store::Db::open(&root.join("typr.sqlite3")).unwrap();
    // The snapshot lists decks by name, so Biology is its first deck.
    let first = db.snapshot().unwrap().decks[0].id;
    db.apply(
        &typr_core::store::Command::ArchiveDeck {
            id: first,
            archived: true,
        },
        0,
    )
    .unwrap();
    drop(db);
    unsafe { std::env::set_var("TYPR_DATA_DIR", &root) };
    let mut app = Headless::start(EngineConfig::new(Root::of::<typr::shell::App>())).unwrap();
    app.wait_for("Spanish")?;
    app.press_key("c")?;
    app.wait_for("Attach image…")?;
    let reader = typr_core::store::Db::open(&root.join("typr.sqlite3")).unwrap();
    let snapshot = reader.snapshot().unwrap();
    let made = snapshot
        .cards
        .iter()
        .find(|card| card.markdown.is_empty())
        .expect("the new card");
    let deck = snapshot
        .decks
        .iter()
        .find(|deck| deck.id == made.deck)
        .unwrap();
    assert_eq!(deck.name, "Spanish");
    Ok(())
}
