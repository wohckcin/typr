//! Foundational domain types shared across scheduling, sessions, views, transfer, and storage.
//!
//! Keeping these pure value types in a leaf module with no persistence imports allows `fsrs`,
//! `schedule`, `edits`, `inbox`, `name`, `reference`, `session`, and `transfer` to depend downward
//! without cycling back through `store`.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use unicode_normalization::UnicodeNormalization;

use crate::day::{DAY_MS, MINUTE_MS};
use crate::hidden;
use crate::library_view::ViewMode;

/// A deck, card, prompt or log row.
pub type Id = i64;

/// FSRS-5's published defaults, one per weight the optimiser trains.
pub const DEFAULT_WEIGHTS: [f64; 19] = [
    0.40255, 1.18385, 3.173, 15.69105, 7.1949, 0.5345, 1.4604, 0.0046, 1.54575, 0.1192, 1.01925,
    1.9395, 0.11, 0.29605, 2.2698, 0.2315, 2.9898, 0.51655, 0.6621,
];

/// How many numbers a weight list holds.
pub const WEIGHT_COUNT: usize = DEFAULT_WEIGHTS.len();

/// The retention a library gets before anyone sets one.
pub const DEFAULT_RETENTION: f64 = 0.9;

/// The narrowest and widest target FSRS accepts.
pub const MIN_RETENTION: u32 = 70;
pub const MAX_RETENTION: u32 = 97;

/// The form search and tag matching compare in: no case, no accents.
pub fn folded(text: &str) -> String {
    text.nfd()
        .filter(|character| !is_combining_mark(*character))
        .flat_map(char::to_lowercase)
        .collect()
}

fn is_combining_mark(character: char) -> bool {
    matches!(
        character as u32,
        0x0300..=0x036F | 0x1AB0..=0x1AFF | 0x1DC0..=0x1DFF | 0x20D0..=0x20FF | 0xFE20..=0xFE2F
    )
}

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

/// A prompt's state as `prompt` stores it, and as folding `review_log` rebuilds it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Folded {
    pub phase: Phase,
    pub interval_ms: i64,
    pub due_at: Option<i64>,
    pub reps: i64,
    pub lapses: i64,
}

impl Folded {
    /// A folded prompt state built from its stored fields.
    pub fn new(
        phase: Phase,
        interval_ms: i64,
        due_at: Option<i64>,
        reps: i64,
        lapses: i64,
    ) -> Self {
        Self {
            phase,
            interval_ms,
            due_at,
            reps,
            lapses,
        }
    }
}

/// What an import wrote, in the units the archive itself counts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ImportReport {
    pub decks: usize,
    pub cards: usize,
    pub reviews: usize,
    pub images: usize,
    pub masks: usize,
}

/// A deck in the tree.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Deck {
    pub id: Id,
    pub parent: Option<Id>,
    pub name: String,
    pub archived: bool,
    /// `None` inherits the parent's value, which is what makes the nearest setting win.
    pub reverse: Option<bool>,
    /// The markdown its cards are rendered from, or `None` to inherit the parent deck's.
    pub template: Option<String>,
    /// How this deck's cards are laid out. Never `None`: a deck with no stored view is a grid.
    pub view: ViewMode,
}

impl Deck {
    /// A live top-level deck with `id` and `name`.
    pub fn new(id: Id, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            ..Self::default()
        }
    }
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

/// A card as another card's reference names it: enough to show it and open it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct CardRef {
    pub id: Id,
    /// The name the card answered to when the reference was resolved.
    pub name: String,
}

/// A card as the library lists it.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct CardSummary {
    /// Where the card sits in its deck, which is the order the notebook and column views read in.
    pub position: i64,
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
    /// The first side's first line, cut short: what a list row shows under the name.
    pub preview: String,
    /// What a list shows instead of the preview where space is tight, and what a `[[…]]` resolves
    /// against. `''` for a card with no text (a diagram) and for a card nobody has renamed.
    pub name: String,
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
    /// The cards this one's text refers to, as of the last save of either side.
    pub references: Vec<CardRef>,
    /// The cards whose text refers to this one, each with the name it answered to.
    pub backlinks: Vec<CardRef>,
}

