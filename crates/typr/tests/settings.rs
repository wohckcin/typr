//! The settings screen: a valid cap is accepted, an out-of-range one is refused with its limit and
//! the old value stays. Persistence is out of scope and is not tested here.

use nsh_core::prelude::*;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;
use typr::screens::settings::{Values, check_new_card_cap, settings, NEW_CARD_CAP_MAX};

#[component]
fn Probe() -> impl IntoElement {
    typr::ui::tokens::install();
    let values = Values::new(20, true);
    view()
        .child(settings(values, |_| {}))
        .child(text(move || format!("cap={}", values.cap.get())))
}

#[test]
fn an_out_of_range_cap_is_refused_and_the_old_value_stays() {
    assert_eq!(check_new_card_cap("100000"), Err(format!("New cards a day must be {NEW_CARD_CAP_MAX} or less")));
    assert_eq!(check_new_card_cap("abc").is_err(), true);
    assert_eq!(check_new_card_cap(" 25 "), Ok(25));
}

#[test]
fn a_refused_cap_names_its_limit_on_screen() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
    app.wait_for("text:/cap=20/")?;
    app.clear("text:20")?;
    app.type_text("text:", "100000")?;
    app.press("role:button[Apply]")?;
    app.wait_for("text:/10000 or less/")?;
    app.wait_for("text:/cap=20/")?;
    Ok(())
}

#[test]
fn an_accepted_cap_takes_effect_at_once() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
    app.wait_for("text:/cap=20/")?;
    app.clear("text:20")?;
    app.type_text("text:", "35")?;
    app.press("role:button[Apply]")?;
    app.wait_for("text:/cap=35/")?;
    Ok(())
}
