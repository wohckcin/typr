//! The store: one SQLite file, one writer thread, and `Store` as the only door.
//!
//! Views never touch a `Connection`: they submit a command and read the snapshot the store task
//! publishes. `db.rs` holds the SQL and runs synchronously, so tests exercise it without a
//! thread; `task.rs` is the single worker that owns the file.

mod db;
mod schema;
mod task;

use std::path::PathBuf;

use rune::dirs::AppDirs;

pub use db::{Db, Folded, folded};
pub use schema::CURRENT_VERSION;
pub use task::Handle;

/// The app id used for both Rune's folders and this app's file.
pub const APP_ID: &str = "typr";

/// A deck, card, prompt or log row.
pub type Id = i64;

/// Where a prompt stands in the learning process.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Still in the new-card queue.
    Learn,
    /// Scheduled: this is a real review.
    Review,
    /// In a relearning step after a failure.
    Relearn,
}

impl Phase {
    pub fn as_str(self) -> &'static str {
        match self {
            Phase::Learn => "learn",
            Phase::Review => "review",
            Phase::Relearn => "relearn",
        }
    }

    /// The schema's `CHECK` constraint keeps this total; an unknown value reads as `Review`.
    pub fn parse(text: &str) -> Self {
        match text {
            "learn" => Phase::Learn,
            "relearn" => Phase::Relearn,
            _ => Phase::Review,
        }
    }
}

/// The two answers a review accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grade {
    Forgot,
    Remembered,
}

impl Grade {
    pub fn as_str(self) -> &'static str {
        match self {
            Grade::Forgot => "forgot",
            Grade::Remembered => "remembered",
        }
    }

    pub fn parse(text: &str) -> Self {
        match text {
            "remembered" => Grade::Remembered,
            _ => Grade::Forgot,
        }
    }
}

/// A deck in the tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Deck {
    pub id: Id,
    pub parent: Option<Id>,
    pub name: String,
    pub archived: bool,
}

/// A card as the library lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardSummary {
    pub id: Id,
    pub deck: Id,
    /// How many sides the text splits into; one side is a note and never queues.
    pub sides: u32,
    /// The card's tags, in the casing each was first written across the whole library.
    pub tags: Vec<String>,
    pub archived: bool,
    pub trashed_at: Option<i64>,
    pub updated_at: i64,
    /// The first side's first line, cut short: what a list row shows.
    pub preview: String,
    /// The card's text, so a session can split it into sides without another round trip.
    pub markdown: String,
    /// The card's text folded the way `LIKE` compares it, so the library can filter without a
    /// round trip to the store thread.
    pub search_text: String,
}

/// Settings the user can change. Only the new-card cap exists in v1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub new_per_day: u32,
}

impl Settings {
    pub const DEFAULT_NEW_PER_DAY: u32 = 20;
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            new_per_day: Self::DEFAULT_NEW_PER_DAY,
        }
    }
}

/// Everything a view may read without a round trip.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    pub decks: Vec<Deck>,
    pub cards: Vec<CardSummary>,
    pub settings: Settings,
    /// Bumped once per applied command, so a view can tell a re-publish from a change.
    pub revision: u64,
}

/// Which slice of the library a list shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CardMode {
    #[default]
    Active,
    Archived,
    Trash,
}

impl Snapshot {
    /// The deck and everything under it, or every deck when `deck` is `None`.
    pub fn subtree(&self, deck: Option<Id>) -> Vec<Id> {
        let Some(deck) = deck else {
            return self.decks.iter().map(|deck| deck.id).collect();
        };
        let mut ids = vec![deck];
        let mut index = 0;
        while index < ids.len() {
            let parent = ids[index];
            ids.extend(
                self.decks
                    .iter()
                    .filter(|d| d.parent == Some(parent))
                    .map(|d| d.id),
            );
            index += 1;
        }
        ids
    }

    /// Decks whose cards the study queues may touch: no archived deck in the chain.
    fn studyable_decks(&self) -> Vec<Id> {
        let archived = |id: Id| self.decks.iter().any(|deck| deck.id == id && deck.archived);
        self.decks
            .iter()
            .filter(|deck| {
                let mut chain = Some(deck.id);
                while let Some(id) = chain {
                    if archived(id) {
                        return false;
                    }
                    chain = self
                        .decks
                        .iter()
                        .find(|d| d.id == id)
                        .and_then(|d| d.parent);
                }
                true
            })
            .map(|deck| deck.id)
            .collect()
    }