impl CardSummary {
    /// The questions this card holds, in the keys the store schedules it under.
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
        if shape.claims_reverse() && self.prompts > 1 {
            keys.push("rev".to_string());
        }
        keys
    }

    /// Whether a mode's slice of the library holds this card.
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

/// Whether the app draws dark, light, or whichever the desktop is set to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Theme {
    #[default]
    System,
    Dark,
    Light,
}

impl Theme {
    pub const ALL: [Theme; 3] = [Theme::System, Theme::Light, Theme::Dark];

    pub fn as_str(self) -> &'static str {
        match self {
            Theme::System => "system",
            Theme::Dark => "dark",
            Theme::Light => "light",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "system" => Some(Theme::System),
            "dark" => Some(Theme::Dark),
            "light" => Some(Theme::Light),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Theme::System => "System",
            Theme::Dark => "Dark",
            Theme::Light => "Light",
        }
    }
}

/// The three groups the settings screen is read in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    Review,
    Appearance,
    Library,
}

impl Group {
    pub const ALL: [Group; 3] = [Group::Review, Group::Appearance, Group::Library];

    pub fn title(self) -> &'static str {
        match self {
            Group::Review => "Review",
            Group::Appearance => "Appearance",
            Group::Library => "Library",
        }
    }
}

/// The two schedulers a library can choose between.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SchedulerKind {
    #[default]
    Fixed,
    Fsrs,
}

impl SchedulerKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SchedulerKind::Fixed => "fixed",
            SchedulerKind::Fsrs => "fsrs",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "fixed" => Some(SchedulerKind::Fixed),
            "fsrs" => Some(SchedulerKind::Fsrs),
            _ => None,
        }
    }
}

/// One setting, named.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SettingKey {
    NewPerDay,
    LearningStepMinutes,
    MaxIntervalDays,
    Scheduler,
    Retention,
    Weights,
    ReverseDefault,
    Theme,
    PreviewFontSize,
}

/// A setting's value, typed so a caller cannot hand a flag to a number.
#[derive(Clone, Debug, PartialEq)]
pub enum SettingValue {
    Number(u32),
    Flag(bool),
    Scheduler(SchedulerKind),
    Weights(Vec<f64>),
    Theme(Theme),
}

/// Settings the user can change.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Settings {
    pub new_per_day: u32,
    pub learning_step_minutes: u32,
    pub max_interval_days: u32,
    pub reverse_default: bool,
    pub scheduler: SchedulerKind,
    pub retention: u32,
    pub weights: Vec<f64>,
    pub theme: Theme,
    pub preview_font_size: u32,
}

impl Settings {
    pub const DEFAULT_NEW_PER_DAY: u32 = 20;
    pub const DEFAULT_LEARNING_STEP_MINUTES: u32 = 10;
    pub const DEFAULT_MAX_INTERVAL_DAYS: u32 = 365;
    pub const DEFAULT_RETENTION: u32 = 90;
    pub const DEFAULT_PREVIEW_FONT_SIZE: u32 = 14;

    /// How long a learning step waits, in milliseconds.
    pub fn learning_step_ms(&self) -> i64 {
        i64::from(self.learning_step_minutes) * MINUTE_MS
    }

    /// The furthest a remembered card can be scheduled, in milliseconds.
    pub fn max_interval_ms(&self) -> i64 {
        i64::from(self.max_interval_days) * DAY_MS
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            new_per_day: Self::DEFAULT_NEW_PER_DAY,
            learning_step_minutes: Self::DEFAULT_LEARNING_STEP_MINUTES,
            max_interval_days: Self::DEFAULT_MAX_INTERVAL_DAYS,
            reverse_default: false,
            scheduler: SchedulerKind::default(),
            retention: Self::DEFAULT_RETENTION,
            weights: DEFAULT_WEIGHTS.to_vec(),
            theme: Theme::default(),
            preview_font_size: Self::DEFAULT_PREVIEW_FONT_SIZE,
        }
    }
}

/// Which slice of the library a list shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CardMode {
    #[default]
    Active,
    Archived,
    Trash,
}

/// How a list is ordered.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Sort {
    #[default]
    LastEdited,
    Created,
    FirstSide,
    DueDate,
    Reviews,
}

/// Which way an order runs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SortDir {
    #[default]
    Ascending,
    Descending,
}

