//! The keymap panel: every binding of one surface, with its command's label and its chord.
//!
//! NSH delivers a key as a keystroke string (`"mod+k"`, `"escape"`). [`chord_from_keystroke`] turns
//! that into the same [`Chord`] the bindings table holds, so a key pressed in NSH is matched against
//! the one table. The panel opens and closes on the shortcut chord the table declares, filters as
//! the user types, and the palette runs the first command a typed query narrows to.

use nsh_core::prelude::*;
use typr_core::keymap::{Chord, Command, Key, Mods, Surface, bindings_of, chord_of};

pub use crate::ui::command::Mode;

/// Reads an NSH keystroke string into a chord. `mod` is Command on Apple platforms and Control
/// elsewhere, as NSH defines it; `None` for a stroke the table cannot hold.
pub fn chord_from_keystroke(stroke: &str, apple: bool) -> Option<Chord> {
    let mut mods = Mods::NONE;
    let mut key = None;
    for part in stroke.split('+') {
        match part.trim().to_ascii_lowercase().as_str() {
            "mod" => {
                if apple {
                    mods.cmd = true
                } else {
                    mods.ctrl = true
                }
            }
            "cmd" | "command" => mods.cmd = true,
            "ctrl" | "control" => mods.ctrl = true,
            "alt" | "option" => mods.alt = true,
            "shift" => mods.shift = true,
            "escape" | "esc" => key = Some(Key::Escape),
            "enter" | "return" => key = Some(Key::Enter),
            "space" => key = Some(Key::Space),
            "backspace" => key = Some(Key::Backspace),
            "left" => key = Some(Key::Left),
            "right" => key = Some(Key::Right),
            "home" => key = Some(Key::Home),
            "end" => key = Some(Key::End),
            "up" => key = Some(Key::Up),
            "down" => key = Some(Key::Down),
            single if single.chars().count() == 1 => key = single.chars().next().map(Key::Char),
            _ => return None,
        }
    }
    let key = key?;
    if mods.is_none() {
        Some(Chord::Key(key))
    } else {
        Some(Chord::Modified { key, mods })
    }
}

/// The key a shifted character is typed on: NSH names `?` as `shift+/`, so a table chord that
/// holds `?` must be written that way or `on_key` refuses it.
fn shift_base(c: char) -> Option<&'static str> {
    Some(match c {
        '!' => "1",
        '@' => "2",
        '#' => "3",
        '$' => "4",
        '%' => "5",
        '^' => "6",
        '&' => "7",
        '*' => "8",
        '(' => "9",
        ')' => "0",
        '_' => "-",
        '+' => "=",
        '{' => "[",
        '}' => "]",
        '|' => "\\",
        ':' => ";",
        '"' => "'",
        '<' => ",",
        '>' => ".",
        '?' => "/",
        '~' => "`",
        'A'..='Z' => {
            return Some(Box::leak(
                c.to_ascii_lowercase().to_string().into_boxed_str(),
            ));
        }
        _ => return None,
    })
}

/// A character as the keystroke NSH takes for it, with `shift+` when the character needs Shift.
fn char_keystroke(c: char, mods: Mods) -> String {
    let mut stroke = Vec::new();
    if mods.cmd {
        stroke.push("cmd".to_owned());
    }
    if mods.ctrl {
        stroke.push("ctrl".to_owned());
    }
    if mods.alt {
        stroke.push("alt".to_owned());
    }
    match shift_base(c) {
        Some(base) => {
            stroke.push("shift".to_owned());
            stroke.push(base.to_owned());
        }
        None => {
            if mods.shift {
                stroke.push("shift".to_owned());
            }
            stroke.push(c.to_string());
        }
    }
    stroke.join("+")
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
    match chord {
        Chord::Key(Key::Space) => Some("space".to_owned()),
        Chord::Key(Key::Enter) => Some("enter".to_owned()),
        Chord::Key(Key::Char(c)) => Some(char_keystroke(c, Mods::NONE)),
        Chord::Key(Key::Escape) => Some("escape".to_owned()),
        Chord::Key(Key::Left) => Some("left".to_owned()),
        Chord::Key(Key::Right) => Some("right".to_owned()),
        Chord::Key(Key::Up) => Some("up".to_owned()),
        Chord::Key(Key::Down) => Some("down".to_owned()),
        Chord::Key(Key::Backspace) => Some("backspace".to_owned()),
        Chord::Key(Key::Home) => Some("home".to_owned()),
        Chord::Key(Key::End) => Some("end".to_owned()),
        Chord::Modified {
            key: Key::Char(c),
            mods,
        } => Some(char_keystroke(c, mods)),
        _ => None,
    }
}

/// The items a command list shows for `surface`: one per binding, named by the command, labelled
/// by its noun, with its chord among the words the query matches and its keystroke for the row to
/// draw.
pub fn items(surface: Surface) -> Vec<nsh_base::CommandItem> {
    // One item per command, not per chord: a command may hold several chords (the arrows and the
    // letters both walk the library), and the list is chosen by the item's id.
    let mut listed = Vec::new();
    for binding in bindings_of(surface) {
        if let Some(item) = listed
            .iter_mut()
            .find(|item: &&mut nsh_base::CommandItem| item.id == binding.command.name())
        {
            let mut words: Vec<SharedString> = item.keywords.clone();
            words.push(binding.chord.to_string().into());
            *item = item.clone().keywords(words);
            continue;
        }
        let mut item = nsh_base::CommandItem::new(binding.command.name(), binding.command.label())
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
