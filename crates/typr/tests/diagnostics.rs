//! The diagnostics screen: a stoppage shows one message, slow commands are listed or stated absent,
//! and an import or export runs its callback and shows its result line.

use nsh_core::prelude::*;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;
use typr::screens::diagnostics::{SlowCommand, diagnostics, stoppage_message};

#[component]
fn Probe() -> impl IntoElement {
    typr::ui::tokens::install();
    let stopped = signal(true);
    let result = signal(None::<SharedString>);
    view().child(diagnostics(
        stopped,
        vec![SlowCommand { name: "load deck".into(), millis: 740 }],
        result,
        move || result.set(Some("Imported 3 cards".into())),
        move || result.set(Some("Exported".into())),
    ))
}

#[component]
fn Quiet() -> impl IntoElement {
    typr::ui::tokens::install();
    diagnostics(signal(false), Vec::new(), signal(None), || {}, || {})
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
fn an_export_shows_its_own_result_line() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
    app.wait_for("role:button[Export]")?;
    app.press("role:button[Export]")?;
    app.wait_for("Exported")?;
    Ok(())
}
