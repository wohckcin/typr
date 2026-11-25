//! How a deck is read: the four layouts its cards can be shown in.
//!
//! The mode is stored per deck (`deck.view`), because a deck of long cards wants the notebook while
//! a deck of vocabulary wants the list. It lives here rather than in the desktop crate so the store
//! can write it and a test can name it without a toolkit.

/// The layout a deck is shown in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ViewMode {
    /// Cards as tiles, as many across as the window holds.
    #[default]
    Grid,
    /// One line per card, which is the only layout a version-11 library had.
    List,
    /// Every card's contents in full, stacked.
    Notebook,
    /// One card's contents at reading width.
    Column,
}

impl ViewMode {
    /// Every view, in the order the switcher shows them.
    pub const ALL: [ViewMode; 4] = [
        ViewMode::Grid,
        ViewMode::List,
        ViewMode::Notebook,
        ViewMode::Column,
    ];

    /// The name the store writes, and the name a settings file would show.
    pub fn as_str(self) -> &'static str {
        match self {
            ViewMode::Grid => "grid",
            ViewMode::List => "list",
            ViewMode::Notebook => "notebook",
            ViewMode::Column => "column",
        }
    }

    /// An unknown name reads as the grid, so a hand-edited file cannot leave a deck unreadable.
    pub fn parse(value: &str) -> ViewMode {
        match value {
            "list" => ViewMode::List,
            "notebook" => ViewMode::Notebook,
            "column" => ViewMode::Column,
            _ => ViewMode::Grid,
        }
    }

    /// What the switcher's button says.
    pub fn label(self) -> &'static str {
        match self {
            ViewMode::Grid => "Grid",
            ViewMode::List => "List",
            ViewMode::Notebook => "Notebook",
            ViewMode::Column => "Column",
        }
    }
}

#[cfg(test)]
mod tests;
