use super::*;
use crate::keymap::{keycap, label, palette, panel};
use rune::Mods as RuneMods;
use typr_core::keymap::{Command, bindings};

fn event(key: RuneKey) -> KeyEvent {
    KeyEvent {
        key,
        mods: RuneMods::default(),
    }
}

fn text(value: &str) -> KeyEvent {
    event(RuneKey::Text(value.to_string()))
}

#[test]
fn an_uppercase_letter_dispatches_the_lowercase_binding() {
    assert_eq!(
        command_for(Surface::Library, &text("S")),
        Some(Command::StartDueSession)
    );
    assert_eq!(
        command_for(Surface::Library, &text("s")),
        Some(Command::StartDueSession)
    );
}

#[test]
fn space_arrives_as_text_and_still_reveals() {
    assert_eq!(
        command_for(Surface::Session, &text(" ")),
        Some(Command::Reveal)
    );
}

#[test]
fn a_command_key_carries_its_modifier() {
    let mut pressed = text("k");
    pressed.mods.cmd = true;
    assert_eq!(from_event(&pressed), Some(Chord::command(Key::Char('k'))));
    assert_eq!(
        command_for(Surface::Library, &pressed),
        Some(Command::ShowPalette)
    );
}

#[test]
fn shift_on_a_letter_is_folded_away_and_its_case_decides() {
    let mut pressed = text("J");
    pressed.mods.shift = true;
    assert_eq!(from_event(&pressed), Some(Chord::Key(Key::Char('j'))));
}

#[test]
fn a_word_or_a_key_with_no_binding_is_not_a_chord() {
    assert_eq!(from_event(&text("ab")), None);
    assert_eq!(from_event(&event(RuneKey::Tab)), None);
    assert_eq!(command_for(Surface::Library, &text("z")), None);
}

#[test]
fn every_surface_can_reach_the_panel_and_the_palette() {
    for surface in [
        Surface::Library,
        Surface::Editor,
        Surface::Session,
        Surface::Summary,
    ] {
        assert_eq!(
            command_for(surface, &text("?")),
            Some(Command::ShowShortcuts)
        );
        let mut palette = text("k");
        palette.mods.cmd = true;
        assert_eq!(command_for(surface, &palette), Some(Command::ShowPalette));
    }
}

#[test]
fn the_table_holds_no_conflict_once_events_are_folded() {
    assert!(typr_core::keymap::conflicts(bindings()).is_empty());
}

#[test]
fn the_readme_lists_every_binding() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../README.md");
    let readme = std::fs::read_to_string(path)
        .expect("the readme is readable")
        // A chord draws as a keycap in capitals; prose spells a letter in its own case.
        .to_lowercase();
    for binding in typr_core::keymap::bindings() {
        let chord = binding.chord.to_string().to_lowercase();
        assert!(
            readme.contains(&format!("`{chord}`")),
            "the readme never mentions {chord} on {:?}",
            binding.surface
        );
    }
}

/// A host that only holds the overlays, for the panel and the palette.
struct Overlays {
    surface: Surface,
    panel: panel::Panel,
    palette: palette::Palette,
    wide: bool,
    ran: Vec<Command>,
}

impl Overlays {
    fn on(surface: Surface) -> Overlays {
        Overlays {
            surface,
            panel: panel::Panel::default(),
            palette: palette::Palette::default(),
            wide: true,
            ran: Vec::new(),
        }
    }
}

impl Host for Overlays {
    fn surface(&self) -> Surface {
        self.surface
    }

    fn panel(&self) -> &panel::Panel {
        &self.panel
    }

    fn panel_mut(&mut self) -> &mut panel::Panel {
        &mut self.panel
    }

    fn palette(&self) -> &palette::Palette {
        &self.palette
    }

    fn palette_mut(&mut self) -> &mut palette::Palette {
        &mut self.palette
    }

    fn wide_enough(&self) -> bool {
        self.wide
    }

    fn run_command(&mut self, command: Command) -> bool {
        self.ran.push(command);
        true
    }
}

#[test]
fn the_panel_opens_and_closes_on_its_key() {
    let mut host = Overlays::on(Surface::Library);
    assert!(!host.panel.open);
    let command = command_for(Surface::Library, &press("?")).expect("the panel has a key");
    assert_eq!(command, Command::ShowShortcuts);
    host.panel.open = true;
    let mut layer = Vec::new();
    panel::build(&host, &mut layer);
    assert!(!layer.is_empty());
    host.panel.open = false;
    let mut layer = Vec::new();
    panel::build(&host, &mut layer);
    assert!(layer.is_empty());
}

#[test]
fn the_panel_filter_narrows_the_list() {
    let all = panel::rows(Surface::Library, "").count();
    let esc = panel::rows(Surface::Library, "esc").count();
    let save = panel::rows(Surface::Library, "save").count();
    assert!(all > esc, "the filter kept every row for {all} rows");
    assert_eq!(save, 0, "the library has no save binding");
}

#[test]
fn the_panel_names_the_chords_and_the_labels() {
    let rows: Vec<String> = panel::rows(Surface::Session, "shuffle")
        .map(|binding| format!("{} {}", binding.chord, binding.command.label()))
        .collect();
    assert_eq!(rows, ["R Shuffle the queue"]);
}

#[test]
fn a_keycap_is_drawn_only_when_the_window_has_room() {
    let mut host = Overlays::on(Surface::Library);
    assert_eq!(keycap(&host, Command::NewCard).as_deref(), Some("C"));
    assert_eq!(label(&host, Command::NewCard, "New card"), "New card  C");
    host.wide = false;
    assert_eq!(keycap(&host, Command::NewCard), None);
    assert_eq!(label(&host, Command::NewCard, "New card"), "New card");
}

#[test]
fn typing_narrows_and_enter_runs() {
    let mut host = Overlays::on(Surface::Library);
    palette::open(&mut host.palette);
    host.palette.filter.replace_text("command");
    let rows = palette::visible(Surface::Library, &host.palette.filter.text);
    assert_eq!(rows, [Command::ShowPalette]);
    assert!(palette::run(&mut host));
    assert_eq!(host.ran, [Command::ShowPalette]);
    assert!(!host.palette.open);
}

#[test]
fn a_session_command_is_not_offered_on_the_library() {
    let library = palette::visible(Surface::Library, "");
    assert!(!library.contains(&Command::GradeRemembered));
    let session = palette::visible(Surface::Session, "");
    assert!(session.contains(&Command::GradeRemembered));
}

#[test]
fn the_palette_moves_its_selection_with_the_arrows() {
    let mut host = Overlays::on(Surface::Library);
    palette::open(&mut host.palette);
    let rows = palette::visible(Surface::Library, "").len();
    palette::move_down(&mut host.palette, Surface::Library);
    palette::move_down(&mut host.palette, Surface::Library);
    assert_eq!(host.palette.index, 2);
    palette::move_up(&mut host.palette);
    assert_eq!(host.palette.index, 1);
    for _ in 0..rows + 3 {
        palette::move_down(&mut host.palette, Surface::Library);
    }
    assert_eq!(host.palette.index, rows - 1);
}

fn press(text: &str) -> KeyEvent {
    KeyEvent {
        key: rune::Key::Text(text.to_string()),
        mods: rune::Mods::default(),
    }
}
