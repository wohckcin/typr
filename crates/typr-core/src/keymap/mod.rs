//! What each key does, in one table.
//!
//! Dispatch, the shortcut panel, the keycap hints and the palette all read this table, so a chord cannot
//! mean one thing when it is pressed and another when it is drawn. The types here name no toolkit crate:
//! `Chord::from_event` lives in the desktop crate, which is where `rune::KeyEvent` exists.

use std::fmt;

/// Where a chord applies. A chord may differ between surfaces; it may not repeat inside one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Surface {
    Library,
    Editor,
    Session,
    Summary,
    Settings,
}

impl Surface {
    pub fn name(self) -> &'static str {
        match self {
            Surface::Library => "Library",
            Surface::Editor => "Editor",
            Surface::Session => "Review",
            Surface::Summary => "Summary",
            Surface::Settings => "Settings",
        }
    }
}

/// A thing the app can be asked to do. Whether it can be done is the surface's business.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Command {
    StartDueSession,
    StartNewSession,
    NewCard,
    Leave,
    Save,
    ToggleHistory,
    Reveal,
    GradeRemembered,
    GradeForgot,
    Forward,
    Back,
    Shuffle,
    ShowShortcuts,
    ShowPalette,
    AddRectangle,
    WiderRectangle,
    NarrowerRectangle,
    TallerRectangle,
    ShorterRectangle,
    CramSession,
    Settings,
    SelectionNext,
    SelectionPrevious,
    EditCard,
    PreviewCard,
    DeleteCard,
    AddCardBelow,
    AddCardAbove,
    MoveCardUp,
    MoveCardDown,
}

impl Command {
    pub fn label(self) -> &'static str {
        match self {
            Command::StartDueSession => "Review cards that are due",
            Command::StartNewSession => "Review new cards",
            Command::NewCard => "New card",
            Command::Leave => "Back to the library",
            Command::Save => "Save the card",
            Command::ToggleHistory => "Show this card's history",
            Command::Reveal => "Reveal the side",
            Command::GradeRemembered => "Remembered",
            Command::GradeForgot => "Forgot",
            Command::Forward => "Forward",
            Command::Back => "Back",
            Command::Shuffle => "Shuffle the queue",
            Command::ShowShortcuts => "Keyboard shortcuts",
            Command::ShowPalette => "Command palette",
            Command::AddRectangle => "Add a rectangle",
            Command::WiderRectangle => "Widen the rectangle",
            Command::NarrowerRectangle => "Narrow the rectangle",
            Command::TallerRectangle => "Make the rectangle taller",
            Command::ShorterRectangle => "Make the rectangle shorter",
            Command::CramSession => "Cram the cards the filter selects",
            Command::Settings => "Settings",
            Command::SelectionNext => "Select the next card",
            Command::SelectionPrevious => "Select the previous card",
            Command::EditCard => "Edit the selected card",
            Command::PreviewCard => "Preview the selected card",
            Command::DeleteCard => "Move the selected card to the trash",
            Command::AddCardBelow => "Add a card below",
            Command::AddCardAbove => "Add a card above",
            Command::MoveCardUp => "Move the card up its deck",
            Command::MoveCardDown => "Move the card down its deck",
        }
    }
}

/// A key, in terms a table can be written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Key {
    /// A letter or punctuation key, always recorded lowercase.
    Char(char),
    Space,
    Enter,
    Escape,
    Left,
    Right,
    Up,
    Down,
    Backspace,
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Key::Char(' ') => f.write_str("Space"),
            Key::Char(c) => write!(f, "{}", c.to_ascii_uppercase()),
            Key::Space => f.write_str("Space"),
            Key::Enter => f.write_str("Enter"),
            Key::Escape => f.write_str("Esc"),
            Key::Left => f.write_str("Left"),
            Key::Right => f.write_str("Right"),
            Key::Up => f.write_str("Up"),
            Key::Down => f.write_str("Down"),
            Key::Backspace => f.write_str("Backspace"),
        }
    }
}

/// Modifier flags for a chord that needs them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Mods {
    pub cmd: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

