//! Settings rows: the label comes first and the control sits at the card's trailing edge.

use nsh_core::prelude::*;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;
use typr::ui::rows::{rows, setting_row};

#[component]
fn Probe() -> impl IntoElement {
    typr::ui::tokens::install();
    rows()
        .test_id("card")
        .child(setting_row("Name", view().test_id("control").child(text("value"))))
        .child(setting_row("Other", view().child(text("more"))))
}

#[test]
fn a_row_has_its_label_before_its_control() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
    app.wait_for("Name")?;
    let nodes = app.screen().layout_nodes();
    let by_id = |id: &str| {
        nodes
            .iter()
            .find(|node| node.test_id.as_deref() == Some(id))
            .unwrap_or_else(|| panic!("no node with id {id}"))
            .bounds
    };
    let (card, control) = (by_id("card"), by_id("control"));
    let card_right = card.origin.x.0 + card.size.width.0;
    let control_right = control.origin.x.0 + control.size.width.0;
    assert!(
        control.origin.x.0 > card.origin.x.0 + card.size.width.0 / 2.0,
        "the control is in the leading half of the card"
    );
    assert!(
        card_right - control_right < 32.0,
        "the control does not reach the trailing edge: {card_right} vs {control_right}"
    );
    Ok(())
}
