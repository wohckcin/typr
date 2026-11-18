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

/// Renders what the success role resolves to, read where the engine has set the tokens.
#[component]
fn SuccessProbe() -> impl IntoElement {
    typr::ui::tokens::install();
    view().child(text(move || {
        let theme = nsh_base::theme();
        let success = typr::ui::tokens::success();
        format!(
            "success_set={} success_differs={} success_is_foreground={}",
            theme.color("success").is_some(),
            success.light != success.dark,
            success == theme.foreground,
        )
    }))
}

#[test]
fn the_success_colour_is_set_and_differs_in_each_appearance() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<SuccessProbe>())).unwrap();
    app.wait_for("text:/success_set=true success_differs=true success_is_foreground=false/")?;
    Ok(())
}
