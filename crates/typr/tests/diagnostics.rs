//! The diagnostics screen: a stoppage shows one message, slow commands are listed or stated absent,
//! and an import or export runs its callback and shows its result line.

use std::time::Duration;

use nsh_core::prelude::*;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;
use typr::screens::diagnostics::{SlowCommand, diagnostics, stoppage_message};
use typr::ui::overlay::Toasts;

#[component]
fn Probe() -> impl IntoElement {
    typr::ui::tokens::install();
    let stopped = signal(true);
    let results = Toasts::new();
    view().child(diagnostics(
        stopped,
        vec![SlowCommand {
            name: "load deck".into(),
            millis: 740,
        }],
        results,
        move || results.push("Imported 3 cards".into()),
        move || results.push("Exported".into()),
    ))
}

#[component]
fn Quiet() -> impl IntoElement {
    typr::ui::tokens::install();
    diagnostics(signal(false), Vec::new(), Toasts::new(), || {}, || {})
}

#[test]
fn a_stoppage_is_stated_once_and_absent_otherwise() {
    assert!(stoppage_message(true).is_some());
    assert!(stoppage_message(false).is_none());
}

#[test]
fn slow_commands_are_listed_and_a_stoppage_shows() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
    app.wait_for("text:/load deck · 740 ms/")?;
    app.wait_for("text:/library work stopped/")?;
    Ok(())
}

#[test]
fn a_quiet_library_states_there_are_no_slow_commands() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Quiet>())).unwrap();
    app.wait_for("No slow commands recorded")?;
    assert!(!app.screen().shows("library work stopped"));
    Ok(())
}

#[test]
fn an_import_runs_its_callback_and_shows_the_result() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
    app.wait_for("role:button[Import]")?;
    app.press("role:button[Import]")?;
    app.wait_for("Imported 3 cards")?;
    Ok(())
}

#[test]
fn a_result_appears_then_leaves() -> Result<(), String> {
    let mut config = EngineConfig::new(Root::of::<Probe>());
    config.clock = nsh_core::time::Clock::manual(std::time::SystemTime::now());
    let mut app = Headless::start(config).unwrap();
    app.wait_for("role:button[Import]")?;
    app.press("role:button[Import]")?;
    app.wait_for("Imported 3 cards")?;
    app.fast_forward(Duration::from_secs(10))?;
    app.wait_for_gone("Imported 3 cards", Duration::from_secs(5))?;
    Ok(())
}

#[test]
fn escape_dismisses_a_result_before_its_timeout() -> Result<(), String> {
    let mut config = EngineConfig::new(Root::of::<Probe>());
    config.clock = nsh_core::time::Clock::manual(std::time::SystemTime::now());
    let mut app = Headless::start(config).unwrap();
    app.wait_for("role:button[Import]")?;
    app.press("role:button[Import]")?;
    app.wait_for("Imported 3 cards")?;
    // Escape's back press reaches the stack's cancel. The wait advances the manual clock, so it
    // stays well inside the timeout.
    app.back()?;
    app.wait_for_gone("Imported 3 cards", Duration::from_secs(1))?;
    Ok(())
}

#[test]
fn an_export_shows_its_own_result_line() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
    app.wait_for("role:button[Export]")?;
    app.press("role:button[Export]")?;
    app.wait_for("Exported")?;
    Ok(())
}
