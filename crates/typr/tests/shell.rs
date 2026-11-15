//! The shell: the sidebar names its three groups with their counts, and an empty group is still
//! named rather than hidden.

use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;
use typr::shell::App;

fn start() -> Headless {
    Headless::start(EngineConfig::new(Root::of::<App>())).unwrap()
}

#[test]
fn the_sidebar_names_its_groups_with_their_counts() -> Result<(), String> {
    let mut app = start();
    app.wait_for("Decks")?;
    assert!(app.screen().shows("Tags"));
    assert!(app.screen().shows("Views"));
    app.wait_for("text:/^2$/")?;
    Ok(())
}

#[test]
fn an_empty_group_is_still_named() -> Result<(), String> {
    let mut app = start();
    app.wait_for("Views")?;
    app.wait_for("text:/^0$/")?;
    Ok(())
}
