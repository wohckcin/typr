//! Store command protocol: typed commands, outcomes, events, and the asynchronous `Store` trait.

use std::sync::Arc;

use crate::error::StoreError;
use crate::library_view::ViewMode;
use crate::model::{
    CardSummary, DuePrompt, Grade, Id, ImportReport, MaskRect, MaskRow, NewPrompt, Phase,
    ReviewRow, SchedulerKind, SettingKey, SettingValue, Snapshot, Stats, ViewSpec,
};

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
    /// Remove every rectangle of one group. Its prompt is retired and its review history stays
    /// readable, the way editing a card's sides never resets its schedule.
    RemoveMaskGroup {
        card: Id,
        group: u32,
    },
    /// Turn a diagram card back into an empty text card: the picture, its caption and every
    /// rectangle go, and the card is edited as markdown again. Only a diagram can be cleared.
    ClearDiagram {
        card: Id,
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
    /// A card's own templates. `template` renders its text, `name` names it, and `None` in either
    /// clears that one: the text falls back to the deck's template, the name to the first side.
    SetCardTemplate {
        card: Id,
        template: Option<String>,
        name: Option<String>,
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
    /// Change one setting.
    SetSetting {
        key: SettingKey,
        value: SettingValue,
    },
    SetScheduler {
        kind: SchedulerKind,
    },
    SetRetention {
        percent: u32,
    },
    SetWeights {
        weights: Vec<f64>,
    },
    SetCardReverse {
        card: Id,
        reverse: Option<bool>,
    },
    SetDeckReverse {
        deck: Id,
        reverse: Option<bool>,
    },
    SetDeckView {
        deck: Id,
        view: ViewMode,
    },
    MoveCardTo {
        card: Id,
        position: u32,
    },
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
    SetPinned {
        card: Id,
        pinned: bool,
    },
    ImportArchive {
        archive: std::path::PathBuf,
        attachments: std::path::PathBuf,
    },
    ExportLibrary {
        target: std::path::PathBuf,
    },
    ImportCsv {
        deck: Id,
        source: std::path::PathBuf,
    },
    ExportCsv {
        deck: Id,
        target: std::path::PathBuf,
    },
    ImportMarkdown {
        deck: Id,
        source: std::path::PathBuf,
    },
    ExportMarkdown {
        deck: Id,
        target: std::path::PathBuf,
    },
    History {
        card: Id,
    },
    Stats {
        from_ms: i64,
        to_ms: i64,
    },
    CardsForView {
        spec: ViewSpec,
    },
}

/// The group of store handlers that owns a command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Domain {
    Deck,
    Diagram,
    Card,
    CardState,
    Review,
    View,
    Transfer,
}

impl Command {
    /// Which store handler owns this command. No wildcard arm: a new variant fails to compile
    /// until it is routed.
    pub(crate) fn domain(&self) -> Domain {
        match self {
            Command::CreateDeck { .. }
            | Command::RenameDeck { .. }
            | Command::ArchiveDeck { .. }
            | Command::MoveDeck { .. }
            | Command::SetDeckTemplate { .. }
            | Command::SetDeckReverse { .. }
            | Command::SetDeckView { .. } => Domain::Deck,
            Command::CreateDiagram { .. }
            | Command::SetDiagram { .. }
            | Command::AddMask { .. }
            | Command::SetMaskGroup { .. }
            | Command::RemoveMask { .. }
            | Command::RemoveMaskGroup { .. }
            | Command::ClearDiagram { .. }
            | Command::MergeMasks { .. }
            | Command::MaskGroups { .. }
            | Command::Masks { .. } => Domain::Diagram,
            Command::CreateCard { .. }
            | Command::SaveCard { .. }
            | Command::SetFields { .. }
            | Command::SetCardTemplate { .. }
            | Command::MoveCard { .. }
            | Command::MoveCardTo { .. }
            | Command::SetTags { .. }
            | Command::SetCardReverse { .. } => Domain::Card,
            Command::ArchiveCard { .. }
            | Command::EmptyTrash
            | Command::TrashCard { .. }
            | Command::RestoreCard { .. }
            | Command::SetDraft { .. }
            | Command::Draft { .. }
            | Command::SetPinned { .. } => Domain::CardState,
            Command::Grade { .. }
            | Command::ResetHistory { .. }
            | Command::DuePrompts { .. }
            | Command::NewPrompts { .. }
            | Command::SetSetting { .. }
            | Command::SetScheduler { .. }
            | Command::SetRetention { .. }
            | Command::SetWeights { .. }
            | Command::History { .. }
            | Command::Stats { .. } => Domain::Review,
            Command::CreateView { .. }
            | Command::RenameView { .. }
            | Command::DeleteView { .. }
            | Command::UpdateView { .. }
            | Command::CardsForView { .. } => Domain::View,
            Command::ImportArchive { .. }
            | Command::ExportLibrary { .. }
            | Command::ImportCsv { .. }
            | Command::ExportCsv { .. }
            | Command::ImportMarkdown { .. }
            | Command::ExportMarkdown { .. } => Domain::Transfer,
        }
    }

