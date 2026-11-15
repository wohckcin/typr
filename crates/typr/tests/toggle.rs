//! A checkbox and a switch change the signal they are bound to when pressed.

use nsh_core::prelude::*;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;
use typr::ui::toggle::{check, toggle};

#[component]
fn Probe() -> impl IntoElement {
    typr::ui::tokens::install();
    let box_on = signal(false);
    let switch_on = signal(false);
    view()
        .gap_2()
        .child(check("Remember", box_on))
        .child(toggle("Sound", switch_on))
        .child(text(move || format!("box={} switch={}", box_on.get(), switch_on.get())))
}

fn start() -> Headless {
    Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap()
}

#[test]
fn pressing_the_checkbox_checks_it() -> Result<(), String> {
    let mut app = start();
    app.wait_for("text:/box=false/")?;
    app.press("role:checkbox[Remember]")?;
    app.wait_for("text:/box=true/")?;
    Ok(())
}

#[test]
fn pressing_the_switch_turns_it_on() -> Result<(), String> {
    let mut app = start();
    app.wait_for("text:/switch=false/")?;
    app.press("role:switch[Sound]")?;
    app.wait_for("text:/switch=true/")?;
    Ok(())
}
