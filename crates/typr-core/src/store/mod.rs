//! The store: one SQLite file, one writer thread, and `Store` as the only door.
//!
//! Views never touch a `Connection`: they submit a command and read the snapshot the store task
//! publishes. `db.rs` holds the SQL and runs synchronously, so tests exercise it without a
//! thread; `task.rs` is the single worker that owns the file.

mod db;
mod schema;
mod task;
mod view;

use std::sync::Arc;

use crate::hidden;

pub use db::{Db, Folded, folded};
pub use schema::{CURRENT_VERSION, SchemaFacts};
pub use task::{Handle, Waker, WorkerStatus};
pub use view::{SavedView, Sort, SortDir, ViewSpec, shuffled};

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
    /// `None` inherits the parent's value, which is what makes the nearest setting win.
    pub reverse: Option<bool>,
    /// The markdown its cards are rendered from, or `None` to inherit the parent deck's.
    pub template: Option<String>,
}

/// Which of the two things a card asks: text, or a picture with rectangles on it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CardKind {
    /// Text is what a card is until it is made a diagram, which is also what a fixture starts as.
    #[default]
    Text,
    Diagram,
}

impl CardKind {
    pub fn parse(value: &str) -> CardKind {
        match value {
            "diagram" => CardKind::Diagram,
            _ => CardKind::Text,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            CardKind::Text => "text",
            CardKind::Diagram => "diagram",
        }
    }
}

/// A card as the library lists it.
///
/// `Default` is what a fixture fills in: a card that asks nothing, in deck 0. Tests name only the
/// fields they care about, so a new field here does not rewrite every literal.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CardSummary {
    pub id: Id,
    pub deck: Id,
    /// Whether the card asks in text or in a picture.
    pub kind: CardKind,
    /// The picture a diagram card asks about, as a name under the attachments folder.
    pub image: Option<String>,
    /// What the picture shows; a diagram card may explain itself.
    pub caption: Option<String>,
    /// Every rectangle the card holds, one row each, ordered by group then top-left first.
    pub masks: Vec<MaskRow>,
    /// How many sides the text splits into; one side is a note and never queues.
    pub sides: u32,
    /// The card's tags, in the casing each was first written across the whole library.
    pub tags: Vec<String>,
    pub archived: bool,
    /// `None` inherits the nearest deck's value, or the library default.
    pub reverse: Option<bool>,
    pub trashed_at: Option<i64>,
    pub updated_at: i64,
    pub created_at: i64,
    /// How many real reviews the card has had, which one of the list's orders ranks by.
    pub reviews: u32,
    /// Whether the card is pinned, which a view can ask to list alone.
    pub pinned: bool,
    /// The first side's first line, cut short: what a list row shows.
    pub preview: String,
    /// The card's text, so a session can split it into sides without another round trip.
    pub markdown: String,
    /// The card's text folded the way `LIKE` compares it, so the library can filter without a
    /// round trip to the store thread.
    pub search_text: String,
    /// How many studyable prompts the card carries. Zero means a note, which never queues.
    pub prompts: u32,
    /// How many of those prompts have never been scheduled.
    pub unscheduled: u32,
    /// The earliest due time among them; what a tile's badge reads.
    pub due_at: Option<i64>,
    /// The values a template fills from, ordered by name.
    pub fields: Vec<(String, String)>,
    /// The template this card's text is rendered from, whether its own or a deck's.
    pub template: Option<String>,
    /// The deck the template is inherited from; `None` means the card carries its own.
    pub template_from: Option<Id>,
}

impl CardSummary {
    /// The questions this card holds, in the keys the store schedules it under.
    ///
    /// A diagram is asked a group at a time, hidden text a group at a time, and a plain card by its
    /// sides. The store counts these in `prompts`; this names them, which is what a cram run needs
    /// to build a queue the scheduler did not.
    pub fn prompt_keys(&self) -> Vec<String> {
        if self.kind == CardKind::Diagram {
            let mut groups: Vec<u32> = self.masks.iter().map(|mask| mask.group).collect();
            groups.sort_unstable();
            groups.dedup();
            return groups
                .into_iter()
                .map(|group| format!("mask:{group}"))
                .collect();
        }
        let shape = hidden::shape(&self.markdown);
        let mut keys = shape.prompt_keys();
        // The effective reverse is decided by the card, its deck and the library, and none of them
        // is here. The count is: the store already dropped `rev` from it when nothing asks it.
        if shape.claims_reverse() && self.prompts > 1 {
            keys.push("rev".to_string());
        }
        keys
    }

    /// Whether a mode's slice of the library holds this card.
    ///
    /// A card is in the archive either because it was archived itself or because the deck holding
    /// it was, which is the only place it can be reached from; the trash is its own slice.
    pub(crate) fn mode_matches(&self, mode: CardMode, visible_decks: &[Id]) -> bool {
        match mode {
            CardMode::Active => {
                !self.archived && self.trashed_at.is_none() && visible_decks.contains(&self.deck)
            }
            CardMode::Archived => {
                self.trashed_at.is_none() && (self.archived || !visible_decks.contains(&self.deck))
            }
            CardMode::Trash => self.trashed_at.is_some(),
        }
    }
}

