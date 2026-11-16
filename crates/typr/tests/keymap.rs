//! The keymap panel and the keystroke adapter: each binding of a surface is listed with its label,
//! and an NSH keystroke maps to the chord the table holds.

use nsh_core::prelude::*;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;
use typr::screens::keymap::{chord_from_keystroke, keymap_panel, keystroke_of, palette as palette_widget, rows, visible_rows};
use typr_core::keymap::{bindings_of, Chord, Command, Key, Mods, Surface};

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
    let shortcut = keystroke_of(Surface::Library, Command::ShowShortcuts).expect("the panel");
    let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
    app.first_render()?;
    app.press_key(&shortcut)?;
    app.wait_for(label)?;
    assert_eq!(rows(Surface::Library).len(), bindings_of(Surface::Library).count());
    Ok(())
}

#[test]
fn filtering_narrows_the_list_ignoring_case() {
    let all = rows(Surface::Library);
    assert!(!all.is_empty());
    let label = all[0].0;
    let needle = label[..4].to_uppercase();
    let narrowed = visible_rows(Surface::Library, &needle);
    assert!(!narrowed.is_empty());
    assert!(narrowed.len() <= all.len());
    assert!(narrowed.iter().all(|(l, _)| l.to_lowercase().contains(&needle.to_lowercase())));
    assert_eq!(visible_rows(Surface::Library, "").len(), all.len());
    assert!(visible_rows(Surface::Library, "zzzznomatch").is_empty());
}

#[component]
fn Panel() -> impl IntoElement {
    typr::ui::tokens::install();
    keymap_panel(Surface::Library)
}

#[test]
fn the_panel_opens_and_closes_on_its_shortcut_key() -> Result<(), String> {
    let shortcut = keystroke_of(Surface::Library, Command::ShowShortcuts).expect("the table binds the panel");
    let first = rows(Surface::Library)[0].0;
    let mut app = Headless::start(EngineConfig::new(Root::of::<Panel>())).unwrap();
    app.first_render()?;
    assert!(!app.screen().shows(first), "the panel starts closed");
    app.press_key(&shortcut)?;
    app.wait_for(first)?;
    app.press_key(&shortcut)?;
    assert!(!app.screen().shows(first), "the same key closes it");
    Ok(())
}

#[component]
fn Filtering() -> impl IntoElement {
    typr::ui::tokens::install();
    keymap_panel(Surface::Library)
}

#[test]
fn typing_in_the_panel_narrows_its_rows() -> Result<(), String> {
    let shortcut = keystroke_of(Surface::Library, Command::ShowShortcuts).expect("the panel");
    let labels = rows(Surface::Library);
    let needle = labels[0].0.to_owned();
    let other = labels
        .iter()
        .map(|(label, _)| *label)
        .find(|label| !label.to_lowercase().contains(&needle.to_lowercase()) && !needle.to_lowercase().contains(&label.to_lowercase()))
        .expect("another command");
    let mut app = Headless::start(EngineConfig::new(Root::of::<Filtering>())).unwrap();
    app.first_render()?;
    app.press_key(&shortcut)?;
    app.wait_for("Filter commands")?;
    app.type_text("Filter commands", &needle)?;
    app.wait_for(&needle)?;
    assert!(!app.screen().shows(other), "{other} should be filtered out");
    Ok(())
}

#[component]
fn PaletteProbe() -> impl IntoElement {
    typr::ui::tokens::install();
    let ran = signal(String::from("nothing run"));
    // The indicator comes first: a row list of cards after it is drawn into its own subtree.
    view()
        .child(text(move || format!("ran={}", ran.get())))
        .child(palette_widget(Surface::Library, move |label| ran.set(label.to_owned())))
}

#[test]
fn the_palette_runs_the_first_command_its_query_narrows_to() -> Result<(), String> {
    let label = rows(Surface::Library)[0].0;
    let needle = label[..4].to_owned();
    let expected = visible_rows(Surface::Library, &needle)
        .into_iter()
        .next()
        .map(|(label, _)| label)
        .expect("the query matches a command");
    let mut app = Headless::start(EngineConfig::new(Root::of::<PaletteProbe>())).unwrap();
    app.wait_for("Run a command")?;
    app.type_text("Run a command", &needle)?;
    app.submit(&format!("text:{needle}"))?;
    app.wait_for(&format!("ran={expected}"))?;
    Ok(())
}
