//! The command list: every binding of a surface is one item, the palette runs the row it is on,
//! and the shortcut panel lists the same bindings without running any.

use nsh_core::prelude::*;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;
use typr::screens::keymap::{command_of, items, keystroke_of, keystroke_of_chord, panel};
use typr::ui::command::Mode;
use typr_core::keymap::{Command, Surface, bindings_of};

#[test]
fn every_binding_is_a_keystroke_nsh_parses() {
    for binding in typr_core::keymap::bindings() {
        let stroke = keystroke_of_chord(binding.chord)
            .unwrap_or_else(|| panic!("{} has no keystroke", binding.chord));
        nsh_core::keymap::Keystroke::parse_with(&stroke, true)
            .unwrap_or_else(|e| panic!("{} -> {stroke:?}: {e}", binding.chord));
    }
}

#[test]
fn every_command_is_one_item_and_nothing_else() {
    let listed = items(Surface::Library);
    let mut names: Vec<&str> = bindings_of(Surface::Library)
        .map(|binding| binding.command.name())
        .collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(listed.len(), names.len(), "one row per command");
    // A command with more than one chord is one row, and every one of its chords finds it.
    for binding in bindings_of(Surface::Library) {
        let rows: Vec<_> = listed
            .iter()
            .filter(|item| item.id == binding.command.name())
            .collect();
        assert_eq!(rows.len(), 1, "{} is one row", binding.command.name());
        let row = rows[0];
        assert_eq!(row.label, binding.command.label());
        assert_eq!(row.disabled, !binding.command.runnable());
        assert_eq!(command_of(&row.id), Some(binding.command));
        let chord: SharedString = binding.chord.to_string().into();
        assert!(row.keywords.contains(&chord), "{} holds {chord}", row.label);
    }
    // A command the palette cannot reach is dimmed rather than dropped: the key is bound and does
    // nothing, which is the fact the row states.
    assert!(listed.iter().any(|item| item.disabled));
    assert!(listed.iter().any(|item| !item.disabled));
}

#[component]
fn PaletteProbe() -> impl IntoElement {
    typr::ui::tokens::install();
    let open = signal(false);
    let ran = signal(String::from("nothing run"));
    on_key("mod+k", move |_| open.update(|shown| *shown = !*shown));
    let shortcut = keystroke_of(Surface::Library, Command::ShowShortcuts).expect("the panel");
    let panel_open = signal(false);
    on_key(&shortcut, move |_| {
        panel_open.update(|shown| *shown = !*shown)
    });
    view()
        // The indicator comes first: a row list of cards after it is drawn into its own subtree.
        .child(text(move || format!("ran={}", ran.get())))
        .child(typr::ui::overlay::decision(open, {
            move || {
                panel(
                    Surface::Library,
                    Mode::Palette,
                    move |id: &SharedString| {
                        ran.set(id.to_string());
                        open.set(false);
                    },
                    move || open.set(false),
                )
            }
        }))
        .child(typr::ui::overlay::decision(panel_open, move || {
            panel(
                Surface::Library,
                Mode::Panel,
                |_: &SharedString| {},
                move || panel_open.set(false),
            )
        }))
}

fn start() -> Result<Headless, String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<PaletteProbe>())).unwrap();
    app.first_render()?;
    Ok(app)
}

#[test]
fn the_palette_runs_the_row_that_is_pressed() -> Result<(), String> {
    // A headless run gives the search field no keyboard focus, so the field's own key handling
    // (Down, Return) cannot be driven here; the host test `command.test.tsx` drives the keys. What
    // this proves is the run path: the row the user presses is the command that runs.
    let third = items(Surface::Library)[2].id.clone();
    let mut app = start()?;
    app.press_key("mod+k")?;
    let label = items(Surface::Library)[2].label.clone();
    app.wait_for(&label)?;
    app.press(&label)?;
    app.wait_for(&format!("ran={third}"))?;
    assert!(!app.screen().shows(&label), "the palette closes");
    Ok(())
}

#[test]
fn the_panel_lists_the_bindings_and_runs_nothing() -> Result<(), String> {
    let shortcut = keystroke_of(Surface::Library, Command::ShowShortcuts).expect("the panel");
    let mut app = start()?;
    app.press_key(&shortcut)?;
    app.wait_for("Keyboard shortcuts")?;
    let first = items(Surface::Library)[0].label.clone();
    assert!(app.screen().shows(&first), "the panel lists the first row");
    app.press(&first)?;
    assert!(
        app.screen().shows("ran=nothing run"),
        "the panel runs nothing"
    );
    Ok(())
}

#[test]
fn typing_narrows_the_list_and_case_does_not_matter() -> Result<(), String> {
    let mut app = start()?;
    app.press_key("mod+k")?;
    app.wait_for("Edit the selected card")?;
    app.type_text("Run a command", "SETT")?;
    app.wait_for("Settings")?;
    assert!(
        !app.screen().shows("New card"),
        "the query filters the list"
    );
    Ok(())
}