/// Settings the user can change. Only the new-card cap exists in v1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub new_per_day: u32,
    /// Whether a card is reviewed from its last side unless a deck or the card says otherwise.
    pub reverse_default: bool,
}

impl Settings {
    pub const DEFAULT_NEW_PER_DAY: u32 = 20;
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            new_per_day: Self::DEFAULT_NEW_PER_DAY,
            reverse_default: false,
        }
    }
}

/// Everything a view may read without a round trip.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Snapshot {
    pub decks: Vec<Deck>,
    pub cards: Vec<CardSummary>,
    /// The views the user saved, by name.
    pub views: Vec<SavedView>,
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
    pub(crate) fn studyable_decks(&self) -> Vec<Id> {
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

    /// The cards a library list shows for a filter: those `spec` keeps, in its order.
    pub fn cards_matching(&self, spec: &ViewSpec) -> Vec<&CardSummary> {
        spec.cards(self)
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

/// A rectangle over a diagram, as fractions of the image's width and height.
///
/// Fractions rather than pixels: the same card is drawn 400 points wide on a tile and 900 wide in a
/// review, and a pixel rectangle would drift between them and between the scene renders and a window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaskRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl MaskRect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> MaskRect {
        MaskRect { x, y, w, h }
    }

    /// Whether the rectangle sits inside the image's own square of fractions.
    pub fn inside_the_image(&self) -> bool {
        self.w > 0.0
            && self.h > 0.0
            && self.x >= 0.0
            && self.y >= 0.0
            && self.x + self.w <= 1.0
            && self.y + self.h <= 1.0
    }
}

/// One rectangle of one group, as the library holds it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaskRow {
    pub group: u32,
    pub rect: MaskRect,
}

/// A typed mutation or query. Every command produces exactly one [`Event`].
///
/// `PartialEq` and not `Eq`: a mask's corners are fractions, and a float has no total equality.
#[derive(Clone, Debug, PartialEq)]
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
    /// A card whose faces are groups of rectangles drawn on an image.
    CreateDiagram {
        deck: Id,
        image: String,
        caption: String,
    },
    /// Replace the picture and the caption of a diagram card; the masks stay where they are.
    SetDiagram {
        card: Id,
        image: String,
        caption: String,
    },
    AddMask {
        card: Id,
        group: u32,
        rect: MaskRect,
    },
    /// Move one rectangle into another group. The pair of groups may end up as one.
    SetMaskGroup {
        card: Id,
        rect: MaskRect,
        group: u32,
    },
    RemoveMask {
        card: Id,
        rect: MaskRect,
    },
    /// Put every listed rectangle into one group, which is how a rectangle that is one answer is
    /// made from several.
    MergeMasks {
        card: Id,
        rects: Vec<MaskRect>,
        group: u32,
    },
    MaskGroups {
        card: Id,
    },
    Masks {
        card: Id,
    },
    SaveCard {
        id: Id,
        markdown: String,
    },
    /// Replace a card's fields wholesale; the card's text is re-rendered from its template.
    SetFields {
        card: Id,
        fields: Vec<(String, String)>,
    },
    /// `None` takes the card back to inheriting its deck's template.
    SetCardTemplate {
        card: Id,
        template: Option<String>,
    },
    /// `None` takes the deck, and every card under it without a template of its own, back to
    /// rendering the text each already holds.
    SetDeckTemplate {
        deck: Id,
        template: Option<String>,
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
        deck: Option<Id>,
    },
    /// The new-card queue, capped by what the study day still allows.
    NewPrompts {
        now_ms: i64,
        offset_ms: i64,
        seed: u64,
        deck: Option<Id>,
    },
    SetSetting {
        key: String,
        value: String,
    },
    /// `None` takes the card back to inheriting its deck.
    SetCardReverse {
        card: Id,
        reverse: Option<bool>,
    },
    /// `None` takes the deck back to inheriting its parent.
    SetDeckReverse {
        deck: Id,
        reverse: Option<bool>,
    },
    /// Save the current filter under a name.
    CreateView {
        spec: ViewSpec,
        name: String,
    },
    RenameView {
        id: Id,
        name: String,
    },
    DeleteView {
        id: Id,
    },
    UpdateView {
        id: Id,
        spec: ViewSpec,
    },
    /// Whether a card is pinned, which a view can ask to list alone.
    SetPinned {
        card: Id,
        pinned: bool,
    },
    History {
        card: Id,
    },
    Stats {
        from_ms: i64,
        to_ms: i64,
    },
    /// The cards the filter selects, for a cram run: the library's list without the schedule.
    CardsForView {
        spec: ViewSpec,
    },
}

/// What a command did.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    Deck(Id),
    Card(Id),
    View(Id),
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
    /// The groups a diagram card asks about, in ascending order.
    MaskGroups(Vec<u32>),
    /// Every rectangle a diagram card holds, ordered by group and then by position.
    Masks(Vec<MaskRow>),
    Draft(Option<String>),
    Stats(Stats),
    /// The cards a filter selects, ready to be walked without regard for their schedules.
    Cards(Vec<CardSummary>),
    Ok,
}

