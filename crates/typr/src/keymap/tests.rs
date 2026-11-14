use super::*;
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
