//! What each key does, in one table.
//!
//! Dispatch, the shortcut panel, the keycap hints and the palette all read this table, so a chord cannot
//! mean one thing when it is pressed and another when it is drawn. The types here name no toolkit crate:
//! `Chord::from_event` lives in the desktop crate, which is where the toolkit's key event exists.

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
#[repr(usize)]
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
    SelectionFirst,
    SelectionLast,
    FocusSearch,
    EditCard,
    PreviewCard,
    DeleteCard,
    AddCardBelow,
    AddCardAbove,
    MoveCardUp,
    MoveCardDown,
}

struct CommandInfo {
    command: Command,
    name: &'static str,
    label: &'static str,
    runnable: bool,
}

const COMMANDS: &[CommandInfo] = &[
    CommandInfo {
        command: Command::StartDueSession,
        name: "start-due-session",
        label: "Review cards that are due",
        runnable: true,
    },
    CommandInfo {
        command: Command::StartNewSession,
        name: "start-new-session",
        label: "Review new cards",
        runnable: true,
    },
    CommandInfo {
        command: Command::NewCard,
        name: "new-card",
        label: "New card",
        runnable: true,
    },
    CommandInfo {
        command: Command::Leave,
        name: "leave",
        label: "Back to the library",
        runnable: true,
    },
    CommandInfo {
        command: Command::Save,
        name: "save",
        label: "Save the card",
        runnable: true,
    },
    CommandInfo {
        command: Command::ToggleHistory,
        name: "toggle-history",
        label: "Show this card's history",
        runnable: false,
    },
    CommandInfo {
        command: Command::Reveal,
        name: "reveal",
        label: "Reveal the side",
        runnable: true,
    },
    CommandInfo {
        command: Command::GradeRemembered,
        name: "grade-remembered",
        label: "Remembered",
        runnable: true,
    },
    CommandInfo {
        command: Command::GradeForgot,
        name: "grade-forgot",
        label: "Forgot",
        runnable: true,
    },
    CommandInfo {
        command: Command::Forward,
        name: "forward",
        label: "Forward",
        runnable: true,
    },
    CommandInfo {
        command: Command::Back,
        name: "back",
        label: "Back",
        runnable: true,
    },
    CommandInfo {
        command: Command::Shuffle,
        name: "shuffle",
        label: "Shuffle the queue",
        runnable: true,
    },
    CommandInfo {
        command: Command::ShowShortcuts,
        name: "show-shortcuts",
        label: "Keyboard shortcuts",
        runnable: true,
    },
    CommandInfo {
        command: Command::ShowPalette,
        name: "show-palette",
        label: "Command palette",
        runnable: true,
    },
    CommandInfo {
        command: Command::AddRectangle,
        name: "add-rectangle",
        label: "Add a rectangle",
        runnable: true,
    },
    CommandInfo {
        command: Command::WiderRectangle,
        name: "wider-rectangle",
        label: "Widen the rectangle",
        runnable: true,
    },
    CommandInfo {
        command: Command::NarrowerRectangle,
        name: "narrower-rectangle",
        label: "Narrow the rectangle",
        runnable: true,
    },
    CommandInfo {
        command: Command::TallerRectangle,
        name: "taller-rectangle",
        label: "Make the rectangle taller",
        runnable: true,
    },
    CommandInfo {
        command: Command::ShorterRectangle,
        name: "shorter-rectangle",
        label: "Make the rectangle shorter",
        runnable: true,
    },
    CommandInfo {
        command: Command::CramSession,
        name: "cram-session",
        label: "Cram the cards the filter selects",
        runnable: false,
    },
    CommandInfo {
        command: Command::Settings,
        name: "settings",
        label: "Settings",
        runnable: true,
    },
    CommandInfo {
        command: Command::SelectionNext,
        name: "selection-next",
        label: "Select the next card",
        runnable: true,
    },
    CommandInfo {
        command: Command::SelectionPrevious,
        name: "selection-previous",
        label: "Select the previous card",
        runnable: true,
    },
    CommandInfo {
        command: Command::SelectionFirst,
        name: "selection-first",
        label: "Select the first card",
        runnable: true,
    },
    CommandInfo {
        command: Command::SelectionLast,
        name: "selection-last",
        label: "Select the last card",
        runnable: true,
    },
    CommandInfo {
        command: Command::FocusSearch,
        name: "focus-search",
        label: "Focus the search field",
        runnable: true,
    },
    CommandInfo {
        command: Command::EditCard,
        name: "edit-card",
        label: "Edit the selected card",
        runnable: true,
    },
    CommandInfo {
        command: Command::PreviewCard,
        name: "preview-card",
        label: "Preview the selected card",
        runnable: false,
    },
    CommandInfo {
        command: Command::DeleteCard,
        name: "delete-card",
        label: "Move the selected card to the trash",
        runnable: true,
    },
    CommandInfo {
        command: Command::AddCardBelow,
        name: "add-card-below",
        label: "Add a card below",
        runnable: false,
    },
    CommandInfo {
        command: Command::AddCardAbove,
        name: "add-card-above",
        label: "Add a card above",
        runnable: false,
    },
    CommandInfo {
        command: Command::MoveCardUp,
        name: "move-card-up",
        label: "Move the card up its deck",
        runnable: false,
    },
    CommandInfo {
        command: Command::MoveCardDown,
        name: "move-card-down",
        label: "Move the card down its deck",
        runnable: false,
    },
];

