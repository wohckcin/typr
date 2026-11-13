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

pub use db::Db;
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
    pub archived: bool,
    pub trashed_at: Option<i64>,
    pub updated_at: i64,
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
        Self { new_per_day: Self::DEFAULT_NEW_PER_DAY }
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
    CreateDeck { parent: Option<Id>, name: String },
    RenameDeck { id: Id, name: String },
    ArchiveDeck { id: Id, archived: bool },
    CreateCard { deck: Id, markdown: String },
    SaveCard { id: Id, markdown: String },
    TrashCard { id: Id },
    RestoreCard { id: Id },
    SetDraft { card: Id, text: Option<String> },
    SetSetting { key: String, value: String },
    History { card: Id },
    Stats { from_ms: i64, to_ms: i64 },
}

/// What a command did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Deck(Id),
    Card(Id),
    /// How many rows changed; zero means the id did not exist.
    Changed(usize),
    History(Vec<ReviewRow>),
    Stats(Stats),
    Ok,
}

/// The answer to one command, carrying the id it was submitted with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub id: u64,
    pub result: Result<Outcome, String>,
}

/// What can go wrong opening or writing the store.
#[derive(Debug)]
pub enum StoreError {
    Sqlite(rusqlite::Error),
    /// No home folder, so there is nowhere to keep the library.
    NoDataDir,
    /// A database written by a newer build: refused rather than downgraded.
    TooNew { found: i64, supported: i64 },
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
            StoreError::Io(e) => write!(f, "could not create the library folder: {e}"),
        }
    }
}

impl std::error::Error for StoreError {}

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
    Db::open(&database_path(dirs))
}

/// The only way the UI reaches the store.
pub trait Store: Send + Sync {
    /// The last published snapshot: decks, cards and settings, never card text.
    fn snapshot(&self) -> Snapshot;

    /// Take the events that arrived since the last call.
    fn poll(&self) -> Vec<Event>;

    fn create_deck(&self, parent: Option<Id>, name: &str) -> u64;
    fn rename_deck(&self, id: Id, name: &str) -> u64;
    fn archive_deck(&self, id: Id, archived: bool) -> u64;
    fn create_card(&self, deck: Id, markdown: &str) -> u64;
    fn save_card(&self, id: Id, markdown: &str) -> u64;
    fn trash_card(&self, id: Id) -> u64;
    fn restore_card(&self, id: Id) -> u64;
    fn set_draft(&self, card: Id, text: Option<&str>) -> u64;
    fn set_setting(&self, key: &str, value: &str) -> u64;
    fn history(&self, card: Id) -> u64;
    fn stats(&self, from_ms: i64, to_ms: i64) -> u64;
}

#[cfg(test)]
mod tests;
