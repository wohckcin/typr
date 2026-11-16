//! The theme reaches a running engine: every role a screen names resolves, and the app's own
//! tokens are the ones from `theme.json`, not a fallback.

use nsh_core::prelude::*;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;

/// Renders what the tokens resolve to, so a test reads them where the engine has set them.
#[component]
fn Probe() -> impl IntoElement {
    typr::ui::tokens::install();
    view().child(text(move || {
        let theme = nsh_base::theme();
        let backdrop = typr::ui::tokens::review_backdrop();
        format!(
            "custom={} differs={} is_background={} radius={}",
            theme.color("review-backdrop").is_some(),
            backdrop.light != backdrop.dark,
            backdrop == theme.background,
            typr::ui::tokens::radius().0,
        )
    }))
}

fn start() -> Headless {
    Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap()
}

#[test]
fn the_apps_own_token_resolves_in_both_appearances() -> Result<(), String> {
    let mut app = start();
    app.wait_for("text:/custom=true differs=true is_background=false/")?;
    Ok(())
}

#[test]
fn the_radius_is_the_themes() -> Result<(), String> {
    let mut app = start();
    // The default theme's `radius` is 0.625rem, which is 10 points.
    app.wait_for("text:/ radius=10/")?;
    Ok(())
}
