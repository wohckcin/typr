//! The keymap panel: every binding of one surface, with its command's label and its chord.
//!
//! NSH delivers a key as a keystroke string (`"mod+k"`, `"escape"`). [`chord_from_keystroke`] turns
//! that into the same [`Chord`] the bindings table holds, so a key pressed in NSH is matched against
//! the one table.

use nsh_core::prelude::*;
use typr_core::keymap::{Binding, Chord, Key, Mods, Surface, bindings_of};

use crate::ui::{status::keycap, surface, tokens};

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

/// The rows the panel shows for `surface`: the command's label and the chord it is bound to.
pub fn rows(surface: Surface) -> Vec<(&'static str, String)> {
    bindings_of(surface).map(|b: &Binding| (b.command.label(), b.chord.to_string())).collect()
}

/// The panel for `surface`.
pub fn keymap_panel(surface: Surface) -> impl IntoElement {
    let mut list = view().gap_2();
    for (label, chord) in rows(surface) {
        list = list.child(surface::card(
            view()
                .flex_row()
                .justify_between()
                .items_center()
                .child(text(label).text_sm().text_color(move || tokens::foreground()))
                .child(keycap(chord)),
        ));
    }
    view().flex_1().p_4().child(list)
}
