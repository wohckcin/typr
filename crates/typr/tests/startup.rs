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
    app.wait_for("Show review counts")?;
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
    // A file where the folder should be: the store cannot open it, and the shell says which path.
    let blocked = std::env::temp_dir().join(format!("typr-blocked-{}", std::process::id()));
    std::fs::write(&blocked, b"not a folder").unwrap();
    // SAFETY: this is the only test in this binary that reads the variable, and it is set before
    // the app is mounted.
    unsafe { std::env::set_var("TYPR_DATA_DIR", blocked.join("library")) };

    let mut app = Headless::start(EngineConfig::new(Root::of::<typr::shell::App>())).unwrap();
    app.wait_for("text:/^Library closed/")?;
    let _ = std::fs::remove_file(&blocked);
    Ok(())
}
