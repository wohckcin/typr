//! The segmented control: every value is a button, pressing one reports its value, and the chosen
//! one is exposed as selected.

use nsh_core::prelude::*;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;
use typr::ui::segmented::segmented;

#[component]
fn Probe() -> impl IntoElement {
    typr::ui::tokens::install();
    let chosen = signal(1_u8);
    view()
        .child(segmented(
            vec![("System", 0_u8), ("Light", 1), ("Dark", 2)],
            move || chosen.get(),
            move |next| chosen.set(next),
        ))
        .child(text(move || format!("chosen={}", chosen.get())))
}

fn start() -> Headless {
    Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap()
}

#[test]
fn pressing_a_segment_reports_its_value() -> Result<(), String> {
    let mut app = start();
    app.wait_for("chosen=1")?;
    app.press("role:button[Dark]")?;
    app.wait_for("chosen=2")?;
    app.press("role:button[System]")?;
    app.wait_for("chosen=0")?;
    Ok(())
}

#[test]
fn the_chosen_segment_is_the_selected_one() -> Result<(), String> {
    let mut app = start();
    app.wait_for("chosen=1")?;
    let selected = |app: &Headless, name: &str| {
        app.screen()
            .accessibility_state(&format!("role:button[{name}]"))
            .map(|state| state.selected)
    };
    assert_eq!(selected(&app, "Light"), Ok(true));
    assert_eq!(selected(&app, "Dark"), Ok(false));
    app.press("role:button[Dark]")?;
    app.wait_for("chosen=2")?;
    assert_eq!(selected(&app, "Dark"), Ok(true));
    assert_eq!(selected(&app, "Light"), Ok(false));
    Ok(())
}