impl Mods {
    pub const NONE: Mods = Mods {
        cmd: false,
        ctrl: false,
        alt: false,
        shift: false,
    };
    pub const COMMAND: Mods = Mods {
        cmd: true,
        ..Mods::NONE
    };

    pub fn is_none(self) -> bool {
        self == Mods::NONE
    }
}

/// One key press, with or without modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Chord {
    Key(Key),
    Modified { key: Key, mods: Mods },
}

impl Chord {
    pub const fn key(key: Key) -> Chord {
        Chord::Key(key)
    }

    pub const fn command(key: Key) -> Chord {
        Chord::Modified {
            key,
            mods: Mods::COMMAND,
        }
    }

    pub const fn alt(key: Key) -> Chord {
        Chord::Modified {
            key,
            mods: Mods {
                alt: true,
                ..Mods::NONE
            },
        }
    }

    pub fn key_of(self) -> Key {
        match self {
            Chord::Key(key) | Chord::Modified { key, .. } => key,
        }
    }

    pub fn mods_of(self) -> Mods {
        match self {
            Chord::Key(_) => Mods::NONE,
            Chord::Modified { mods, .. } => mods,
        }
    }
}

impl fmt::Display for Chord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Chord::Key(key) => write!(f, "{key}"),
            Chord::Modified { key, mods } => {
                if mods.cmd {
                    f.write_str("⌘")?;
                }
                if mods.ctrl {
                    f.write_str("Ctrl+")?;
                }
                if mods.alt {
                    f.write_str("Alt+")?;
                }
                if mods.shift && !matches!(key, Key::Char(_)) {
                    f.write_str("Shift+")?;
                }
                write!(f, "{key}")
            }
        }
    }
}

/// One row of the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Binding {
    pub surface: Surface,
    pub command: Command,
    pub chord: Chord,
}

/// The table. Every binding the app has, and nowhere else.
const TABLE: &[Binding] = &[
    row(
        Surface::Library,
        Command::StartDueSession,
        Chord::key(Key::Char('s')),
    ),
    row(
        Surface::Library,
        Command::StartNewSession,
        Chord::key(Key::Char('n')),
    ),
    row(
        Surface::Library,
        Command::NewCard,
        Chord::key(Key::Char('c')),
    ),
    row(
        Surface::Library,
        Command::ShowPalette,
        Chord::command(Key::Char('k')),
    ),
    row(
        Surface::Library,
        Command::ShowShortcuts,
        Chord::key(Key::Char('?')),
    ),
    row(Surface::Library, Command::EditCard, Chord::key(Key::Enter)),
    row(
        Surface::Library,
        Command::PreviewCard,
        Chord::key(Key::Char('p')),
    ),
    row(
        Surface::Library,
        Command::DeleteCard,
        Chord::key(Key::Backspace),
    ),
    row(
        Surface::Library,
        Command::AddCardBelow,
        Chord::key(Key::Char('o')),
    ),
    row(
        Surface::Library,
        Command::AddCardAbove,
        Chord::key(Key::Char('a')),
    ),
    row(Surface::Library, Command::MoveCardUp, Chord::alt(Key::Up)),
    row(
        Surface::Library,
        Command::MoveCardDown,
        Chord::alt(Key::Down),
    ),
    row(Surface::Editor, Command::Leave, Chord::key(Key::Escape)),
    row(Surface::Editor, Command::Save, Chord::key(Key::Char('s'))),
    row(
        Surface::Editor,
        Command::ToggleHistory,
        Chord::key(Key::Char('h')),
    ),
    row(
        Surface::Editor,
        Command::ShowPalette,
        Chord::command(Key::Char('k')),
    ),
    row(
        Surface::Editor,
        Command::ShowShortcuts,
        Chord::key(Key::Char('?')),
    ),
    row(
        Surface::Editor,
        Command::AddRectangle,
        Chord::alt(Key::Char('n')),
    ),
    row(
        Surface::Editor,
        Command::WiderRectangle,
        Chord::alt(Key::Right),
    ),
    row(
        Surface::Editor,
        Command::NarrowerRectangle,
        Chord::alt(Key::Left),
    ),
    row(
        Surface::Editor,
        Command::TallerRectangle,
        Chord::alt(Key::Down),
    ),
    row(
        Surface::Editor,
        Command::ShorterRectangle,
        Chord::alt(Key::Up),
    ),
    row(Surface::Session, Command::Reveal, Chord::key(Key::Space)),
    row(Surface::Session, Command::Reveal, Chord::key(Key::Enter)),
    row(
        Surface::Session,
        Command::GradeRemembered,
        Chord::key(Key::Char('j')),
    ),
    row(
        Surface::Session,
        Command::GradeForgot,
        Chord::key(Key::Char('f')),
    ),
    row(Surface::Session, Command::Forward, Chord::key(Key::Right)),
    row(Surface::Session, Command::Back, Chord::key(Key::Left)),
    row(
        Surface::Session,
        Command::Shuffle,
        Chord::key(Key::Char('r')),
    ),
    row(Surface::Session, Command::Leave, Chord::key(Key::Escape)),
    row(
        Surface::Session,
        Command::ShowPalette,
        Chord::command(Key::Char('k')),
    ),
    row(
        Surface::Session,
        Command::ShowShortcuts,
        Chord::key(Key::Char('?')),
    ),
    row(Surface::Summary, Command::Leave, Chord::key(Key::Escape)),
    row(
        Surface::Summary,
        Command::ShowPalette,
        Chord::command(Key::Char('k')),
    ),
    row(
        Surface::Summary,
        Command::ShowShortcuts,
        Chord::key(Key::Char('?')),
    ),
    row(
        Surface::Library,
        Command::CramSession,
        Chord::key(Key::Char('x')),
    ),
    row(
        Surface::Library,
        Command::SelectionNext,
        Chord::key(Key::Down),
    ),
    row(
        Surface::Library,
        Command::SelectionNext,
        Chord::key(Key::Char('j')),
    ),
    row(
        Surface::Library,
        Command::SelectionPrevious,
        Chord::key(Key::Up),
    ),
    row(
        Surface::Library,
        Command::SelectionPrevious,
        Chord::key(Key::Char('k')),
    ),
    row(
        Surface::Library,
        Command::Settings,
        Chord::key(Key::Char(',')),
    ),
    row(
        Surface::Session,
        Command::Settings,
        Chord::key(Key::Char(',')),
    ),
    row(Surface::Settings, Command::Leave, Chord::key(Key::Escape)),
];

