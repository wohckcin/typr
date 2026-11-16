//! The accessibility and keyboard group of the Finishing checklist, as far as a headless driver
//! can hold it: every screen's controls carry a role and a name, the keyboard can move through a
//! screen without breaking it, and an overlay closes on Escape.
//!
//! What a window is needed for — focus order in a real platform, ring placement against the clip,
//! a screen reader, screenshots in both appearances — is not here; `design.md` records it as not
//! run.

use nsh_core::prelude::*;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;
use typr::screens::diagnostics::{SlowCommand, diagnostics};
use typr::screens::editor as card_editor;
use typr::screens::history::{Record, history};
use typr::screens::settings::{LibraryInfo, Values, settings};
use typr::screens::study::study;
use typr_core::screen::Queue;
use typr_core::session::{Entry, Session};
use typr_core::store::{CardSummary, Deck, Snapshot};

fn library_info() -> LibraryInfo {
    LibraryInfo {
        database: "/tmp/typr/typr.sqlite3".into(),
        schema: 12,
        bytes: 4096,
        attachments: "/tmp/typr/attachments".into(),
        logs: "/tmp/typr/logs".into(),
    }
}

fn snapshot() -> Snapshot {
    Snapshot {
        decks: vec![Deck {
            id: 1,
            parent: None,
            name: "Spanish".to_owned(),
            archived: false,
            reverse: None,
            template: None,
            view: Default::default(),
        }],
        cards: vec![CardSummary {
            id: 10,
            deck: 1,
            markdown: "gato\n---\ncat".to_owned(),
            preview: "gato".to_owned(),
            sides: 2,
            ..CardSummary::default()
        }],
        ..Snapshot::default()
    }
}

#[component]
fn LibraryScreen() -> impl IntoElement {
    typr::ui::tokens::install();
    typr::shell::shell(snapshot(), true)
}

#[component]
fn Editor() -> impl IntoElement {
    typr::ui::tokens::install();
    view().flex_1().child(
        card_editor::editor()
            .saved("gato")
            .draft(Some(SharedString::from("gato!"))),
    )
}

#[component]
fn StudyScreen() -> impl IntoElement {
    typr::ui::tokens::install();
    let entries = vec![Entry::sides(1, ["Question", "Answer"], 0)];
    study(
        std::rc::Rc::new(std::cell::RefCell::new(Session::new(Queue::Due, entries))),
        |_, _, _| true,
        || {},
        || {},
    )
}

#[component]
fn HistoryScreen() -> impl IntoElement {
    typr::ui::tokens::install();
    history(vec![Record {
        phase: "review".into(),
        grade: "good".into(),
        at: "10:00".into(),
        due: "tomorrow".into(),
    }])
}

#[component]
fn SettingsScreen() -> impl IntoElement {
    typr::ui::tokens::install();
    settings(Values::new(20, true), library_info(), |_| {}, || {}, || {})
}

#[component]
fn DiagnosticsScreen() -> impl IntoElement {
    typr::ui::tokens::install();
    diagnostics(
        signal(false),
        vec![SlowCommand {
            name: "SaveCard".into(),
            millis: 240,
        }],
        signal(None),
        || {},
        || {},
    )
}

/// Moves the keyboard around a screen and confirms it still shows what it showed: a stop that
/// throws or tears the screen down fails here.
fn keyboard_pass(app: &mut Headless, landmark: &str) -> Result<(), String> {
    for _ in 0..10 {
        app.press_key("tab")?;
    }
    for _ in 0..3 {
        app.press_key("shift+tab")?;
    }
    app.press_key("enter")?;
    app.wait_for(landmark)?;
    Ok(())
}

#[test]
fn the_library_names_every_control_and_survives_the_keyboard() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<LibraryScreen>())).unwrap();
    app.wait_for("role:button[Spanish]")?;
    app.wait_for("role:button[Settings]")?;
    app.wait_for("role:button[Diagnostics]")?;
    keyboard_pass(&mut app, "Spanish")
}

#[test]
fn the_editor_names_its_controls_and_survives_the_keyboard() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Editor>())).unwrap();
    for control in ["Back", "Save", "Attach image"] {
        app.wait_for(&format!("role:button[{control}]"))?;
    }
    keyboard_pass(&mut app, "Unsaved")
}

#[test]
fn the_study_screen_names_its_controls_and_survives_the_keyboard() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<StudyScreen>())).unwrap();
    app.wait_for("role:button[Show answer]")?;
    keyboard_pass(&mut app, "Question")
}

#[test]
fn the_history_screen_survives_the_keyboard() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<HistoryScreen>())).unwrap();
    app.wait_for("text:/review · good/")?;
    keyboard_pass(&mut app, "review · good")
}

#[test]
fn the_settings_screen_names_its_controls_and_survives_the_keyboard() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<SettingsScreen>())).unwrap();
    for control in [
        "Apply",
        "Show Database in the file manager",
        "Show Attachments in the file manager",
        "Show Log in the file manager",
    ] {
        app.wait_for(&format!("role:button[{control}]"))?;
    }
    keyboard_pass(&mut app, "Schema 12")
}

#[test]
fn the_diagnostics_screen_names_its_controls_and_survives_the_keyboard() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<DiagnosticsScreen>())).unwrap();
    app.wait_for("role:button[Import]")?;
    app.wait_for("role:button[Export]")?;
    app.wait_for("text:/SaveCard · 240 ms/")?;
    keyboard_pass(&mut app, "Slow commands")
}

#[test]
fn escape_dismisses_the_leave_question_without_leaving() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Editor>())).unwrap();
    app.wait_for("role:button[Back]")?;
    app.press("role:button[Back]")?;
    app.wait_for("Save this card before leaving?")?;
    // Escape's back press reaches the overlay's cancel (`Headless::back` is what the platforms
    // send), so the question closes without the card being left.
    app.back()?;
    app.wait_for_gone(
        "Save this card before leaving?",
        std::time::Duration::from_secs(5),
    )?;
    app.wait_for("role:button[Back]")?;
    Ok(())
}

#[test]
fn a_larger_text_size_reaches_every_screen_without_losing_a_control() -> Result<(), String> {
    // The window cannot be asked for a larger text size on macOS (the platform pins it to 1), so
    // the check runs the app's own layout at 1.5 as a platform would report it.
    let mut app = Headless::start(EngineConfig::new(Root::of::<LibraryScreen>())).unwrap();
    app.wait_for("role:button[Spanish]")?;
    let normal = app.screen().span_style("text:Spanish")?.font_size;
    app.set_environment("a larger text size", |environment| {
        environment.font_scale = 1.5
    })?;
    app.wait_for("role:button[Spanish]")?;
    let larger = app.screen().span_style("text:Spanish")?.font_size;
    assert!(
        larger > normal,
        "the sidebar's text grew with the setting: {normal:?} then {larger:?}"
    );
    // Every control is still reachable at that size.
    for control in ["All cards", "Spanish", "Settings", "Diagnostics"] {
        app.wait_for(&format!("role:button[{control}]"))?;
    }
    Ok(())
}