    /// The cards a library list shows for a filter.
    pub fn cards_matching(
        &self,
        query: &str,
        tag: Option<&str>,
        deck: Option<Id>,
        mode: CardMode,
    ) -> Vec<&CardSummary> {
        let needle = folded(query);
        let scope = self.subtree(deck);
        let visible_decks = self.studyable_decks();
        let wanted_tag = tag.map(folded);
        self.cards
            .iter()
            .filter(|card| match mode {
                CardMode::Active => {
                    !card.archived
                        && card.trashed_at.is_none()
                        && visible_decks.contains(&card.deck)
                }
                CardMode::Archived => card.archived && card.trashed_at.is_none(),
                CardMode::Trash => card.trashed_at.is_some(),
            })
            .filter(|card| deck.is_none() || scope.contains(&card.deck))
            .filter(|card| {
                wanted_tag
                    .as_ref()
                    .is_none_or(|tag| card.tags.iter().any(|own| folded(own) == *tag))
            })
            .filter(|card| needle.is_empty() || card.search_text.contains(&needle))
            .collect()
    }
}

/// One prompt that is due now, with the card it belongs to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DuePrompt {
    pub card: Id,
    pub key: String,
    pub due_at: i64,
}

/// A card still waiting for its first lesson.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewPrompt {
    pub card: Id,
    pub key: String,
}

/// Grades recorded in a window, counting real reviews only.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub reviews: u32,
    pub lapses: u32,
}

/// One row of a card's history, learning cycles included.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewRow {
    pub card: Id,
    pub key: String,
    pub phase: Phase,
    pub at: i64,
    pub grade: Grade,
    pub interval_ms: i64,
    pub due_at: Option<i64>,
    pub duration_ms: Option<i64>,
}

/// A typed mutation or query. Every command produces exactly one [`Event`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    CreateDeck {
        parent: Option<Id>,
        name: String,
    },
    RenameDeck {
        id: Id,
        name: String,
    },
    ArchiveDeck {
        id: Id,
        archived: bool,
    },
    MoveDeck {
        id: Id,
        parent: Option<Id>,
    },
    CreateCard {
        deck: Id,
        markdown: String,
    },
    SaveCard {
        id: Id,
        markdown: String,
    },
    MoveCard {
        id: Id,
        deck: Id,
    },
    ArchiveCard {
        id: Id,
        archived: bool,
    },
    EmptyTrash,
    SetTags {
        card: Id,
        tags: Vec<String>,
    },
    TrashCard {
        id: Id,
    },
    RestoreCard {
        id: Id,
    },
    SetDraft {
        card: Id,
        text: Option<String>,
    },
    Draft {
        card: Id,
    },
    Grade {
        card: Id,
        key: String,
        grade: Grade,
        duration_ms: Option<i64>,
    },
    ResetHistory {
        card: Id,
        key: String,
    },
    /// The due queue, ordered and capped exactly as the scheduler would serve it.
    DuePrompts {
        now_ms: i64,
        offset_ms: i64,
        seed: u64,
    },
    /// The new-card queue, capped by what the study day still allows.
    NewPrompts {
        now_ms: i64,
        offset_ms: i64,
        seed: u64,
    },
    Search {
        query: String,
        deck: Option<Id>,
    },
    SetSetting {
        key: String,
        value: String,
    },
    History {
        card: Id,
    },
    Stats {
        from_ms: i64,
        to_ms: i64,
    },
}

/// What a command did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Deck(Id),
    Card(Id),
    /// How many rows changed; zero means the id did not exist.
    Changed(usize),
    /// Where the grade left the prompt.
    Graded {
        phase: Phase,
        due_at: Option<i64>,
    },
    History(Vec<ReviewRow>),
    Due(Vec<DuePrompt>),
    New(Vec<NewPrompt>),
    Draft(Option<String>),
    Search(Vec<Id>),
    Stats(Stats),
    Ok,
}

/// The answer to one command, carrying the id it was submitted with.
#[derive(Debug)]
pub struct Event {
    pub id: u64,
    pub result: Result<Outcome, StoreError>,
}