/// The table, as a slice.
pub fn bindings() -> &'static [Binding] {
    TABLE
}

const fn row(surface: Surface, command: Command, chord: Chord) -> Binding {
    Binding {
        surface,
        command,
        chord,
    }
}

/// The command a chord runs on a surface, if any.
pub fn dispatch(surface: Surface, chord: Chord) -> Option<Command> {
    table()
        .iter()
        .find(|binding| binding.surface == surface && binding.chord == chord)
        .map(|binding| binding.command)
}

/// The chord bound to a command on a surface, for drawing a keycap.
pub fn chord_of(surface: Surface, command: Command) -> Option<Chord> {
    table()
        .iter()
        .find(|binding| binding.surface == surface && binding.command == command)
        .map(|binding| binding.chord)
}

/// Every binding of one surface, in table order.
pub fn bindings_of(surface: Surface) -> impl Iterator<Item = &'static Binding> {
    table()
        .iter()
        .filter(move |binding| binding.surface == surface)
}

fn table() -> &'static [Binding] {
    bindings()
}

/// Pairs of rows that bind one chord twice on one surface. Must be empty; a test says so.
pub fn conflicts(bindings: &[Binding]) -> Vec<(Surface, Chord, Command, Command)> {
    let mut found = Vec::new();
    for (index, first) in bindings.iter().enumerate() {
        for second in &bindings[index + 1..] {
            if first.surface == second.surface && first.chord == second.chord {
                found.push((first.surface, first.chord, first.command, second.command));
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The chords the app answered before this table existed, transcribed from the three `match` arms in
    /// `crates/typr/src/main.rs` and `crates/typr/src/study.rs`.
    const TODAY: &[(Surface, Chord, Command)] = &[
        (
            Surface::Library,
            Chord::key(Key::Char('s')),
            Command::StartDueSession,
        ),
        (
            Surface::Library,
            Chord::key(Key::Char('n')),
            Command::StartNewSession,
        ),
        (Surface::Editor, Chord::key(Key::Escape), Command::Leave),
        (
            Surface::Editor,
            Chord::key(Key::Char('h')),
            Command::ToggleHistory,
        ),
        (Surface::Editor, Chord::key(Key::Char('s')), Command::Save),
        (Surface::Session, Chord::key(Key::Space), Command::Reveal),
        (Surface::Session, Chord::key(Key::Enter), Command::Reveal),
        (
            Surface::Session,
            Chord::key(Key::Char('j')),
            Command::GradeRemembered,
        ),
        (
            Surface::Session,
            Chord::key(Key::Char('f')),
            Command::GradeForgot,
        ),
        (Surface::Session, Chord::key(Key::Right), Command::Forward),
        (Surface::Session, Chord::key(Key::Left), Command::Back),
        (
            Surface::Session,
            Chord::key(Key::Char('r')),
            Command::Shuffle,
        ),
        (Surface::Session, Chord::key(Key::Escape), Command::Leave),
        (Surface::Summary, Chord::key(Key::Escape), Command::Leave),
    ];

    #[test]
    fn every_chord_that_exists_today_is_in_the_table() {
        for (surface, chord, command) in TODAY {
            assert_eq!(
                dispatch(*surface, *chord),
                Some(*command),
                "{} {} is not bound to {command:?} any more",
                surface.name(),
                chord
            );
        }
    }

    #[test]
    fn no_chord_is_bound_twice_on_one_surface() {
        assert_eq!(conflicts(bindings()), Vec::new());
    }

    #[test]
    fn a_duplicate_is_reported_with_both_commands() {
        let duplicate = [
            row(
                Surface::Library,
                Command::NewCard,
                Chord::key(Key::Char('c')),
            ),
            row(Surface::Library, Command::Save, Chord::key(Key::Char('c'))),
        ];
        let found = conflicts(&duplicate);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, Surface::Library);
        assert_eq!(found[0].1, Chord::key(Key::Char('c')));
        assert_eq!((found[0].2, found[0].3), (Command::NewCard, Command::Save));
    }

    #[test]
    fn the_same_chord_may_differ_between_surfaces() {
        let table = [
            row(
                Surface::Session,
                Command::Reveal,
                Chord::key(Key::Char('r')),
            ),
            row(
                Surface::Library,
                Command::StartNewSession,
                Chord::key(Key::Char('r')),
            ),
        ];
        assert_eq!(conflicts(&table), Vec::new());
        assert_eq!(
            dispatch(Surface::Session, Chord::key(Key::Char('r'))),
            Some(Command::Shuffle)
        );
        assert_eq!(dispatch(Surface::Library, Chord::key(Key::Char('r'))), None);
    }

    #[test]
    fn a_chord_the_table_does_not_bind_does_nothing() {
        assert_eq!(dispatch(Surface::Library, Chord::key(Key::Char('z'))), None);
        assert_eq!(
            dispatch(Surface::Library, Chord::command(Key::Char('k'))),
            Some(Command::ShowPalette)
        );
    }

    #[test]
    fn a_keycap_is_found_back_from_its_command() {
        assert_eq!(
            chord_of(Surface::Session, Command::Reveal),
            Some(Chord::key(Key::Space))
        );
        assert_eq!(chord_of(Surface::Library, Command::Save), None);
    }

    #[test]
    fn a_chord_prints_the_way_a_keycap_reads() {
        assert_eq!(Chord::key(Key::Space).to_string(), "Space");
        assert_eq!(Chord::key(Key::Escape).to_string(), "Esc");
        assert_eq!(Chord::key(Key::Char('j')).to_string(), "J");
        assert_eq!(Chord::command(Key::Char('k')).to_string(), "⌘K");
    }

    #[test]
    fn a_surface_lists_its_own_bindings() {
        for surface in [Surface::Library, Surface::Session] {
            let listed: Vec<&Binding> = bindings_of(surface).collect();
            assert!(!listed.is_empty(), "{surface:?} has no bindings");
            assert!(listed.iter().all(|binding| binding.surface == surface));
        }
    }
}
