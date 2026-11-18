//! The icon set: every glyph draws in both appearances without a panic or a parse failure.

use nsh_core::prelude::*;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;
use typr::ui::icon::{Icon, icon};

#[component]
fn Probe() -> impl IntoElement {
    typr::ui::tokens::install();
    let mut row = view().flex_row().gap_2();
    for glyph in Icon::ALL {
        row = row.child(icon(glyph));
    }
    view().child(row).child(text("icons drawn"))
}

#[test]
fn every_glyph_draws() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
    app.wait_for("icons drawn")?;
    Ok(())
}

#[test]
fn every_glyph_is_well_formed_svg() {
    // A source that cannot be parsed draws nothing and only logs, so the markup is checked here.
    for glyph in Icon::ALL {
        let markup = glyph.markup();
        assert!(markup.starts_with("<svg") && markup.ends_with("</svg>"));
        assert_eq!(
            markup.matches('<').count(),
            markup.matches('>').count(),
            "{glyph:?}"
        );
    }
}
