//! Turning a toolkit key event into a table chord, and running what the table names.

pub mod palette;
pub mod panel;

use rune::{El, Key as RuneKey, KeyEvent};
use typr_core::keymap::{Chord, Command, Key, Mods, Surface};

/// Below this width the app draws a control's label without its keycap.
pub const KEYCAP_MIN_WIDTH: f32 = 520.0;

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

/// What the overlays and the keycaps need from the app.
pub trait Host: 'static {
    /// The surface the keyboard is on.
    fn surface(&self) -> Surface;
    /// The shortcut panel's state.
    fn panel(&self) -> &panel::Panel;
    /// The shortcut panel's state, to change.
    fn panel_mut(&mut self) -> &mut panel::Panel;
    /// The palette's state.
    fn palette(&self) -> &palette::Palette;
    /// The palette's state, to change.
    fn palette_mut(&mut self) -> &mut palette::Palette;
    /// Whether the window has room for a keycap beside a label.
    fn wide_enough(&self) -> bool;
    /// Run a command on the current surface. Answers whether it was used. The `Tx` is here
    /// because a few commands end in focus or a dialog rather than in state, and the palette must
    /// reach them the same way a key does.
    fn run_command(&mut self, command: Command, tx: &mut rune::Tx) -> bool;
}

/// The keycap a control should draw, or `None` when the window is too narrow for one.
pub fn keycap<S: Host + ?Sized>(s: &S, command: Command) -> Option<String> {
    if !s.wide_enough() {
        return None;
    }
    typr_core::keymap::chord_of(s.surface(), command).map(|chord| chord.to_string())
}

/// A control's label, with its keycap appended when there is room for one.
pub fn label<S: Host + ?Sized>(s: &S, command: Command, label: &str) -> String {
    match keycap(s, command) {
        Some(chord) => format!("{label}  {chord}"),
        None => label.to_string(),
    }
}

/// The chord a command answers on a surface.
pub fn chord(surface: Surface, command: Command) -> Option<Chord> {
    typr_core::keymap::chord_of(surface, command)
}

/// The panel and the palette, for a `rune_kit` overlay layer.
pub fn layer<S: Host>(s: &S, layer: &mut Vec<El>) {
    let mut overlays = Vec::new();
    panel::build(s, &mut overlays);
    palette::build(s, &mut overlays);
    layer.extend(overlays);
}

/// A keycap chip for a chord.
pub fn kbd(chord: Chord) -> El {
    static KIT: std::sync::LazyLock<rune_kit::Kit> =
        std::sync::LazyLock::new(|| rune_kit::Kit::neutral(true));
    rune_kit::Badge::new(chord.to_string())
        .variant(rune_kit::BadgeVariant::Subtle)
        .build(&KIT)
}
