//! Turning a toolkit key event into a table chord, and running what the table names.

use rune::{Key as RuneKey, KeyEvent};
use typr_core::keymap::{Chord, Key, Mods, Surface};

/// The chord an event presses, or `None` when the event carries nothing the table can name.
///
/// The table cannot be extended from here — `Chord` belongs to `typr-core` and Rust forbids an inherent
/// impl on a foreign type — so this is the free function the task called `Chord::from_event`. Letters are
/// folded to lowercase before they reach the table, which is how `S` and `s` both save.
pub fn from_event(event: &KeyEvent) -> Option<Chord> {
    let key = match &event.key {
        RuneKey::Text(text) => letter(text)?,
        RuneKey::Enter => Key::Enter,
        RuneKey::Escape => Key::Escape,
        RuneKey::Left => Key::Left,
        RuneKey::Right => Key::Right,
        RuneKey::Up => Key::Up,
        RuneKey::Down => Key::Down,
        RuneKey::Backspace => Key::Backspace,
        _ => return None,
    };
    let mods = Mods {
        cmd: event.mods.cmd,
        ctrl: event.mods.ctrl,
        alt: event.mods.alt,
        shift: event.mods.shift && !matches!(key, Key::Char(_)),
    };
    Some(if mods.is_none() {
        Chord::Key(key)
    } else {
        Chord::Modified { key, mods }
    })
}

fn letter(text: &str) -> Option<Key> {
    let mut characters = text.chars();
    let first = characters.next()?;
    if characters.next().is_some() {
        return None;
    }
    Some(match first {
        ' ' => Key::Space,
        '\n' | '\r' => Key::Enter,
        other => Key::Char(other.to_ascii_lowercase()),
    })
}

/// The command a key event runs on a surface.
pub fn command_for(surface: Surface, event: &KeyEvent) -> Option<typr_core::keymap::Command> {
    let chord = from_event(event)?;
    typr_core::keymap::dispatch(surface, chord)
}

#[cfg(test)]
mod tests;