/// Everything a list is filtered and ordered by.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct ViewSpec {
    pub deck: Option<Id>,
    pub query: String,
    pub tags: Vec<String>,
    pub mode: CardMode,
    pub pinned_only: bool,
    pub sort: Sort,
    pub dir: SortDir,
}

/// A view as the store holds it.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct SavedView {
    pub id: Id,
    pub name: String,
    pub spec: ViewSpec,
}

/// Everything a view may read without a round trip.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct Snapshot {
    pub decks: Vec<Deck>,
    pub cards: Vec<CardSummary>,
    pub views: Vec<SavedView>,
    pub settings: Settings,
    pub revision: u64,
}

/// One node of the tag tree: a path, the cards under it, and the paths below it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct TagBranch {
    pub path: String,
    pub cards: u32,
    pub children: Vec<TagBranch>,
}

impl Snapshot {
    /// Every tag in the library as a tree of paths, so a screen can list a tag's children under it.
    pub fn tag_tree(&self) -> Vec<TagBranch> {
        let mut cards_under: BTreeMap<String, BTreeSet<Id>> = BTreeMap::new();
        let mut written: HashMap<String, String> = HashMap::new();
        for card in &self.cards {
            for tag in &card.tags {
                let segments = tag
                    .split('/')
                    .filter(|segment| !segment.trim().is_empty())
                    .collect::<Vec<_>>();
                for end in 1..=segments.len() {
                    let path = segments[..end].join("/");
                    let key = folded(&path);
                    written.entry(key.clone()).or_insert(path);
                    cards_under.entry(key).or_default().insert(card.id);
                }
            }
        }
        let mut roots: Vec<TagBranch> = Vec::new();
        for key in cards_under.keys() {
            let path = written.get(key).expect("every counted path is written");
            let mut node = &mut roots;
            let mut walked = String::new();
            for segment in path.split('/') {
                if !walked.is_empty() {
                    walked.push('/');
                }
                walked.push_str(segment);
                let at = match node
                    .iter()
                    .position(|branch| folded(&branch.path) == folded(&walked))
                {
                    Some(at) => at,
                    None => {
                        node.push(TagBranch {
                            path: walked.clone(),
                            cards: 0,
                            children: Vec::new(),
                        });
                        node.len() - 1
                    }
                };
                node = &mut node[at].children;
            }
        }
        count_branches(&mut roots, &cards_under);
        sort_branches(&mut roots);
        roots
    }

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
}

fn count_branches(branches: &mut [TagBranch], counts: &BTreeMap<String, BTreeSet<Id>>) {
    for branch in branches {
        let key = folded(&branch.path);
        branch.cards = counts
            .get(&key)
            .map_or(0, |ids| u32::try_from(ids.len()).unwrap_or(u32::MAX));
        count_branches(&mut branch.children, counts);
    }
}

fn sort_branches(branches: &mut Vec<TagBranch>) {
    branches.sort_by(|left, right| {
        folded(&left.path)
            .cmp(&folded(&right.path))
            .then_with(|| left.path.cmp(&right.path))
    });
    for branch in branches {
        sort_branches(&mut branch.children);
    }
}

/// One prompt that is due now, with the card it belongs to.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct DuePrompt {
    pub card: Id,
    pub key: String,
    pub due_at: i64,
}

/// A card still waiting for its first lesson.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct NewPrompt {
    pub card: Id,
    pub key: String,
}

/// Grades recorded in a window, counting real reviews only.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Stats {
    pub reviews: u32,
    pub lapses: u32,
}

/// One row of a card's history, learning cycles included.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
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

impl ReviewRow {
    /// One review history record built from its stored columns.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        card: Id,
        key: impl Into<String>,
        phase: Phase,
        at: i64,
        grade: Grade,
        interval_ms: i64,
        due_at: Option<i64>,
        duration_ms: Option<i64>,
    ) -> Self {
        Self {
            card,
            key: key.into(),
            phase,
            at,
            grade,
            interval_ms,
            due_at,
            duration_ms,
        }
    }
}

/// A rectangle over a diagram, as fractions of the image's width and height.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
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
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct MaskRow {
    pub group: u32,
    pub rect: MaskRect,
}

impl MaskRow {
    /// A mask row placing `rect` in `group`.
    pub fn new(group: u32, rect: MaskRect) -> Self {
        Self { group, rect }
    }
}
