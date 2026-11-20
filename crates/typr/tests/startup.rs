//! The app opens: the first frame draws, the frame after a change draws too, and a library that
//! cannot be opened says so instead of panicking.

use nsh_core::prelude::*;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;
use typr_core::store::{CardSummary, Deck, Snapshot};

fn card(id: i64, preview: &str, position: i64) -> CardSummary {
    CardSummary {
        id,
        deck: 1,
        preview: preview.into(),
        markdown: preview.into(),
        sides: 1,
        // A new prompt, so the Inbox the app opens on holds the card.
        prompts: 1,
        unscheduled: 1,
        position,
        ..CardSummary::default()
    }
}

fn fixture() -> Snapshot {
    Snapshot {
        decks: vec![Deck {
            id: 1,
            parent: None,
            name: "Spanish".into(),
            archived: false,
            reverse: None,
            template: None,
            view: typr_core::library_view::ViewMode::default(),
        }],
        cards: vec![card(1, "hola", 0), card(2, "adios", 1)],
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
fn the_app_renders_twice_without_panicking() -> Result<(), String> {
    let mut app = start();
    app.wait_for("hola")?;
    // A route change disposes the work area's last screen and draws another; the library comes back
    // with a fresh render, which is the frame that used to read a disposed prop.
    app.press("Settings")?;
    app.wait_for("Find a setting")?;
    app.press_key("escape")?;
    app.wait_for("hola")?;
    Ok(())
}

#[test]
fn the_library_moves_with_the_arrow_keys() -> Result<(), String> {
    let mut app = start();
    app.wait_for("hola")?;
    // The first move lands on the first card; the second reaches the one after it.
    app.press_key("down")?;
    app.press_key("down")?;
    // Enter opens the card in force, which is the second one of the two.
    app.press_key("enter")?;
    app.wait_for("adios")?;
    Ok(())
}

#[test]
fn a_missing_data_folder_is_a_message_not_a_panic() -> Result<(), String> {
    let _turn = ENV.lock().unwrap_or_else(|poison| poison.into_inner());
    // A file where the folder should be: the store cannot open it, and the shell says which path.
    let blocked = std::env::temp_dir().join(format!("typr-blocked-{}", std::process::id()));
    std::fs::write(&blocked, b"not a folder").unwrap();
    // SAFETY: the tests that read the variable take turns.
    unsafe { std::env::set_var("TYPR_DATA_DIR", blocked.join("library")) };

    let mut app = Headless::start(EngineConfig::new(Root::of::<typr::shell::App>())).unwrap();
    app.wait_for("text:/^Library closed/")?;
    let _ = std::fs::remove_file(&blocked);
    Ok(())
}

/// The environment is process-wide, so the tests that set it take turns.
static ENV: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn a_library_that_does_not_exist_starts_with_flashcards_and_notes() -> Result<(), String> {
    let _turn = ENV.lock().unwrap_or_else(|poison| poison.into_inner());
    let root = std::env::temp_dir().join(format!("typr-startup-seed-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    unsafe { std::env::set_var("TYPR_DATA_DIR", &root) };
    // The folder is absent on the first open and empty on the second: both are the same library.
    let db = typr::paths::open_from_environment().map_err(|error| format!("{error:?}"))?;
    let snapshot = db.snapshot().map_err(|error| format!("{error:?}"))?;
    let names: Vec<_> = snapshot.decks.iter().map(|d| d.name.as_str()).collect();
    assert_eq!(names, ["Flashcards", "Notes"]);
    assert!(snapshot.decks.iter().all(|d| d.parent.is_none()));
    let again = typr::paths::open_from_environment().map_err(|error| format!("{error:?}"))?;
    let reopened = again.snapshot().map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        reopened.decks.len(),
        2,
        "the seed happens once, not on every open"
    );
    Ok(())
}

#[test]
fn a_library_that_has_decks_gains_no_starter_decks() -> Result<(), String> {
    let _turn = ENV.lock().unwrap_or_else(|poison| poison.into_inner());
    let root = std::env::temp_dir().join(format!("typr-startup-keep-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    unsafe { std::env::set_var("TYPR_DATA_DIR", &root) };
    let db = typr::paths::open_from_environment().map_err(|e| format!("{e:?}"))?;
    // Archived decks still count as decks, so archiving both must not bring the seed back.
    for deck in db.snapshot().map_err(|e| format!("{e:?}"))?.decks {
        let archive = typr_core::store::Command::ArchiveDeck {
            id: deck.id,
            archived: true,
        };
        db.apply(&archive, 0).map_err(|e| format!("{e:?}"))?;
    }
    drop(db);
    let again = typr::paths::open_from_environment().map_err(|e| format!("{e:?}"))?;
    let decks = again.snapshot().map_err(|e| format!("{e:?}"))?.decks;
    assert_eq!(
        decks.len(),
        2,
        "nothing is added to a library that has decks"
    );
    Ok(())
}
