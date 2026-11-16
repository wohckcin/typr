//! The keymap panel: every binding of one surface, with its command's label and its chord.
//!
//! NSH delivers a key as a keystroke string (`"mod+k"`, `"escape"`). [`chord_from_keystroke`] turns
//! that into the same [`Chord`] the bindings table holds, so a key pressed in NSH is matched against
//! the one table. The panel opens and closes on the shortcut chord the table declares, filters as
//! the user types, and the palette runs the first command a typed query narrows to.

use nsh_core::prelude::*;
use typr_core::keymap::{Binding, Chord, Command, Key, Mods, Surface, bindings_of, chord_of};

use crate::ui::{
    input::{field, submit_field},
    status::keycap,
    surface, tokens,
};

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
        Chord::Modified {
            key: Key::Char(c),
            mods,
        } => Some(char_keystroke(c, mods)),
        _ => None,
    }
}

/// The rows the panel shows for `surface`: the command's label and the chord it is bound to.
pub fn rows(surface: Surface) -> Vec<(&'static str, String)> {
    bindings_of(surface)
        .map(|b: &Binding| (b.command.label(), b.chord.to_string()))
        .collect()
}

/// The rows whose label contains `query`, ignoring case; the whole list when it is empty.
pub fn visible_rows(surface: Surface, query: &str) -> Vec<(&'static str, String)> {
    let needle = query.trim().to_lowercase();
    rows(surface)
        .into_iter()
        .filter(|(label, _)| needle.is_empty() || label.to_lowercase().contains(&needle))
        .collect()
}

fn row_views(rows: Vec<(&'static str, String)>) -> impl IntoElement {
    let mut list = view().gap_2();
    if rows.is_empty() {
        list = list.child(
            text("No matching command")
                .text_sm()
                .text_color(tokens::muted_foreground),
        );
    }
    for (label, chord) in rows {
        list = list.child(surface::card(
            view()
                .flex_row()
                .justify_between()
                .items_center()
                .child(text(label).text_sm().text_color(tokens::foreground))
                .child(keycap().chord(chord)),
        ));
    }
    list
}

/// The panel for `surface`. It opens and closes on the shortcut chord the table binds, and its list
/// narrows to the commands that match what is typed.
pub fn keymap_panel(surface: Surface) -> impl IntoElement {
    let open = signal(false);
    let query = signal(String::new());
    if let Some(key) = keystroke_of(surface, Command::ShowShortcuts) {
        on_key(&key, move |_| open.update(|shown| *shown = !*shown));
    }
    view().flex_1().p_4().child(dynamic(move || {
        if !open.get() {
            return view().into_element();
        }
        view()
            .gap_3()
            .child(
                field()
                    .placeholder("Filter commands")
                    .value(move || SharedString::from(query.get()))
                    .on_change(move |text: &String| query.set(text.to_string())),
            )
            .child(dynamic(move || {
                row_views(visible_rows(surface, &query.get())).into_element()
            }))
            .into_element()
    }))
}

/// The palette for `surface`: typing a query narrows the surface's commands, and Enter runs the
/// first one left. A command the surface does not bind is never offered, because the list comes
/// from the one binding table.
pub fn palette(
    surface: Surface,
    on_run: impl Fn(&'static str) + Clone + 'static,
) -> impl IntoElement {
    let query = signal(String::new());
    let typing = query;
    let run = on_run.clone();
    view()
        .flex_1()
        .gap_3()
        .p_4()
        .child(submit_field("Run a command", query, move |_| {
            if let Some((label, _)) = visible_rows(surface, &typing.get_untracked())
                .into_iter()
                .next()
            {
                run(label);
            }
        }))
        .child(dynamic(move || {
            let rows = visible_rows(surface, &query.get());
            let mut list = view().gap_2();
            if rows.is_empty() {
                list = list.child(
                    text("No matching command")
                        .text_sm()
                        .text_color(tokens::muted_foreground),
                );
            }
            for (label, chord) in rows {
                list = list.child(surface::card(
                    view()
                        .flex_row()
                        .justify_between()
                        .items_center()
                        .child(text(label).text_sm().text_color(tokens::foreground))
                        .child(keycap().chord(chord)),
                ));
            }
            list.into_element()
        }))
}
