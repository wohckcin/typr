//! The keymap panel and the keystroke adapter: each binding of a surface is listed with its label,
//! and an NSH keystroke maps to the chord the table holds.

use nsh_core::prelude::*;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;
use typr::screens::keymap::{chord_from_keystroke, keymap_panel, rows};
use typr_core::keymap::{Chord, Key, Mods, Surface, bindings_of};

#[test]
fn a_keystroke_maps_to_the_table_chord() {
    assert_eq!(
        chord_from_keystroke("mod+k", true),
        Some(Chord::Modified { key: Key::Char('k'), mods: Mods { cmd: true, ..Mods::NONE } })
    );
    assert_eq!(chord_from_keystroke("escape", true), Some(Chord::Key(Key::Escape)));
    assert_eq!(chord_from_keystroke("hyper+zz", true), None);
}

#[component]
fn Probe() -> impl IntoElement {
    typr::ui::tokens::install();
    keymap_panel(Surface::Library)
}

#[test]
fn every_binding_of_the_surface_is_listed_with_its_label() -> Result<(), String> {
    let first = bindings_of(Surface::Library).next().expect("the library binds something");
    let label = first.command.label();
    let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
    app.wait_for(label)?;
    assert_eq!(rows(Surface::Library).len(), bindings_of(Surface::Library).count());
    Ok(())
}
