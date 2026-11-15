//! The ui button presses by role and label, and a disabled one does not.


use nsh_core::prelude::*;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;
use typr::ui::button::{Variant, button};

#[component]
fn Probe() -> impl IntoElement {
    typr::ui::tokens::install();
    let count = signal(0_u32);
    view()
        .flex_row()
        .child(button(Variant::Primary, "Add", move || count.update(|n| *n += 1)))
        .child(text(move || format!("pressed {}", count.get())))
}

#[test]
fn a_press_on_the_primary_button_runs_its_action() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
    app.wait_for("pressed 0")?;
    app.press("role:button[Add]")?;
    app.wait_for("pressed 1")?;
    Ok(())
}