    /// The command's name, for the diagnostics list of slow writes.
    pub fn name(&self) -> &'static str {
        let domain = self.domain();
        let name = match domain {
            Domain::Deck => self.deck_name(),
            Domain::Diagram => self.diagram_name(),
            Domain::Card => self.card_edit_name(),
            Domain::CardState => self.card_state_name(),
            Domain::Review => self.review_name(),
            Domain::View => self.view_name(),
            Domain::Transfer => self.transfer_name(),
        };
        name.unwrap_or_else(|| unreachable!("{self:?} is routed to {domain:?}"))
    }

    fn deck_name(&self) -> Option<&'static str> {
        Some(match self {
            Command::CreateDeck { .. } => "CreateDeck",
            Command::RenameDeck { .. } => "RenameDeck",
            Command::ArchiveDeck { .. } => "ArchiveDeck",
            Command::MoveDeck { .. } => "MoveDeck",
            Command::SetDeckTemplate { .. } => "SetDeckTemplate",
            Command::SetDeckReverse { .. } => "SetDeckReverse",
            Command::SetDeckView { .. } => "SetDeckView",
            _ => return None,
        })
    }

    fn diagram_name(&self) -> Option<&'static str> {
        Some(match self {
            Command::CreateDiagram { .. } => "CreateDiagram",
            Command::SetDiagram { .. } => "SetDiagram",
            Command::AddMask { .. } => "AddMask",
            Command::SetMaskGroup { .. } => "SetMaskGroup",
            Command::RemoveMask { .. } => "RemoveMask",
            Command::RemoveMaskGroup { .. } => "RemoveMaskGroup",
            Command::ClearDiagram { .. } => "ClearDiagram",
            Command::MergeMasks { .. } => "MergeMasks",
            Command::MaskGroups { .. } => "MaskGroups",
            Command::Masks { .. } => "Masks",
            _ => return None,
        })
    }

    fn card_edit_name(&self) -> Option<&'static str> {
        Some(match self {
            Command::CreateCard { .. } => "CreateCard",
            Command::SaveCard { .. } => "SaveCard",
            Command::SetFields { .. } => "SetFields",
            Command::SetCardTemplate { .. } => "SetCardTemplate",
            Command::MoveCard { .. } => "MoveCard",
            Command::MoveCardTo { .. } => "MoveCardTo",
            Command::SetTags { .. } => "SetTags",
            Command::SetCardReverse { .. } => "SetCardReverse",
            _ => return None,
        })
    }

    fn card_state_name(&self) -> Option<&'static str> {
        Some(match self {
            Command::ArchiveCard { .. } => "ArchiveCard",
            Command::EmptyTrash => "EmptyTrash",
            Command::TrashCard { .. } => "TrashCard",
            Command::RestoreCard { .. } => "RestoreCard",
            Command::SetDraft { .. } => "SetDraft",
            Command::Draft { .. } => "Draft",
            Command::SetPinned { .. } => "SetPinned",
            _ => return None,
        })
    }

    fn review_name(&self) -> Option<&'static str> {
        Some(match self {
            Command::Grade { .. } => "Grade",
            Command::ResetHistory { .. } => "ResetHistory",
            Command::DuePrompts { .. } => "DuePrompts",
            Command::NewPrompts { .. } => "NewPrompts",
            Command::SetSetting { .. } => "SetSetting",
            Command::SetScheduler { .. } => "SetScheduler",
            Command::SetRetention { .. } => "SetRetention",
            Command::SetWeights { .. } => "SetWeights",
            Command::History { .. } => "History",
            Command::Stats { .. } => "Stats",
            _ => return None,
        })
    }

    fn view_name(&self) -> Option<&'static str> {
        Some(match self {
            Command::CreateView { .. } => "CreateView",
            Command::RenameView { .. } => "RenameView",
            Command::DeleteView { .. } => "DeleteView",
            Command::UpdateView { .. } => "UpdateView",
            Command::CardsForView { .. } => "CardsForView",
            _ => return None,
        })
    }

    fn transfer_name(&self) -> Option<&'static str> {
        Some(match self {
            Command::ImportArchive { .. } => "ImportArchive",
            Command::ExportLibrary { .. } => "ExportLibrary",
            Command::ImportCsv { .. } => "ImportCsv",
            Command::ExportCsv { .. } => "ExportCsv",
            Command::ImportMarkdown { .. } => "ImportMarkdown",
            Command::ExportMarkdown { .. } => "ExportMarkdown",
            _ => return None,
        })
    }
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
    MaskGroups(Vec<u32>),
    Masks(Vec<MaskRow>),
    Draft(Option<String>),
    Stats(Stats),
    Cards(Vec<CardSummary>),
    Imported(ImportReport),
    Exported,
    Ok,
}

