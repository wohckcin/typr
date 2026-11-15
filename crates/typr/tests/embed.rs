//! Card embeds: an embed draws the other card's body set apart, a missing card is named, and a card
//! that embeds itself draws a marker rather than its own text.

use nsh_core::prelude::*;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;
use typr::screens::embed::{Cards, embedded};

fn cards() -> Cards {
    vec![("Photosynthesis".to_owned(), "Light becomes sugar.".to_owned())]
}

#[component]
fn Note() -> impl IntoElement {
    typr::ui::tokens::install();
    embedded("Before ![[Photosynthesis]] after", cards(), "Note".to_owned())
}

#[component]
fn Missing() -> impl IntoElement {
    typr::ui::tokens::install();
    embedded("See ![[Nowhere]] here", cards(), "Note".to_owned())
}

#[component]
fn SelfEmbed() -> impl IntoElement {
    typr::ui::tokens::install();
    let me: Cards = vec![("Loop".to_owned(), "Intro ![[Loop]] end".to_owned())];
    embedded("Intro ![[Loop]] end", me, "Loop".to_owned())
}

#[test]
fn an_embed_draws_the_other_card_body() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Note>())).unwrap();
    app.wait_for("Light becomes sugar.")?;
    Ok(())
}

#[test]
fn a_missing_card_is_named() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Missing>())).unwrap();
    app.wait_for("No card named Nowhere")?;
    Ok(())
}

#[test]
fn a_self_embed_draws_a_marker_not_its_body() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<SelfEmbed>())).unwrap();
    app.wait_for("“Loop” embeds itself")?;
    Ok(())
}
