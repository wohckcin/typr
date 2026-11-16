//! The controls' size scale: one value sets the frame, and the frame grows with the user's text.

use nsh_core::prelude::*;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;
use typr::ui::{
    button::{Variant, button},
    size::Size,
};

#[component]
fn Sizes() -> impl IntoElement {
    typr::ui::tokens::install();
    view()
        .gap_3()
        .p_4()
        .child(
            button()
                .variant(Variant::Primary)
                .label("Small")
                .size(Size::Sm),
        )
        .child(button().variant(Variant::Primary).label("Middle"))
        .child(
            button()
                .variant(Variant::Primary)
                .label("Large")
                .size(Size::Lg),
        )
}

#[component]
fn RaisingText() -> impl IntoElement {
    typr::ui::tokens::install();
    view()
        .p_4()
        .child(button().variant(Variant::Primary).label("Save"))
}

fn height_of(app: &Headless, selector: &str) -> Result<f32, String> {
    let node = app
        .screen()
        .press_target(selector)?
        .ok_or_else(|| format!("{selector} has no node"))?;
    app.screen()
        .layout_nodes()
        .into_iter()
        .find(|candidate| candidate.id == node)
        .map(|laid| laid.bounds.size.height.0)
        .ok_or_else(|| format!("{selector} has no layout"))
}

#[test]
fn a_size_sets_the_frame_and_the_text_size_lifts_it() -> Result<(), String> {
    let mut small = Headless::start(EngineConfig::new(Root::of::<Sizes>())).unwrap();
    small.wait_for("Middle")?;
    let short = height_of(&small, "role:button[Small]")?;
    let middle = height_of(&small, "role:button[Middle]")?;
    let tall = height_of(&small, "role:button[Large]")?;
    assert!(
        short < middle,
        "small is shorter than the default: {short} < {middle}"
    );
    assert!(
        middle < tall,
        "large is taller than the default: {middle} < {tall}"
    );

    let mut plain = Headless::start(EngineConfig::new(Root::of::<RaisingText>())).unwrap();
    plain.wait_for("role:button[Save]")?;
    let at_default = height_of(&plain, "role:button[Save]")?;

    let mut raised = Headless::start(EngineConfig {
        environment: nsh_core::platform::Environment {
            font_scale: 2.0,
            ..Default::default()
        },
        ..EngineConfig::new(Root::of::<RaisingText>())
    })
    .unwrap();
    raised.wait_for("role:button[Save]")?;
    let at_double = height_of(&raised, "role:button[Save]")?;
    assert!(
        at_double > at_default,
        "the frame grew with the text: {at_double} > {at_default}"
    );
    Ok(())
}