impl Outcome {
    /// Whether this answer came from a command that wrote to the library.
    pub fn writes(&self) -> bool {
        match self {
            Outcome::Deck(_)
            | Outcome::Card(_)
            | Outcome::View(_)
            | Outcome::Changed(_)
            | Outcome::Graded { .. }
            | Outcome::Imported(_)
            | Outcome::Ok => true,
            Outcome::MaskGroups(_) | Outcome::Masks(_) | Outcome::Exported => false,
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
#[non_exhaustive]
pub struct Event {
    pub id: u64,
    pub result: Result<Outcome, StoreError>,
}

impl Event {
    /// A store completion event carrying `id` and `result`.
    pub fn new(id: u64, result: Result<Outcome, StoreError>) -> Self {
        Self { id, result }
    }
}

/// The only way the UI reaches the store.
pub trait Store: Send + Sync {
    fn snapshot(&self) -> Arc<Snapshot>;
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
    fn set_fields(&self, card: Id, fields: &[(String, String)]) -> Result<u64, StoreError>;
    fn set_card_template(
        &self,
        card: Id,
        template: Option<&str>,
        name: Option<&str>,
    ) -> Result<u64, StoreError>;
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
    fn due_prompts(
        &self,
        now_ms: i64,
        offset_ms: i64,
        seed: u64,
        deck: Option<Id>,
    ) -> Result<u64, StoreError>;
    fn new_prompts(
        &self,
        now_ms: i64,
        offset_ms: i64,
        seed: u64,
        deck: Option<Id>,
    ) -> Result<u64, StoreError>;
    fn cards_for_view(&self, spec: &ViewSpec) -> Result<u64, StoreError>;
    fn set_setting(&self, key: SettingKey, value: SettingValue) -> Result<u64, StoreError>;
    fn set_scheduler(&self, kind: SchedulerKind) -> Result<u64, StoreError>;
    fn set_retention(&self, percent: u32) -> Result<u64, StoreError>;
    fn set_weights(&self, weights: &[f64]) -> Result<u64, StoreError>;
    fn set_card_reverse(&self, card: Id, reverse: Option<bool>) -> Result<u64, StoreError>;
    fn set_deck_reverse(&self, deck: Id, reverse: Option<bool>) -> Result<u64, StoreError>;
    fn set_deck_view(&self, deck: Id, view: ViewMode) -> Result<u64, StoreError>;
    fn move_card_to(&self, card: Id, position: u32) -> Result<u64, StoreError>;
    fn create_diagram(&self, deck: Id, image: &str, caption: &str) -> Result<u64, StoreError>;
    fn set_diagram(&self, card: Id, image: &str, caption: &str) -> Result<u64, StoreError>;
    fn add_mask(&self, card: Id, group: u32, rect: MaskRect) -> Result<u64, StoreError>;
    fn set_mask_group(&self, card: Id, rect: MaskRect, group: u32) -> Result<u64, StoreError>;
    fn remove_mask(&self, card: Id, rect: MaskRect) -> Result<u64, StoreError>;
    fn remove_mask_group(&self, card: Id, group: u32) -> Result<u64, StoreError>;
    fn clear_diagram(&self, card: Id) -> Result<u64, StoreError>;
    fn merge_masks(&self, card: Id, rects: &[MaskRect], group: u32) -> Result<u64, StoreError>;
    fn mask_groups(&self, card: Id) -> Result<u64, StoreError>;
    fn masks(&self, card: Id) -> Result<u64, StoreError>;
    fn create_view(&self, spec: &ViewSpec, name: &str) -> Result<u64, StoreError>;
    fn rename_view(&self, id: Id, name: &str) -> Result<u64, StoreError>;
    fn delete_view(&self, id: Id) -> Result<u64, StoreError>;
    fn update_view(&self, id: Id, spec: &ViewSpec) -> Result<u64, StoreError>;
    fn set_pinned(&self, card: Id, pinned: bool) -> Result<u64, StoreError>;
    fn history(&self, card: Id) -> Result<u64, StoreError>;
    fn stats(&self, from_ms: i64, to_ms: i64) -> Result<u64, StoreError>;
    fn import_archive(
        &self,
        archive: &std::path::Path,
        attachments: &std::path::Path,
    ) -> Result<u64, StoreError>;
    fn export_library(&self, target: &std::path::Path) -> Result<u64, StoreError>;
    fn import_csv(&self, deck: Id, source: &std::path::Path) -> Result<u64, StoreError>;
    fn export_csv(&self, deck: Id, target: &std::path::Path) -> Result<u64, StoreError>;
    fn import_markdown(&self, deck: Id, source: &std::path::Path) -> Result<u64, StoreError>;
    fn export_markdown(&self, deck: Id, target: &std::path::Path) -> Result<u64, StoreError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn samples() -> Vec<(Command, Domain)> {
        vec![
            (
                Command::CreateDeck {
                    parent: None,
                    name: String::new(),
                },
                Domain::Deck,
            ),
            (Command::ClearDiagram { card: 1 }, Domain::Diagram),
            (
                Command::CreateCard {
                    deck: 1,
                    markdown: String::new(),
                },
                Domain::Card,
            ),
            (Command::EmptyTrash, Domain::CardState),
            (Command::History { card: 1 }, Domain::Review),
            (Command::DeleteView { id: 1 }, Domain::View),
            (
                Command::ExportLibrary {
                    target: Default::default(),
                },
                Domain::Transfer,
            ),
        ]
    }

    #[test]
    fn every_domain_names_what_it_routes() {
        for (command, domain) in samples() {
            assert!(!command.name().is_empty());
            assert_eq!(command.domain(), domain, "{command:?}");
        }
    }
}
