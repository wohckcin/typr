//! The fill a sidebar row takes: the selected one has a fill its sibling does not, and it is not
//! transparent. Read on the engine, where the theme exists.

use nsh_core::prelude::*;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;

#[component]
fn Probe() -> impl IntoElement {
    typr::ui::tokens::install();
    view().child(text(|| {
        let selected = typr::ui::tokens::row_fill(true);
        let plain = typr::ui::tokens::row_fill(false);
        format!(
            "differs={} selected_opaque={}",
            selected != plain,
            selected.light.a > 0.0 && selected.dark.a > 0.0
        )
    }))
}

#[test]
fn the_selected_row_has_a_fill_its_sibling_does_not() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
    app.wait_for("text:/differs=true selected_opaque=true/")?;
    Ok(())
}