impl Outcome {
    /// Whether this answer came from a command that wrote to the library. Only a write moves the
    /// revision and only a write means the snapshot the views hold is stale.
    pub fn writes(&self) -> bool {
        match self {
            Outcome::Deck(_)
            | Outcome::Card(_)
            | Outcome::View(_)
            | Outcome::Changed(_)
            | Outcome::Graded { .. }
            | Outcome::Ok => true,
            Outcome::MaskGroups(_) | Outcome::Masks(_) => false,
            Outcome::History { .. }
            | Outcome::Due(_)
            | Outcome::New(_)
            | Outcome::Draft(_)
            | Outcome::Stats(_)
            | Outcome::Cards(_) => false,
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
    /// A file that could not be read as this app's library, named so the reason can be acted on.
    #[error("the library at {path} is unreadable: {source}")]
    Unreadable {
        path: std::path::PathBuf,
        #[source]
        source: rusqlite::Error,
    },
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
    /// A deck name another deck under the same parent already uses.
    #[error("{what} {id} does not exist")]
    Missing { what: &'static str, id: Id },
    #[error("a {what} called {name:?} already exists")]
    DuplicateName { what: &'static str, name: String },
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
    /// A card that cannot change kind because it still holds what the other kind would hide.
    #[error("card {card} still holds {holds}, so it cannot change kind until that is cleared")]
    NotEmpty { card: Id, holds: &'static str },
    /// The worker thread is gone, so nothing was sent and no answer will come.
    #[error("a mask on card {card} does not sit inside its image")]
    MaskOutsideImage { card: Id },
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
    /// Replace a card's fields; its text is re-rendered from whatever template is in force.
    fn set_fields(&self, card: Id, fields: &[(String, String)]) -> Result<u64, StoreError>;
    /// `None` takes the card back to inheriting its deck's template.
    fn set_card_template(&self, card: Id, template: Option<&str>) -> Result<u64, StoreError>;
    /// `None` takes the deck, and the cards under it without one of their own, back to the text
    /// each already holds.
    fn set_deck_template(&self, deck: Id, template: Option<&str>) -> Result<u64, StoreError>;
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

    /// Prompts whose relearning step or review date has come, most overdue first and shuffled
    /// within one study day. `deck` scopes the queue to that deck and its descendants; `None` is
    /// the whole library, and a scope on an archived deck answers nothing.
    fn due_prompts(
        &self,
        now_ms: i64,
        offset_ms: i64,
        seed: u64,
        deck: Option<Id>,
    ) -> Result<u64, StoreError>;
    /// Prompts with nothing scheduled yet, capped by what this study day still allows. `deck`
    /// scopes the queue to that deck and its descendants; `None` is the whole library.
    fn new_prompts(
        &self,
        now_ms: i64,
        offset_ms: i64,
        seed: u64,
        deck: Option<Id>,
    ) -> Result<u64, StoreError>;
    /// The cards the filter `spec` selects, for a cram run: the snapshot's own filter, studyable
    /// cards only, and neither a due date nor the new-card allowance in the way.
    fn cards_for_view(&self, spec: &ViewSpec) -> Result<u64, StoreError>;
    fn set_setting(&self, key: &str, value: &str) -> Result<u64, StoreError>;
    fn set_card_reverse(&self, card: Id, reverse: Option<bool>) -> Result<u64, StoreError>;
    fn set_deck_reverse(&self, deck: Id, reverse: Option<bool>) -> Result<u64, StoreError>;
    fn create_diagram(&self, deck: Id, image: &str, caption: &str) -> Result<u64, StoreError>;
    fn set_diagram(&self, card: Id, image: &str, caption: &str) -> Result<u64, StoreError>;
    fn add_mask(&self, card: Id, group: u32, rect: MaskRect) -> Result<u64, StoreError>;
    fn set_mask_group(&self, card: Id, rect: MaskRect, group: u32) -> Result<u64, StoreError>;
    fn remove_mask(&self, card: Id, rect: MaskRect) -> Result<u64, StoreError>;
    fn merge_masks(&self, card: Id, rects: &[MaskRect], group: u32) -> Result<u64, StoreError>;
    fn mask_groups(&self, card: Id) -> Result<u64, StoreError>;
    fn masks(&self, card: Id) -> Result<u64, StoreError>;
    /// Save the filter `spec` under `name`. A name already in use is refused.
    fn create_view(&self, spec: &ViewSpec, name: &str) -> Result<u64, StoreError>;
    fn rename_view(&self, id: Id, name: &str) -> Result<u64, StoreError>;
    fn delete_view(&self, id: Id) -> Result<u64, StoreError>;
    /// Replace every part of a saved view's filter, keeping its name and id.
    fn update_view(&self, id: Id, spec: &ViewSpec) -> Result<u64, StoreError>;
    fn set_pinned(&self, card: Id, pinned: bool) -> Result<u64, StoreError>;
    fn history(&self, card: Id) -> Result<u64, StoreError>;
    fn stats(&self, from_ms: i64, to_ms: i64) -> Result<u64, StoreError>;
}

#[cfg(test)]
mod tests;