/// What can go wrong opening or writing the store.
#[derive(Debug)]
pub enum StoreError {
    Sqlite(rusqlite::Error),
    /// No home folder, so there is nowhere to keep the library.
    NoDataDir,
    /// A database written by a newer build: refused rather than downgraded.
    TooNew {
        found: i64,
        supported: i64,
    },
    /// A grade for a prompt that does not exist, e.g. for a card with one side.
    NoPrompt {
        card: Id,
        key: String,
    },
    /// A move that would put a deck inside itself.
    Cycle {
        deck: Id,
    },
    Io(std::io::Error),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Sqlite(e) => write!(f, "database error: {e}"),
            StoreError::NoDataDir => write!(f, "no home folder to keep the library in"),
            StoreError::TooNew { found, supported } => write!(
                f,
                "this library was written by a newer version of typr (schema {found}, this build knows {supported})"
            ),
            StoreError::NoPrompt { card, key } => write!(f, "card {card} has no prompt {key}"),
            StoreError::Cycle { deck } => write!(f, "deck {deck} cannot be moved inside itself"),
            StoreError::Io(e) => write!(f, "a file operation on the library failed: {e}"),
        }
    }
}

impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StoreError::Sqlite(error) => Some(error),
            StoreError::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<rusqlite::Error> for StoreError {
    fn from(error: rusqlite::Error) -> Self {
        StoreError::Sqlite(error)
    }
}

impl From<std::io::Error> for StoreError {
    fn from(error: std::io::Error) -> Self {
        StoreError::Io(error)
    }
}

/// The library file inside an app's data folder.
pub fn database_path(dirs: &AppDirs) -> PathBuf {
    dirs.data().join("typr.sqlite3")
}

/// The folder a card's images are copied into, beside the database.
pub fn attachments_root(dirs: &AppDirs) -> PathBuf {
    dirs.data().join("attachments")
}

/// The attachments folder for the process environment.
pub fn attachments_root_from_environment() -> Result<PathBuf, StoreError> {
    let dirs = AppDirs::new(APP_ID).ok_or(StoreError::NoDataDir)?;
    Ok(attachments_root(&dirs))
}

/// Open the library from the process environment, creating its folder if needed.
///
/// `RUNE_DIRS_ROOT`, when set, moves every folder (see [`rune::dirs`]), which is how a test or an
/// isolated run keeps out of the real one.
pub fn open_from_environment() -> Result<Db, StoreError> {
    let dirs = AppDirs::new(APP_ID).ok_or(StoreError::NoDataDir)?;
    open_in(&dirs)
}

/// Open the library inside the given folders.
pub fn open_in(dirs: &AppDirs) -> Result<Db, StoreError> {
    std::fs::create_dir_all(dirs.data())?;
    std::fs::create_dir_all(attachments_root(dirs))?;
    Db::open(&database_path(dirs))
}

/// The only way the UI reaches the store.
///
/// Every command answers exactly one [`Event`] carrying the id the method returned. An id that
/// never answers means the worker has already closed, which only happens while quitting.
pub trait Store: Send + Sync {
    /// The last published snapshot: decks, cards and settings, never card text.
    fn snapshot(&self) -> Snapshot;

    /// Take the events that arrived since the last call.
    fn poll(&self) -> Vec<Event>;

    fn create_deck(&self, parent: Option<Id>, name: &str) -> u64;
    fn rename_deck(&self, id: Id, name: &str) -> u64;
    fn archive_deck(&self, id: Id, archived: bool) -> u64;
    fn move_deck(&self, id: Id, parent: Option<Id>) -> u64;
    fn create_card(&self, deck: Id, markdown: &str) -> u64;
    fn save_card(&self, id: Id, markdown: &str) -> u64;
    fn move_card(&self, id: Id, deck: Id) -> u64;
    fn archive_card(&self, id: Id, archived: bool) -> u64;
    fn set_tags(&self, card: Id, tags: &[String]) -> u64;
    fn trash_card(&self, id: Id) -> u64;
    fn restore_card(&self, id: Id) -> u64;
    fn empty_trash(&self) -> u64;
    fn set_draft(&self, card: Id, text: Option<&str>) -> u64;
    fn draft(&self, card: Id) -> u64;
    fn grade(&self, card: Id, key: &str, grade: Grade, duration_ms: Option<i64>) -> u64;
    fn reset_history(&self, card: Id, key: &str) -> u64;
    fn search(&self, query: &str, deck: Option<Id>) -> u64;
    /// The due queue as the scheduler would serve it: most overdue first, ties shuffled.
    fn due_prompts(&self, now_ms: i64, offset_ms: i64, seed: u64) -> u64;
    /// The new-card queue, capped by what this study day still allows.
    fn new_prompts(&self, now_ms: i64, offset_ms: i64, seed: u64) -> u64;
    fn set_setting(&self, key: &str, value: &str) -> u64;
    fn history(&self, card: Id) -> u64;
    fn stats(&self, from_ms: i64, to_ms: i64) -> u64;
}

#[cfg(test)]
mod tests;