impl Command {
    pub fn label(self) -> &'static str {
        COMMANDS[self as usize].label
    }

    /// The command whose [`Command::name`] is `name`, if the table holds it.
    pub fn from_name(name: &str) -> Option<Command> {
        COMMANDS
            .iter()
            .find(|info| info.name == name)
            .map(|info| info.command)
    }

    /// Whether the app carries this command out today. A binding whose command is not carried out
    /// is still listed, dimmed, rather than hidden: the key is bound and does nothing, which is the
    /// fact worth showing.
    pub fn runnable(self) -> bool {
        COMMANDS[self as usize].runnable
    }

    /// The command's name in the table: what a palette identifies it by, and what a test reads.
    /// Lower-case with dashes, so it survives being an id and never reads as its label.
    pub fn name(self) -> &'static str {
        COMMANDS[self as usize].name
    }
}

/// Every command the app can be asked for, in the enum's order.
pub const ALL: &[Command] = &[
    Command::StartDueSession,
    Command::StartNewSession,
    Command::NewCard,
    Command::Leave,
    Command::Save,
    Command::ToggleHistory,
    Command::Reveal,
    Command::GradeRemembered,
    Command::GradeForgot,
    Command::Forward,
    Command::Back,
    Command::Shuffle,
    Command::ShowShortcuts,
    Command::ShowPalette,
    Command::AddRectangle,
    Command::WiderRectangle,
    Command::NarrowerRectangle,
    Command::TallerRectangle,
    Command::ShorterRectangle,
    Command::CramSession,
    Command::Settings,
    Command::SelectionNext,
    Command::SelectionPrevious,
    Command::SelectionFirst,
    Command::SelectionLast,
    Command::FocusSearch,
    Command::EditCard,
    Command::PreviewCard,
    Command::DeleteCard,
    Command::AddCardBelow,
    Command::AddCardAbove,
    Command::MoveCardUp,
    Command::MoveCardDown,
];

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
    Home,
    End,
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
            Key::Home => f.write_str("Home"),
            Key::End => f.write_str("End"),
            Key::Backspace => f.write_str("Backspace"),
        }
    }
}

/// Modifier flags for a chord that needs them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
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
#[non_exhaustive]
pub struct Binding {
    pub surface: Surface,
    pub command: Command,
    pub chord: Chord,
}

impl Binding {
    /// A binding connecting `chord` to `command` on `surface`.
    pub const fn new(surface: Surface, command: Command, chord: Chord) -> Self {
        Self {
            surface,
            command,
            chord,
        }
    }
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
        Command::FocusSearch,
        Chord::key(Key::Char('/')),
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
        Command::SelectionPrevious,
        Chord::key(Key::Left),
    ),
    row(
        Surface::Library,
        Command::SelectionNext,
        Chord::key(Key::Right),
    ),
    row(
        Surface::Library,
        Command::SelectionFirst,
        Chord::key(Key::Home),
    ),
    row(
        Surface::Library,
        Command::SelectionLast,
        Chord::key(Key::End),
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
/// Every chord bound to `command` on `surface`: a command may have more than one (Down and `j`
/// both walk the library), and every one must act.
pub fn chords_of(surface: Surface, command: Command) -> Vec<Chord> {
    table()
        .iter()
        .filter(|binding| binding.surface == surface && binding.command == command)
        .map(|binding| binding.chord)
        .collect()
}

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
    fn the_grids_ends_are_home_and_end() {
        assert_eq!(
            chord_of(Surface::Library, Command::SelectionFirst),
            Some(Chord::key(Key::Home)),
            "Home selects the first card"
        );
        assert_eq!(
            chord_of(Surface::Library, Command::SelectionLast),
            Some(Chord::key(Key::End)),
            "End selects the last card"
        );
    }

    #[test]
    fn the_arrow_keys_step_the_grid() {
        for (chord, command) in [
            (Chord::key(Key::Up), Command::SelectionPrevious),
            (Chord::key(Key::Left), Command::SelectionPrevious),
            (Chord::key(Key::Down), Command::SelectionNext),
            (Chord::key(Key::Right), Command::SelectionNext),
        ] {
            assert_eq!(
                dispatch(Surface::Library, chord),
                Some(command),
                "{chord} moves the selection"
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
