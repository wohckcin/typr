//! The store: one SQLite file, one writer thread, and `Store` as the only door.
//!
//! Views never touch a `Connection`: they submit a command and read the snapshot the store task
//! publishes. `db.rs` holds the SQL and runs synchronously, so tests exercise it without a
//! thread; `task.rs` is the single worker that owns the file.

mod db;
mod schema;
mod task;

use std::sync::Arc;

pub use db::{Db, Folded, folded};
pub use schema::{CURRENT_VERSION, SchemaFacts};
pub use task::{Handle, Waker, WorkerStatus};

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

    /// `None` when the text is not one of the three the schema's `CHECK` allows.
    pub fn parse(text: &str) -> Option<Phase> {
        match text {
            "learn" => Some(Phase::Learn),
            "review" => Some(Phase::Review),
            "relearn" => Some(Phase::Relearn),
            _ => None,
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

    /// `None` when the text is not one of the two the schema's `CHECK` allows.
    pub fn parse(text: &str) -> Option<Grade> {
        match text {
            "remembered" => Some(Grade::Remembered),
            "forgot" => Some(Grade::Forgot),
            _ => None,
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
    History {
        rows: Vec<ReviewRow>,
        /// Real reviews, the ones the summary and the history panel count.
        reviews: u32,
    },
    Due(Vec<DuePrompt>),
    New(Vec<NewPrompt>),
    Draft(Option<String>),
    Stats(Stats),
    Ok,
}

impl Outcome {
    /// Whether this answer came from a command that wrote to the library. Only a write moves the
    /// revision and only a write means the snapshot the views hold is stale.
    pub fn writes(&self) -> bool {
        match self {
            Outcome::Deck(_)
            | Outcome::Card(_)
            | Outcome::Changed(_)
            | Outcome::Graded { .. }
            | Outcome::Ok => true,
            Outcome::History { .. }
            | Outcome::Due(_)
            | Outcome::New(_)
            | Outcome::Draft(_)
            | Outcome::Stats(_) => false,
        }
    }
}

/// The answer to one command, carrying the id it was submitted with.
#[derive(Debug)]
pub struct Event {
    pub id: u64,
    pub result: Result<Outcome, StoreError>,
}

/// What can go wrong opening or writing the store.
///
/// Every variant says what the caller can do about it and keeps its cause reachable through
/// [`std::error::Error::source`], so the UI can show one line while a log or a debugger can still
/// see the SQLite or `io` error underneath.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StoreError {
    /// The database refused a statement. The cause is reachable through `source()`, not repeated
    /// here, so a chain-aware description reads once.
    #[error("the database refused the statement")]
    Sqlite(#[from] rusqlite::Error),
    /// No home folder, so there is nowhere to keep the library.
    #[error("no home folder to keep the library in")]
    NoDataDir,
    /// A database written by a newer build: refused rather than downgraded.
    #[error(
        "this library was written by a newer version of typr (schema {found}, this build knows {supported})"
    )]
    TooNew { found: i64, supported: i64 },
    /// A grade for a prompt that does not exist, e.g. for a card with one side.
    #[error("card {card} has no prompt {key}")]
    NoPrompt { card: Id, key: String },
    /// A move that would put a deck inside itself.
    #[error("deck {deck} cannot be moved inside itself")]
    Cycle { deck: Id },
    /// A schema version no build could have written: the file is not one of ours.
    #[error("this file records schema version {version}, which is not a typr library")]
    Corrupt { version: i64 },
    /// A value in the database that no build of typr writes.
    #[error("the library holds {value} in {column}, which typr never wrote")]
    Text { column: &'static str, value: String },
    /// A setting the store cannot honour.
    #[error("{value} is not a valid value for {key}")]
    Setting { key: String, value: String },
    /// A file operation on the library failed. The cause is reachable through `source()`.
    #[error("a file operation on the library failed")]
    Io(#[from] std::io::Error),
    /// The worker thread is gone, so nothing was sent and no answer will come.
    #[error("the library is closed")]
    Closed,
    /// The background library work panicked. The connection is suspect, so nothing restarts it.
    #[error("the library stopped: {message}")]
    WorkerPanicked { message: String },
}

/// The only way the UI reaches the store.
///
/// Every command answers exactly one [`Event`] carrying the id the method returned. An id that
/// never answers means the worker has already closed, which only happens while quitting.
pub trait Store: Send + Sync {
    /// The last published snapshot: decks, cards and settings. Cards carry their markdown, so a
    /// session can split sides without another round trip; the snapshot is cloned per call and is
    /// republished only when a command wrote.
    fn snapshot(&self) -> Arc<Snapshot>;

    /// Take the events that arrived since the last call.
    fn poll(&self) -> Vec<Event>;

    fn create_deck(&self, parent: Option<Id>, name: &str) -> Result<u64, StoreError>;
    fn rename_deck(&self, id: Id, name: &str) -> Result<u64, StoreError>;
    fn archive_deck(&self, id: Id, archived: bool) -> Result<u64, StoreError>;
    fn move_deck(&self, id: Id, parent: Option<Id>) -> Result<u64, StoreError>;
    fn create_card(&self, deck: Id, markdown: &str) -> Result<u64, StoreError>;
    fn save_card(&self, id: Id, markdown: &str) -> Result<u64, StoreError>;
    fn move_card(&self, id: Id, deck: Id) -> Result<u64, StoreError>;
    fn archive_card(&self, id: Id, archived: bool) -> Result<u64, StoreError>;
    fn set_tags(&self, card: Id, tags: &[String]) -> Result<u64, StoreError>;
    fn trash_card(&self, id: Id) -> Result<u64, StoreError>;
    fn restore_card(&self, id: Id) -> Result<u64, StoreError>;
    fn empty_trash(&self) -> Result<u64, StoreError>;
    fn set_draft(&self, card: Id, text: Option<&str>) -> Result<u64, StoreError>;
    fn draft(&self, card: Id) -> Result<u64, StoreError>;
    fn grade(
        &self,
        card: Id,
        key: &str,
        grade: Grade,
        duration_ms: Option<i64>,
    ) -> Result<u64, StoreError>;
    fn reset_history(&self, card: Id, key: &str) -> Result<u64, StoreError>;

    /// The due queue as the scheduler would serve it: most overdue first, ties shuffled.
    fn due_prompts(&self, now_ms: i64, offset_ms: i64, seed: u64) -> Result<u64, StoreError>;
    /// The new-card queue, capped by what this study day still allows.
    fn new_prompts(&self, now_ms: i64, offset_ms: i64, seed: u64) -> Result<u64, StoreError>;
    fn set_setting(&self, key: &str, value: &str) -> Result<u64, StoreError>;
    fn history(&self, card: Id) -> Result<u64, StoreError>;
    fn stats(&self, from_ms: i64, to_ms: i64) -> Result<u64, StoreError>;
}

#[cfg(test)]
mod tests;
