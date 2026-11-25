//! The keymap panel: every binding of one surface, with its command's label and its chord.
//!
//! NSH takes a key as a keystroke string (`"cmd+k"`, `"escape"`). [`keystroke`] turns a [`Chord`] of
//! the bindings table into NSH's own [`Keystroke`], so `on_key` is registered from the one table.
//! The panel opens and closes on the shortcut chord the table declares, filters as
//! the user types, and the palette runs the first command a typed query narrows to.

use nsh_core::keymap::{Key as NshKey, Keystroke, Modifiers};
use nsh_core::prelude::*;
use typr_core::keymap::{Chord, Command, Key, Mods, Surface, bindings_of, chord_of};

pub use crate::ui::command::{CommandItem, Mode};

/// The key a shifted character is typed on: NSH names `?` as `shift+/`, so a table chord that
/// holds `?` must be written that way or `on_key` refuses it.
const SHIFTED: &[(char, char)] = &[
    ('!', '1'),
    ('@', '2'),
    ('#', '3'),
    ('$', '4'),
    ('%', '5'),
    ('^', '6'),
    ('&', '7'),
    ('*', '8'),
    ('(', '9'),
    (')', '0'),
    ('_', '-'),
    ('+', '='),
    ('{', '['),
    ('}', ']'),
    ('|', '\\'),
    (':', ';'),
    ('"', '\''),
    ('<', ','),
    ('>', '.'),
    ('?', '/'),
    ('~', '`'),
];

/// The base key of a character that needs Shift (`?` is `/`, `A` is `a`).
fn shift_base(c: char) -> Option<char> {
    if c.is_ascii_uppercase() {
        return Some(c.to_ascii_lowercase());
    }
    SHIFTED.iter().find(|(s, _)| *s == c).map(|&(_, base)| base)
}

/// Maps one keymap [`Key`] to NSH's [`NshKey`], reporting whether Shift must be added.
fn map_key(key: Key) -> (NshKey, bool) {
    match key {
        Key::Char(c) => match shift_base(c) {
            Some(base) => (NshKey::Char(base), true),
            None => (NshKey::Char(c), false),
        },
        Key::Space => (NshKey::Char(' '), false),
        Key::Enter => (NshKey::Enter, false),
        Key::Escape => (NshKey::Escape, false),
        Key::Left => (NshKey::Left, false),
        Key::Right => (NshKey::Right, false),
        Key::Up => (NshKey::Up, false),
        Key::Down => (NshKey::Down, false),
        Key::Home => (NshKey::Home, false),
        Key::End => (NshKey::End, false),
        Key::Backspace => (NshKey::Backspace, false),
    }
}

/// One table chord as NSH's keystroke, with Shift added for a character that needs it.
pub fn keystroke(chord: Chord) -> Option<Keystroke> {
    let (key, mods) = match chord {
        Chord::Key(key) => (key, Mods::NONE),
        Chord::Modified { key, mods } => (key, mods),
    };
    let (key, shifted) = map_key(key);
    let modifiers = Modifiers {
        ctrl: mods.ctrl,
        alt: mods.alt,
        shift: mods.shift || shifted,
        cmd: mods.cmd,
        ..Modifiers::default()
    };
    Some(Keystroke { modifiers, key })
}

/// The keystroke string NSH's `on_key` takes for a command on `surface`, from the table.
pub fn keystroke_of(surface: Surface, command: Command) -> Option<String> {
    keystroke_of_chord(chord_of(surface, command)?)
}

/// Every keystroke `command` is bound to on `surface`.
pub fn keystrokes_of(surface: Surface, command: Command) -> Vec<String> {
    typr_core::keymap::chords_of(surface, command)
        .into_iter()
        .filter_map(keystroke_of_chord)
        .collect()
}

/// One chord as the keystroke string `on_key` takes.
pub fn keystroke_of_chord(chord: Chord) -> Option<String> {
    keystroke(chord).map(|k| k.to_string())
}

/// The items a command list shows for `surface`: one per binding, named by the command, labelled
/// by its noun, with its chord among the words the query matches and its keystroke for the row to
/// draw.
pub fn items(surface: Surface) -> Vec<CommandItem> {
    // One item per command, not per chord: a command may hold several chords (the arrows and the
    // letters both walk the library), and the list is chosen by the item's id.
    let mut listed = Vec::new();
    for binding in bindings_of(surface) {
        if let Some(item) = listed
            .iter_mut()
            .find(|item: &&mut CommandItem| item.id == binding.command.name())
        {
            let mut words: Vec<SharedString> = item.keywords.clone();
            words.push(binding.chord.to_string().into());
            *item = item.clone().keywords(words);
            continue;
        }
        let mut item = CommandItem::new(binding.command.name(), binding.command.label())
            .keywords([binding.chord.to_string()])
            .disabled(!binding.command.runnable());
        if let Some(keys) = keystroke_of_chord(binding.chord) {
            item = item.shortcut(keys);
        }
        listed.push(item);
    }
    listed
}

/// The command a palette item names, if the table holds it.
pub fn command_of(id: &str) -> Option<Command> {
    Command::from_name(id)
}

/// The command list for `surface`: the palette runs the chosen command, the shortcut panel lists
/// the same bindings without running any.
pub fn panel(
    surface: Surface,
    mode: Mode,
    on_run: impl Fn(&SharedString) + Clone + 'static,
    on_cancel: impl Fn() + Clone + 'static,
) -> impl IntoElement {
    let items = items(surface);
    crate::ui::command::command(items, "Run a command", mode, on_run, on_cancel)
}
