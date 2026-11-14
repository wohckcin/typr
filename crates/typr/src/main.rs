//! `typr` — a local-first flashcard app on Rune.

use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use rune::tok::{self, type_};
use rune::{AppSpec, Cx, El, TextEdit, Tx, Waker, col, row, text};
use typr::attachments::Attachments;
use typr::day_offset;
use typr::editor::{self, Editor};
use typr::history::{self, History};
use typr::keymap::{self, Host as _};
use typr::library::{self, Library};
use typr::logging;
use typr::paths;
use typr::screen::Screen;
use typr::settings;
use typr::study::{self, Host};
use typr_core::clock::{Clock, SystemClock};
use typr_core::keymap::{Command, Surface};
use typr_core::screen::Queue;
use typr_core::session::{Entry, Mode, Session};
use typr_core::store::{
    CardKind, CardMode, CardRef, CardSummary, Deck, Grade, Handle, Id, MaskRect, MaskRow, Outcome,
    SavedView, SettingKey, SettingValue, Settings, Snapshot, Sort, SortDir, Store, StoreError,
    ViewSpec, WorkerStatus,
};
use typr_core::{day, hidden};

/// A named starting state for a headless `--scene` run.
type Scene = (&'static str, fn(&mut State));

/// How long a command may take before its answer is worth a record.
const SLOW_COMMAND: Duration = Duration::from_millis(250);

/// What to record about a command that took long enough to matter, and nothing when it did not.
fn slow_record(name: &str, elapsed: Duration) -> Option<String> {
    (elapsed > SLOW_COMMAND).then(|| format!("{name} answered after {} ms", elapsed.as_millis()))
}

/// Record the worker's end, then reach the log file one last time.
///
/// Runs in `main` after the run loop, never from `Drop`: the handle is dropped during teardown
/// inside `rune::run`, and a flush there would come before its own record.
fn finish(status: Option<&WorkerStatus>) {
    for (level, message) in quit_records(status) {
        log::log!(target: "typr", level, "{message}");
    }
    logging::shutdown();
}

/// What a quit leaves in the log. A worker that ended badly says so; a quiet end is written down
/// too, because otherwise a clean quit and a killed process look the same afterwards.
fn quit_records(status: Option<&WorkerStatus>) -> Vec<(log::Level, String)> {
    match status.and_then(WorkerStatus::ended) {
        Some(error) => vec![(
            log::Level::Error,
            format!(
                "the library worker ended badly: {}",
                logging::describe(&*error)
            ),
        )],
        None => vec![(
            log::Level::Debug,
            "the window closed and the worker ended quietly".to_string(),
        )],
    }
}

/// The worker's end, kept where it outlives the state that owns the handle.
static WORKER: OnceLock<WorkerStatus> = OnceLock::new();

struct State {
    /// The screen on display.
    screen: Screen,
    library: Library,
    editor: Editor,
    session: Session,
    /// The library's writer thread, absent until `setup` has opened it.
    store: Option<Handle>,
    /// How many prompts are due at this moment, for the summary.
    due_now: usize,
    /// The queue the user asked for, waiting on the store's answer.
    wanted: Option<Queue>,
    /// The due-queue request in flight, so its answer is not mistaken for another.
    due_request: Option<u64>,
    /// The new-queue request in flight.
    new_request: Option<u64>,
    /// Tags typed on a card that does not exist yet; they go on as soon as it has an id.
    pending_tags: Vec<String>,
    /// The draft last written down, so typing one key does not write it again.
    last_draft: String,
    /// The draft request in flight, whose answer fills a new card.
    draft_request: Option<u64>,
    /// The mid-session due request in flight, whose answer joins the running queue.
    arrivals: Option<u64>,
    /// The deck the running session was opened for, so arrivals stay inside it.
    session_deck: Option<Id>,
    /// The size the picture pane was laid out at, which places a diagram face's covers.
    picture_pane: (f32, f32),
    /// The history request in flight, with the card it was made for.
    history_request: Option<(u64, Id)>,
    /// The create-card request in flight, whose answer carries the new id.
    card_request: Option<u64>,
    /// The last thing that went wrong, shown until something else replaces it.
    failure: Option<String>,
    /// A scene brought its own library, so opening the real one must not replace it.
    seeded: bool,
    /// The seed the next shuffle uses, so each press reorders differently.
    seed: u64,
    /// A new card waiting for the deck that has to be created for it.
    pending_new_card: Option<String>,
    /// A new diagram waiting for the card that has to be created for it.
    pending_new_diagram: Option<(String, String)>,
    /// Fields typed before their card existed, stored once it does.
    pending_fields: Vec<(String, String)>,
    /// The rectangles drawn before the card existed. They are sent once the card has an id,
    /// because a rectangle has nothing to hang on until then.
    pending_masks: Vec<(u32, MaskRect)>,
    deck_request: Option<u64>,
    /// Every command that went out and has not been answered: what it was, and when it left.
    sent: HashMap<u64, (&'static str, Instant)>,
    /// The shortcut panel, and what is typed into it.
    panel: keymap::panel::Panel,
    /// The command palette, and what is typed into it.
    palette: keymap::palette::Palette,
    /// The cram selection request in flight, whose answer is the run's queue.
    cram_request: Option<u64>,
    /// The filter the cram session walks, kept so the summary can save it as a view.
    cram_spec: Option<ViewSpec>,
    /// The name the summary will save that filter under.
    cram_name: TextEdit,
    /// The save-view request in flight.
    cram_save_request: Option<u64>,
    /// Whether the filter has been saved, so a second press creates no second view.
    cram_saved: bool,
    /// The text each setting row is editing, so a refusal can leave it on screen.
    setting_inputs: HashMap<SettingKey, TextEdit>,
    /// The screen to go back to when settings closes.
    settings_from: Option<Screen>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            screen: Screen::Library,
            library: Library::default(),
            editor: Editor::default(),
            session: Session::empty(),
            store: None,
            due_now: 0,
            wanted: None,
            due_request: None,
            new_request: None,
            pending_tags: Vec::new(),
            last_draft: String::new(),
            draft_request: None,
            arrivals: None,
            session_deck: None,
            history_request: None,
            card_request: None,
            failure: None,
            seeded: false,
            picture_pane: (0.0, 0.0),
            seed: 1,
            pending_new_card: None,
            pending_new_diagram: None,
            pending_fields: Vec::new(),
            pending_masks: Vec::new(),
            deck_request: None,
            sent: HashMap::new(),
            panel: keymap::panel::Panel::default(),
            palette: keymap::palette::Palette::default(),
            cram_request: None,
            cram_spec: None,
            cram_name: TextEdit::default(),
            cram_save_request: None,
            cram_saved: false,
            setting_inputs: SettingKey::ALL
                .into_iter()
                .map(|key| (key, TextEdit::default()))
                .collect(),
            settings_from: None,
        }
    }
}

impl editor::Host for State {
    fn editor(&self) -> &Editor {
        &self.editor
    }

    fn editor_mut(&mut self) -> &mut Editor {
        &mut self.editor
    }

    fn persist(&mut self) -> bool {
        State::persist(self)
    }

    fn send_masks(&mut self, card: Id, change: &crate::editor::MaskChange) -> bool {
        let Some(store) = &self.store else {
            self.say("The library is not open, so the rectangle was not stored.".to_string());
            return false;
        };
        let sent = match change {
            crate::editor::MaskChange::Add { group, rect } => store.add_mask(card, *group, *rect),
            crate::editor::MaskChange::Regroup { rects, group } => {
                store.merge_masks(card, rects, *group)
            }
            crate::editor::MaskChange::Remove { rect } => store.remove_mask(card, *rect),
        };
        match sent {
            Ok(_) => true,
            Err(error) => {
                self.fail(&error);
                false
            }
        }
    }

    fn close_editor(&mut self) {
        self.screen = Screen::Library;
    }

    fn note_typing(&mut self) {
        let text = self.editor.buffer.text.clone();
        if text == self.last_draft {
            return;
        }
        let key = self.editor.draft_key();
        let sent = match &self.store {
            Some(store) if text.trim().is_empty() => store.set_draft(key, None),
            Some(store) => store.set_draft(key, Some(&text)),
            None => return,
        };
        if let Err(error) = sent {
            self.fail(&error);
            return;
        }
        self.last_draft = text;
    }

    fn leave_editor(&mut self) {
        if self.editor.request_leave() == editor::Leaving::Leave {
            self.close_editor();
        }
    }

    fn set_tags(&mut self, card: Id, tags: &[String]) {
        let sent = match self.store.as_ref() {
            Some(store) => store.set_tags(card, tags),
            None => Err(StoreError::Closed),
        };
        if let Err(error) = sent {
            self.fail(&error);
        }
    }

    fn open_card(&mut self, card: Id) {
        <State as crate::library::Host>::open_card(self, card);
    }

    fn card_names(&self) -> Vec<String> {
        let editing = self.editor.card;
        let mut names: Vec<String> = self
            .library
            .snapshot()
            .cards
            .iter()
            .filter(|card| {
                Some(card.id) != editing && card.trashed_at.is_none() && !card.name.is_empty()
            })
            .map(|card| card.name.clone())
            .collect();
        names.sort_by_key(|name| typr_core::store::folded(name));
        names.dedup_by(|a, b| typr_core::store::folded(a) == typr_core::store::folded(b));
        names
    }

    fn embed(&self, name: &str) -> Option<String> {
        self.embed_body(name)
    }
}

/// How many whole study days a prompt is past its due date, which is what decides whether a
/// shuffle may move it. A prompt that is not due yet counts as zero.
fn overdue_days(now_ms: i64, due_at_ms: i64, offset_ms: i64) -> i64 {
    (day::study_day(now_ms, offset_ms) - day::study_day(due_at_ms, offset_ms)).max(0)
}

/// `1 card`, `2 cards`, so a report line never reads like a template.
fn count_of(count: usize, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

/// The lower-case extension a transfer reads its format from.
fn transfer_format(path: &std::path::Path) -> Option<String> {
    path.extension()
        .and_then(std::ffi::OsStr::to_str)
        .map(str::to_ascii_lowercase)
}

/// Remember the id of a command that went out, answering whether it did, and say why when it did
/// not.
fn remember(
    in_flight: &mut HashMap<u64, (&'static str, Instant)>,
    name: &'static str,
    request: &mut Option<u64>,
    failure: &mut Option<String>,
    sent: Result<u64, StoreError>,
) -> bool {
    match sent {
        Ok(id) => {
            in_flight.insert(id, (name, Instant::now()));
            *request = Some(id);
            true
        }
        Err(error) => {
            log::warn!(target: "typr", "{}", logging::describe(&error));
            *failure = Some(error.to_string());
            false
        }
    }
}

impl State {
    /// Say what went wrong on the failure line, and put the whole chain in the log.
    fn fail(&mut self, error: &StoreError) {
        log::warn!(target: "typr", "{}", logging::describe(error));
        self.failure = Some(error.to_string());
    }

    /// Record a command whose answer took long enough to matter, then forget it.
    fn note_round_trip(&mut self, id: u64) {
        if let Some((name, started)) = self.sent.remove(&id)
            && let Some(line) = slow_record(name, started.elapsed())
        {
            log::warn!(target: "typr", "{line}");
        }
    }

    /// Note when a command went out, or say why it could not.
    /// Open the settings screen, remembering the screen it was opened from.
    fn open_settings(&mut self) {
        if self.screen != Screen::Settings {
            self.settings_from = Some(self.screen);
        }
        let settings = self.library.snapshot().settings.clone();
        for key in SettingKey::ALL {
            let text = settings.value(key).text();
            self.setting_inputs
                .entry(key)
                .or_default()
                .replace_text(text);
        }
        self.screen = Screen::Settings;
    }

    /// Close settings and return to whatever was on screen, with a session at the card it was on.
    fn leave_settings(&mut self) {
        self.screen = self.settings_from.take().unwrap_or(Screen::Library);
    }

    /// Store a value the screen has already parsed; a refusal stays on the failure line.
    fn store_setting(&mut self, key: SettingKey, value: SettingValue) {
        if let Some(input) = self.setting_inputs.get_mut(&key) {
            input.replace_text(value.text());
        }
        let sent = match &self.store {
            Some(store) => store.set_setting(key, value),
            None => Err(StoreError::Closed),
        };
        self.track("settings", sent);
    }

    fn track(&mut self, name: &'static str, result: Result<u64, StoreError>) {
        match result {
            Ok(id) => {
                self.sent.insert(id, (name, Instant::now()));
            }
            Err(error) => self.fail(&error),
        }
    }

    /// Say something that is not a store error, and record it too.
    fn say(&mut self, message: String) {
        log::warn!(target: "typr", "{message}");
        self.failure = Some(message);
    }

    /// Send a transfer and put its progress on the failure line; its answer replaces that line.
    fn send_transfer(&mut self, name: &'static str, sent: Result<u64, StoreError>) -> Option<u64> {
        match sent {
            Ok(id) => {
                self.sent.insert(id, (name, Instant::now()));
                self.report(
                    if name == "import" {
                        "Importing…"
                    } else {
                        "Exporting…"
                    }
                    .to_string(),
                );
                Some(id)
            }
            Err(error) => {
                self.fail(&error);
                None
            }
        }
    }

    /// Say what a transfer did on the line a failure would use, so success is not silent.
    fn report(&mut self, message: String) {
        log::info!(target: "typr", "{message}");
        self.failure = Some(message);
    }

    /// Import a file the user chose, picking the reader from its extension.
    fn import_path(&mut self, path: std::path::PathBuf) -> Option<u64> {
        let Some(format) = transfer_format(&path) else {
            self.say(format!(
                "{} has no extension, so its format is unknown.",
                path.display()
            ));
            return None;
        };
        if !matches!(format.as_str(), "mochi" | "csv" | "md" | "markdown") {
            self.say(format!(
                "A .{format} file is not something this library can import."
            ));
            return None;
        }
        let deck = if matches!(format.as_str(), "csv" | "md" | "markdown") {
            match self.library.selected_deck {
                Some(deck) => Some(deck),
                None => {
                    self.say(
                        "Choose a deck first: this format holds one deck's cards.".to_string(),
                    );
                    return None;
                }
            }
        } else {
            None
        };
        let Some(store) = self.store.as_ref() else {
            self.say("There is no library to import into.".to_string());
            return None;
        };
        let sent = match format.as_str() {
            "mochi" => store.import_archive(&path, self.editor.attachments.root()),
            "csv" => store.import_csv(deck?, &path),
            _ => store.import_markdown(deck?, &path),
        };
        self.send_transfer("import", sent)
    }

    /// Export the library, or its selected deck, to a file the user chose.
    fn export_path(&mut self, path: std::path::PathBuf) -> Option<u64> {
        let Some(format) = transfer_format(&path) else {
            self.say(format!(
                "{} has no extension, so the format to write is unknown.",
                path.display()
            ));
            return None;
        };
        if !matches!(format.as_str(), "typr" | "csv" | "md" | "markdown") {
            self.say(format!(
                "A .{format} file is not something this library can export."
            ));
            return None;
        }
        let deck = if matches!(format.as_str(), "csv" | "md" | "markdown") {
            match self.library.selected_deck {
                Some(deck) => Some(deck),
                None => {
                    self.say(
                        "Choose a deck first: this format holds one deck's cards.".to_string(),
                    );
                    return None;
                }
            }
        } else {
            None
        };
        let Some(store) = self.store.as_ref() else {
            self.say("There is no library to export from.".to_string());
            return None;
        };
        let sent = match format.as_str() {
            "typr" => store.export_library(&path),
            "csv" => store.export_csv(deck?, &path),
            _ => store.export_markdown(deck?, &path),
        };
        self.send_transfer("export", sent)
    }

    /// A deck name nothing else is called, so asking for a new deck always works.
    fn free_deck_name(&self) -> String {
        let taken: Vec<&str> = self
            .library
            .snapshot()
            .decks
            .iter()
            .map(|deck| deck.name.as_str())
            .collect();
        let mut index = 1;
        loop {
            let candidate = format!("Deck {index}");
            if !taken.contains(&candidate.as_str()) {
                return candidate;
            }
            index += 1;
        }
    }
}

impl library::Host for State {
    fn open_settings(&mut self) {
        State::open_settings(self);
    }

    fn keycaps(&self) -> bool {
        keymap::Host::wide_enough(self)
    }

    fn library(&self) -> &Library {
        &self.library
    }

    fn library_mut(&mut self) -> &mut Library {
        &mut self.library
    }

    fn archive_card(&mut self, card: Id) {
        let sent = match self.store.as_ref() {
            Some(store) => store.archive_card(card, true),
            None => Err(StoreError::Closed),
        };
        self.track("archive_card", sent);
    }

    fn set_card_reverse(&mut self, card: Id, reverse: Option<bool>) {
        let sent = match self.store.as_ref() {
            Some(store) => store.set_card_reverse(card, reverse),
            None => Err(StoreError::Closed),
        };
        self.track("set_card_reverse", sent);
    }

    fn set_deck_reverse(&mut self, deck: Id, reverse: Option<bool>) {
        let sent = match self.store.as_ref() {
            Some(store) => store.set_deck_reverse(deck, reverse),
            None => Err(StoreError::Closed),
        };
        self.track("set_deck_reverse", sent);
    }

    fn trash_card(&mut self, card: Id) {
        let sent = match self.store.as_ref() {
            Some(store) => store.trash_card(card),
            None => Err(StoreError::Closed),
        };
        self.track("trash_card", sent);
    }

    fn open_card(&mut self, card: Id) {
        let summary = self
            .library
            .snapshot()
            .cards
            .iter()
            .find(|own| own.id == card)
            .cloned();
        let Some(summary) = summary else {
            return;
        };
        let deck_name = self
            .library
            .snapshot()
            .decks
            .iter()
            .find(|deck| deck.id == summary.deck)
            .map(|deck| deck.name.clone());
        self.editor = Editor::new(Some(card), summary.markdown.clone());
        self.editor.load(&summary, deck_name.as_deref());
        self.editor.load_tags(&summary.tags);
        self.editor.set_reverse(summary.reverse);
        self.last_draft = summary.markdown.clone();
        self.pending_tags.clear();
        self.screen = Screen::Editor;
    }

    fn move_card(&mut self, card: Id, deck: Id) {
        let sent = match self.store.as_ref() {
            Some(store) => store.move_card(card, deck),
            None => Err(StoreError::Closed),
        };
        self.track("move_card", sent);
    }

    fn new_card(&mut self) {
        self.editor = Editor::new(None, String::new());
        // A deck that carries a template types its new cards through fields, so the table is
        // there before the card is and the text is rendered rather than hand-written.
        if let Some(deck) = self.deck_for_new_card()
            && let Some(found) = self
                .library
                .snapshot()
                .decks
                .iter()
                .find(|own| own.id == deck)
                .cloned()
            && let Some(template) = found.template
        {
            self.editor.start(&template, deck, Some(&found.name));
        }
        self.last_draft.clear();
        self.pending_tags.clear();
        self.screen = Screen::Editor;
        // A card that was typed and never saved is still waiting under the draft key of zero.
        let sent = match &self.store {
            Some(store) => store.draft(0),
            None => return,
        };
        remember(
            &mut self.sent,
            "draft",
            &mut self.draft_request,
            &mut self.failure,
            sent,
        );
    }

    fn new_deck(&mut self) {
        let parent = self.library.selected_deck;
        let name = self.free_deck_name();
        let sent = match self.store.as_ref() {
            Some(store) => store.create_deck(parent, &name),
            None => Err(StoreError::Closed),
        };
        self.track("create_deck", sent);
    }

    fn rename_deck(&mut self, deck: Id, name: &str) {
        let sent = match self.store.as_ref() {
            Some(store) => store.rename_deck(deck, name),
            None => Err(StoreError::Closed),
        };
        self.track("rename_deck", sent);
    }

    fn set_deck_template(&mut self, deck: Id, template: Option<&str>) {
        let sent = match self.store.as_ref() {
            Some(store) => store.set_deck_template(deck, template),
            None => Err(StoreError::Closed),
        };
        self.track("set_deck_template", sent);
    }

    fn create_view(&mut self, name: &str) {
        let spec = self.library.spec();
        let sent = match self.store.as_ref() {
            Some(store) => store.create_view(&spec, name),
            None => Err(StoreError::Closed),
        };
        self.track("create_view", sent);
    }

    fn rename_view(&mut self, id: Id, name: &str) {
        let sent = match self.store.as_ref() {
            Some(store) => store.rename_view(id, name),
            None => Err(StoreError::Closed),
        };
        self.track("rename_view", sent);
    }

    fn delete_view(&mut self, id: Id) {
        let sent = match self.store.as_ref() {
            Some(store) => store.delete_view(id),
            None => Err(StoreError::Closed),
        };
        // The list forgets the view at once: a row that goes on filtering after its delete was
        // pressed would read as a lost click.
        self.library.close_view_name();
        if self.library.selected_view == Some(id) {
            // The view's own filter goes with it; keeping the search and the mode would leave a
            // list that no row on screen explains.
            self.library.forget_filters();
        }
        self.track("delete_view", sent);
    }

    fn update_view(&mut self, id: Id, spec: &typr_core::store::ViewSpec) {
        let sent = match self.store.as_ref() {
            Some(store) => store.update_view(id, spec),
            None => Err(StoreError::Closed),
        };
        self.track("update_view", sent);
    }

    fn set_pinned(&mut self, card: Id, pinned: bool) {
        let sent = match self.store.as_ref() {
            Some(store) => store.set_pinned(card, pinned),
            None => Err(StoreError::Closed),
        };
        self.track("set_pinned", sent);
    }

    fn move_deck(&mut self, deck: Id, parent: Option<Id>) {
        let sent = match self.store.as_ref() {
            Some(store) => store.move_deck(deck, parent),
            None => Err(StoreError::Closed),
        };
        self.track("move_deck", sent);
    }

    fn archive_deck(&mut self, deck: Id) {
        let sent = match self.store.as_ref() {
            Some(store) => store.archive_deck(deck, true),
            None => Err(StoreError::Closed),
        };
        self.track("archive_deck", sent);
    }

    fn unarchive_deck(&mut self, deck: Id) {
        let sent = match self.store.as_ref() {
            Some(store) => store.archive_deck(deck, false),
            None => Err(StoreError::Closed),
        };
        self.track("archive_deck", sent);
    }

    fn restore_card(&mut self, card: Id) {
        // The archive is undone by un-archiving; the trash by restoring. Which one depends on the
        // view the button sits in, and that is the list's mode.
        let from_trash = self.library.mode == CardMode::Trash;
        let sent = match self.store.as_ref() {
            Some(store) if from_trash => store.restore_card(card),
            Some(store) => store.archive_card(card, false),
            None => Err(StoreError::Closed),
        };
        self.track("restore_card", sent);
    }

    fn empty_trash(&mut self) {
        let sent = match self.store.as_ref() {
            Some(store) => store.empty_trash(),
            None => Err(StoreError::Closed),
        };
        self.track("empty_trash", sent);
    }

    fn import_file(&mut self, path: std::path::PathBuf) -> Option<u64> {
        self.import_path(path)
    }

    fn export_file(&mut self, path: std::path::PathBuf) -> Option<u64> {
        self.export_path(path)
    }
}

impl history::Host for State {
    fn history(&self) -> &History {
        self.editor.history()
    }

    fn history_mut(&mut self) -> &mut History {
        self.editor.history_mut()
    }
}

impl study::Host for State {
    fn open_settings(&mut self) {
        State::open_settings(self);
    }

    fn keycaps(&self) -> bool {
        keymap::Host::wide_enough(self)
    }

    fn session(&self) -> &Session {
        &self.session
    }

    fn session_mut(&mut self) -> &mut Session {
        &mut self.session
    }

    fn embed(&self, name: &str) -> Option<String> {
        self.embed_body(name)
    }

    fn picture(&self, src: &str) -> Option<rune::Image> {
        self.editor.attachments.picture(src)
    }

    fn picture_pane(&self) -> (f32, f32) {
        self.picture_pane
    }

    fn set_picture_pane(&mut self, width: f32, height: f32) {
        self.picture_pane = (width, height);
    }

    fn submit_grade(&mut self, card: Id, key: &str, grade: Grade) -> Result<u64, StoreError> {
        if self.session.mode() == Mode::Cram {
            // A cram answer stays in the session: it schedules nothing, so no request goes out and
            // no answer is coming back. The id is the one an unsent command carries.
            self.session.answer_here();
            if self.session.finished() {
                self.screen = Screen::Summary;
            }
            return Ok(0);
        }
        let Some(store) = &self.store else {
            self.fail(&StoreError::Closed);
            return Err(StoreError::Closed);
        };
        let sent = store.grade(card, key, grade, None);
        // A grade is a command like any other: it is timed, so a slow one is written down.
        if let Ok(id) = &sent {
            self.sent.insert(*id, ("grade", Instant::now()));
        }
        if let Err(error) = &sent {
            self.fail(error);
        }
        sent
    }

    fn due_now(&self) -> usize {
        self.due_now
    }

    fn reshuffle(&mut self) {
        self.seed = self.seed.wrapping_add(1);
        self.session.reshuffle(self.seed);
    }

    fn leave_session(&mut self) {
        self.screen = Screen::Library;
        self.session = Session::empty();
        // A question already sent has no reader once the screen is gone, and its answer must not
        // put a card into a session nobody is looking at.
        self.arrivals = None;
        self.session_deck = None;
        // The drill's filter belongs to the summary; the library holds its own again.
        self.cram_spec = None;
        self.cram_save_request = None;
        self.cram_saved = false;
    }

    fn cram_name(&self) -> &TextEdit {
        &self.cram_name
    }

    fn cram_name_field(&mut self) -> &mut TextEdit {
        &mut self.cram_name
    }

    fn cram_saved(&self) -> bool {
        self.cram_saved || self.cram_save_request.is_some()
    }

    fn save_cram_view(&mut self) {
        // Pressing twice must not leave two views behind: the same filter and name cannot be a
        // second drill, and the button reads Saved from the moment the first press goes out.
        if self.cram_saved() {
            return;
        }
        let Some(spec) = self.cram_spec.clone() else {
            return;
        };
        let name = self.cram_name.text.trim().to_string();
        if name.is_empty() {
            self.say("Name the view before saving it".to_string());
            return;
        }
        let sent = match &self.store {
            Some(store) => store.create_view(&spec, &name),
            None => Err(StoreError::Closed),
        };
        remember(
            &mut self.sent,
            "create_view",
            &mut self.cram_save_request,
            &mut self.failure,
            sent,
        );
    }
}

impl State {
    /// Open the library, unless this run is a headless snapshot or script: those draw fixtures
    /// and must never touch the real one.
    fn setup(&mut self, waker: Waker) {
        if rune::tooling::is_headless() {
            return;
        }
        // The log is the only record that outlives a crash, so it is installed before anything
        // else can fail.
        let log_path = paths::logs_root_from_environment().and_then(|logs| {
            logging::install(&logs, logging::level_from_env()).map_err(StoreError::Io)
        });
        let log_path = match log_path {
            Ok(path) => Some(path),
            Err(error) => {
                self.say(format!(
                    "The log could not be opened: {}",
                    logging::describe(&error)
                ));
                None
            }
        };
        let draw = waker.clone();
        let opened = paths::attachments_root_from_environment().and_then(|root| {
            let db = paths::open_from_environment()?;
            let facts = db.schema_facts();
            let database = db.path().to_path_buf();
            let handle = Handle::spawn(db, Box::new(SystemClock), waker)?;
            Ok((root, handle, facts, database))
        });
        match opened {
            Ok((root, handle, facts, database)) => {
                log::info!(
                    target: "typr",
                    "library {} (schema {} found, {} written, {} applied), log {}",
                    database.display(),
                    facts.found,
                    facts.written,
                    facts.applied,
                    log_path
                        .as_ref()
                        .map_or_else(|| "not written".to_string(), |path| path.display().to_string()),
                );
                let _ = WORKER.set(handle.status());
                if !self.seeded {
                    self.library.set_snapshot(handle.snapshot());
                }
                self.editor.attachments = Attachments::new(root, draw);
                self.store = Some(handle);
            }
            Err(error) => {
                // Every store error carries its own reason, so the line is the error itself.
                self.say(format!("The library could not be opened: {error}"));
            }
        }
    }

    /// Write the editor's buffer to the library, and mark it saved only once it was stored.
    fn persist(&mut self) -> bool {
        if !self.editor.saveable() {
            self.editor.refused = true;
            return false;
        }
        let text = self.editor.buffer.text.clone();
        let tags = self.editor.tag_list();

        // A diagram card is stored with its picture: the text it carries (if any) still goes along,
        // but the kind, the image and the caption are what make it a diagram.
        if self.editor.diagram() {
            let Some(image) = self.editor.image.clone() else {
                self.say("A diagram card needs a picture.".to_string());
                return false;
            };
            let caption = self.editor.caption.text.trim().to_string();
            let stored = match (self.editor.card, &self.store) {
                (Some(card), Some(store)) => {
                    // A diagram's prompt comes from its rectangles, so its text is not saved: the
                    // kind, the picture and the caption are what the card is.
                    let sent = store
                        .set_diagram(card, &image, &caption)
                        .and_then(|_| store.set_tags(card, &tags));
                    match sent {
                        Ok(_) => true,
                        Err(error) => {
                            self.fail(&error);
                            false
                        }
                    }
                }
                (None, Some(store)) => {
                    if self.card_request.is_some() {
                        self.say("The card is still being saved.".to_string());
                        return false;
                    }
                    // The card id comes back asynchronously, so the picture is kept until it does.
                    self.pending_new_diagram = Some((image.clone(), caption.clone()));
                    self.pending_masks = self
                        .editor
                        .masks
                        .iter()
                        .map(|mask| (mask.group, mask.rect))
                        .collect();
                    self.pending_tags = tags;
                    self.pending_new_card = Some(text.clone());
                    match self.deck_for_new_card() {
                        Some(deck) => {
                            let sent = store.create_diagram(deck, &image, &caption);
                            remember(
                                &mut self.sent,
                                "create_diagram",
                                &mut self.card_request,
                                &mut self.failure,
                                sent,
                            )
                        }
                        None => {
                            let name = self.free_deck_name();
                            let sent = store.create_deck(None, &name);
                            if !remember(
                                &mut self.sent,
                                "create_deck",
                                &mut self.deck_request,
                                &mut self.failure,
                                sent,
                            ) {
                                self.pending_new_card = None;
                                self.pending_new_diagram = None;
                                self.pending_tags.clear();
                            }
                            false
                        }
                    }
                }
                (_, None) => {
                    self.say("The library is not open, so nothing was saved.".to_string());
                    return false;
                }
            };
            if !stored {
                return false;
            }
            if let Some(store) = &self.store
                && let Err(error) = store.set_draft(self.editor.draft_key(), None)
            {
                self.fail(&error);
            }
            self.last_draft.clear();
            self.editor.save();
            return true;
        }

        // A templated card's text is not typed: its fields are stored, and the store renders the
        // text from them. A card that does not exist yet keeps its fields until it does.
        if self.editor.templated() {
            let fields = self.editor.field_pairs();
            match (self.editor.card, &self.store) {
                (Some(card), Some(store)) => {
                    let sent = store
                        .set_fields(card, &fields)
                        .and_then(|_| store.set_tags(card, &tags));
                    if let Err(error) = &sent {
                        self.fail(error);
                        return false;
                    }
                    if let Err(error) = store.set_draft(card, None) {
                        self.fail(&error);
                    }
                    self.last_draft.clear();
                    return self.editor.save();
                }
                (None, Some(store)) => {
                    if self.card_request.is_some() {
                        self.say("The card is still being saved.".to_string());
                        return false;
                    }
                    self.pending_fields = fields;
                    self.pending_tags = tags;
                    self.pending_new_card = Some(self.editor.text().to_string());
                    match self.deck_for_new_card() {
                        Some(deck) => {
                            let sent = store.create_card(deck, self.editor.text());
                            if !remember(
                                &mut self.sent,
                                "create_card",
                                &mut self.card_request,
                                &mut self.failure,
                                sent,
                            ) {
                                self.pending_fields.clear();
                                self.pending_tags.clear();
                                self.pending_new_card = None;
                                return false;
                            }
                        }
                        None => {
                            self.say("A card needs a deck to live in.".to_string());
                            self.pending_fields.clear();
                            return false;
                        }
                    }
                    // The card is still to come, so the fields are not stored yet.
                    return false;
                }
                (_, None) => {
                    self.say("The library is not open, so nothing was saved.".to_string());
                    return false;
                }
            }
        }

        let stored = match (self.editor.card, &self.store) {
            (Some(card), Some(store)) => {
                // Two commands, so a refused tag write leaves the sides saved; the card stays
                // dirty, which is what tells the reader the tags did not land.
                let sent = store
                    .save_card(card, &text)
                    .and_then(|_| store.set_tags(card, &tags));
                if let Err(error) = &sent {
                    self.fail(error);
                    false
                } else {
                    if let Err(error) = store.set_draft(card, None) {
                        self.fail(&error);
                    }
                    true
                }
            }
            (None, Some(store)) => {
                if self.card_request.is_some() {
                    self.say("The card is still being saved.".to_string());
                    return false;
                }
                self.pending_tags = tags;
                match self.deck_for_new_card() {
                    Some(deck) => {
                        let sent = store.create_card(deck, &text);
                        remember(
                            &mut self.sent,
                            "create_card",
                            &mut self.card_request,
                            &mut self.failure,
                            sent,
                        )
                    }
                    None => {
                        // A library with no deck yet: the first card brings one with it.
                        self.pending_new_card = Some(text.clone());
                        let name = self.free_deck_name();
                        let sent = store.create_deck(None, &name);
                        if !remember(
                            &mut self.sent,
                            "create_deck",
                            &mut self.deck_request,
                            &mut self.failure,
                            sent,
                        ) {
                            self.pending_new_card = None;
                            self.pending_tags.clear();
                        }
                        // The card itself is still to come, so the buffer is not saved yet:
                        // reporting it saved would lose the text if the app quit now.
                        false
                    }
                }
            }
            (_, None) => {
                self.say("The library is not open, so nothing was saved.".to_string());
                return false;
            }
        };
        if !stored {
            return false;
        }
        // The draft stops being needed the moment the card is stored.
        if let Some(store) = &self.store
            && let Err(error) = store.set_draft(self.editor.draft_key(), None)
        {
            self.fail(&error);
        }
        self.last_draft.clear();
        self.editor.save()
    }

    /// Where a card that has never been saved goes.
    fn deck_for_new_card(&self) -> Option<Id> {
        self.library.selected_deck.or_else(|| {
            self.library
                .snapshot()
                .decks
                .iter()
                .find(|deck| !deck.archived && deck.parent.is_none())
                .map(|deck| deck.id)
        })
    }

    /// Ask the store for the queue the user chose. The session opens when the answer arrives.
    fn start_session(&mut self, queue: Queue) {
        let Some(store) = &self.store else { return };
        let now = typr_core::clock::SystemClock.now_ms();
        let offset = day_offset::local_offset_ms();
        let seed = now as u64;
        let deck = self.library.selected_deck;
        let due = store.due_prompts(now, offset, seed, deck);
        remember(
            &mut self.sent,
            "due_prompts",
            &mut self.due_request,
            &mut self.failure,
            due,
        );
        if queue == Queue::New {
            let new = store.new_prompts(now, offset, seed, deck);
            remember(
                &mut self.sent,
                "new_prompts",
                &mut self.new_request,
                &mut self.failure,
                new,
            );
        }
        // The scope travels with the request: a selection changed while the answer is in flight
        // must not move the session's arrivals into another deck.
        self.session_deck = deck;
        self.wanted = Some(queue);
    }

    /// Ask the store for the cards the library's filter selects, and open a cram run over them.
    ///
    /// The filter is the one on screen, so a drill answers what the user is looking at rather than
    /// what the schedule happens to hold. The spec travels with the request: the summary saves the
    /// filter the run was started from, not whatever the library holds when the run ends.
    fn start_cram(&mut self) {
        let Some(store) = &self.store else {
            self.fail(&StoreError::Closed);
            return;
        };
        let spec = self.library.spec();
        self.cram_name.replace_text(drill_name(&spec));
        // A new drill is a new filter, and the previous one may already have been saved.
        self.cram_saved = false;
        self.cram_save_request = None;
        let sent = store.cards_for_view(&spec);
        if remember(
            &mut self.sent,
            "cards_for_view",
            &mut self.cram_request,
            &mut self.failure,
            sent,
        ) {
            self.cram_spec = Some(spec);
        }
    }

    /// Whether an arrivals answer is for the session on screen. A request already sent when the
    /// reader walked away must not put a card into a session nobody is looking at, because that
    /// session then reads as unfinished and the app stops offering a new one.
    fn takes_arrivals(&self, id: u64) -> bool {
        self.arrivals == Some(id) && matches!(self.screen, Screen::Session)
    }

    /// Turn the prompts the store answered with into session entries.
    fn session_entries(&self, prompts: Vec<(Id, String, Option<i64>)>) -> Vec<Entry> {
        let snapshot = self.store.as_ref().map(Store::snapshot).unwrap_or_default();
        let now = typr_core::clock::SystemClock.now_ms();
        let offset = day_offset::local_offset_ms();
        prompts
            .into_iter()
            .filter_map(|(id, key, due_at)| {
                let card = snapshot.cards.iter().find(|card| card.id == id)?;
                let overdue = due_at.map_or(0, |due_at| overdue_days(now, due_at, offset));

                entry_for(card, &key, overdue)
            })
            .collect()
    }

    /// Open the session over the prompts the store answered with.
    fn open_session(&mut self, queue: Queue, prompts: Vec<(Id, String, Option<i64>)>) {
        let entries = self.session_entries(prompts);
        if entries.is_empty() {
            return;
        }
        self.session = Session::new(queue, entries);
        self.screen = Screen::Session;
    }

    /// Ask the store what fell due since the session opened; the answer joins the queue.
    fn ask_for_arrivals(&mut self) {
        let Some(store) = &self.store else { return };
        let now = typr_core::clock::SystemClock.now_ms();
        let offset = day_offset::local_offset_ms();
        let sent = store.due_prompts(now, offset, now as u64, self.session_deck);
        remember(
            &mut self.sent,
            "due_prompts",
            &mut self.arrivals,
            &mut self.failure,
            sent,
        );
    }

    /// Answer what the store has said. A grade the store refused is dropped, so the card stays.
    fn drain(&mut self) -> bool {
        let Some(store) = &self.store else {
            return false;
        };
        let events = store.poll();
        let now = typr_core::clock::SystemClock.now_ms();
        let mut changed = false;
        let mut wrote = false;
        for event in events {
            changed = true;
            self.note_round_trip(event.id);
            wrote |= event.result.as_ref().is_ok_and(Outcome::writes);
            match event.result {
                Ok(Outcome::Graded { .. }) => {
                    let acknowledged = self.session.acknowledge(event.id, now);
                    // Leaving the session first must not be undone by a grade that lands after.
                    if acknowledged && matches!(self.screen, Screen::Session) {
                        if self.session.finished() {
                            self.screen = Screen::Summary;
                        } else {
                            // Something may have fallen due while the session was running.
                            self.ask_for_arrivals();
                        }
                    }
                }
                Ok(Outcome::Due(prompts)) if self.takes_arrivals(event.id) => {
                    self.arrivals = None;
                    self.due_now = prompts.iter().filter(|prompt| prompt.key == "fwd").count();
                    let arriving: Vec<(Id, String, Option<i64>)> = prompts
                        .into_iter()
                        .filter(|prompt| !self.session.holds(prompt.card))
                        .map(|prompt| (prompt.card, prompt.key, Some(prompt.due_at)))
                        .collect();
                    for entry in self.session_entries(arriving) {
                        self.session.arrive(entry);
                    }
                }
                Ok(Outcome::Due(prompts)) if self.due_request == Some(event.id) => {
                    self.due_request = None;
                    self.due_now = prompts.iter().filter(|prompt| prompt.key == "fwd").count();
                    if self.wanted == Some(Queue::Due) {
                        self.wanted = None;
                        let prompts = prompts
                            .into_iter()
                            .map(|p| (p.card, p.key, Some(p.due_at)))
                            .collect();
                        self.open_session(Queue::Due, prompts);
                    }
                }
                Ok(Outcome::History { rows, reviews }) => {
                    if let Some((_, card)) = self.history_request.filter(|(id, _)| *id == event.id)
                    {
                        self.history_request = None;
                        self.editor.history.set(card, rows, reviews);
                    }
                }
                Ok(Outcome::New(prompts)) if self.new_request == Some(event.id) => {
                    self.new_request = None;
                    if self.wanted.take() == Some(Queue::New) {
                        let prompts = prompts.into_iter().map(|p| (p.card, p.key, None)).collect();
                        self.open_session(Queue::New, prompts);
                    }
                }
                Ok(Outcome::Cards(cards)) if self.cram_request == Some(event.id) => {
                    self.cram_request = None;
                    // A run over nothing would open on a summary of no answers, so the library
                    // stays where it is and says why.
                    if cards.is_empty() {
                        self.cram_spec = None;
                        self.say("Nothing to cram: the filter selects no card".to_string());
                    } else {
                        self.session = Session::cram(cram_entries(&cards));
                        self.session_deck = None;
                        self.screen = Screen::Session;
                    }
                }
                Ok(Outcome::View(_)) if self.cram_save_request == Some(event.id) => {
                    self.cram_save_request = None;
                    self.cram_saved = true;
                }
                Ok(Outcome::Card(id)) if self.card_request == Some(event.id) => {
                    self.card_request = None;
                    self.editor.card = Some(id);
                    // The card exists now, so the buffer is saved if it still holds what was sent.
                    if self.pending_new_card.as_deref() == Some(self.editor.buffer.text.as_str()) {
                        self.editor.save();
                    }
                    self.pending_new_card = None;
                    // A diagram whose card was created as one already holds its picture; the store
                    // answers with the same card either way, so this only sets what a text card
                    // needs to become a diagram.
                    self.pending_new_diagram = None;
                    // Fields typed before the card existed are stored now that it does, which
                    // renders its text from them.
                    let fields = std::mem::take(&mut self.pending_fields);
                    if !fields.is_empty() {
                        let sent = match &self.store {
                            Some(store) => store.set_fields(id, &fields),
                            None => Err(StoreError::Closed),
                        };
                        match sent {
                            Ok(_) => {
                                self.editor.save();
                            }
                            Err(error) => self.fail(&error),
                        }
                    }
                    // Rectangles drawn before the card existed are stored now that it does.
                    for (group, rect) in std::mem::take(&mut self.pending_masks) {
                        let sent = match &self.store {
                            Some(store) => store.add_mask(id, group, rect),
                            None => Err(StoreError::Closed),
                        };
                        if let Err(error) = sent {
                            self.fail(&error);
                            break;
                        }
                    }
                    let tags = std::mem::take(&mut self.pending_tags);
                    if !tags.is_empty() {
                        let sent = match &self.store {
                            Some(store) => store.set_tags(id, &tags),
                            None => Err(StoreError::Closed),
                        };
                        if let Err(error) = sent {
                            self.fail(&error);
                        }
                    }
                }
                Ok(Outcome::Draft(text)) if self.draft_request == Some(event.id) => {
                    self.draft_request = None;
                    if let Some(text) = text {
                        self.editor.restore_draft(&text);
                        self.last_draft = self.editor.buffer.text.clone();
                    }
                }
                Ok(Outcome::Deck(id)) if self.deck_request == Some(event.id) => {
                    self.deck_request = None;
                    let diagram = self.pending_new_diagram.clone();
                    if let (Some(text), Some(store)) = (self.pending_new_card.take(), &self.store) {
                        let sent = match diagram {
                            Some((image, caption)) => store.create_diagram(id, &image, &caption),
                            None => store.create_card(id, &text),
                        };
                        if !remember(
                            &mut self.sent,
                            "create_card",
                            &mut self.card_request,
                            &mut self.failure,
                            sent,
                        ) {
                            self.pending_tags.clear();
                        }
                    }
                }
                Ok(Outcome::Imported(report)) => {
                    // A spreadsheet or Markdown import writes into a deck that already existed,
                    // so it has no deck of its own to report.
                    let across = if report.decks == 0 {
                        String::new()
                    } else {
                        format!(" across {}", count_of(report.decks, "deck", "decks"))
                    };
                    self.report(format!(
                        "Imported {}{across}: {}, {}, {}.",
                        count_of(report.cards, "card", "cards"),
                        count_of(report.reviews, "review", "reviews"),
                        count_of(report.images, "picture", "pictures"),
                        count_of(report.masks, "mask", "masks"),
                    ));
                }
                Ok(Outcome::Exported) => self.report("Exported.".to_string()),
                Ok(_) => {}
                Err(error) => {
                    self.session.refuse(event.id);
                    // A refusal leaves the offer open: the reader can press save again.
                    if self.cram_save_request == Some(event.id) {
                        self.cram_save_request = None;
                    }
                    self.fail(&error);
                }
            }
        }
        if wrote && let Some(store) = &self.store {
            self.library.set_snapshot(store.snapshot());
        }
        changed
    }
}

fn view(s: &State, cx: &mut Cx<State>) -> El {
    // The interface scale is part of the next frame: a value stored this frame lays out the next.
    cx.zoom(s.library.snapshot().settings.scale);
    let screen = match s.screen {
        Screen::Library => library::view::<State>(s),
        Screen::Editor => editor::view::<State>(s),
        Screen::Session => study::view::<State>(s),
        Screen::Summary => study::summary::<State>(s),
        Screen::Settings => settings::view::<State>(s),
    };
    let body = match &s.failure {
        None => screen,
        Some(failure) => col()
            .w_full()
            .h_full()
            .child(screen.grow(1.0))
            .child(status_bar(failure)),
    };
    let mut layer = Vec::new();
    keymap::layer::<State>(s, &mut layer);
    rune_kit::root(body, layer)
}

/// The one line at the bottom that says what went wrong, so a failed command is not silent.
fn status_bar(failure: &str) -> El {
    let t = tok::theme(true);
    row()
        .key("failure")
        .w_full()
        .padding(12.0)
        .center_y()
        .fill(t.bg)
        .child(text(failure).font(type_::LABEL).color(t.text_secondary))
}

/// Move the session's relearning steps on, and say how long until the next one lands.
fn advance_session(s: &mut State, now_ms: i64) -> (usize, Option<i64>) {
    let landed = s.session.advance_clock(now_ms);
    (landed, s.session.next_wake(now_ms))
}

fn poll(s: &mut State, _dt: f64, tx: &mut Tx) -> bool {
    let mut wake = None;
    let changed = frame_step(s, typr_core::clock::SystemClock.now_ms(), &mut wake);
    if let Some(ms) = wake {
        tx.wake_after(std::time::Duration::from_millis(ms as u64));
    }
    changed
}

/// One frame's work: land any relearning step, take the store's answers, and report when the frame
/// loop should look again. `wake` is how the delay reaches `Tx::wake_after`, and taking it as an
/// out-parameter keeps the request observable in a test.
fn frame_step(s: &mut State, now_ms: i64, wake: &mut Option<i64>) -> bool {
    let (landed, delay) = advance_session(s, now_ms);
    *wake = delay;
    s.drain() || landed > 0
}

impl State {
    /// Run the command a key asked for, on the surface it was pressed on.
    fn run_command(&mut self, command: Command) -> bool {
        match self.screen {
            Screen::Library => self.run_library(command),
            Screen::Editor => self.run_editor(command),
            Screen::Session => study::run(self, command) || self.run_common(command),
            Screen::Summary => self.run_summary(command),
            Screen::Settings => self.run_settings(command),
        }
    }

    /// Settings answers Leave, and the panel and palette it shares with every screen.
    fn run_settings(&mut self, command: Command) -> bool {
        match command {
            Command::Leave => {
                self.leave_settings();
                true
            }
            _ => self.run_common(command),
        }
    }

    /// The commands every surface answers: the panel and the palette.
    fn run_common(&mut self, command: Command) -> bool {
        match command {
            Command::ShowShortcuts => {
                self.panel.open = !self.panel.open;
                true
            }
            Command::ShowPalette => {
                keymap::palette::open(&mut self.palette);
                true
            }
            Command::Settings => {
                self.open_settings();
                true
            }
            _ => false,
        }
    }

    fn run_library(&mut self, command: Command) -> bool {
        if self.run_common(command) {
            return true;
        }
        match command {
            // A session starts from a settled library, so the keys do nothing while one is running.
            _ if !self.session.finished() => false,
            Command::StartDueSession => {
                self.start_session(Queue::Due);
                true
            }
            Command::StartNewSession => {
                self.start_session(Queue::New);
                true
            }
            Command::NewCard => {
                library::Host::new_card(self);
                true
            }
            Command::CramSession => {
                self.start_cram();
                true
            }
            _ => false,
        }
    }

    fn run_editor(&mut self, command: Command) -> bool {
        if self.run_common(command) {
            return true;
        }
        match command {
            // Leaving asks first when the buffer holds work that is not on disk.
            Command::Leave => {
                editor::Host::leave_editor(self);
                true
            }
            Command::Save => self.persist(),
            Command::AddRectangle => {
                // The keyboard fallback places a rectangle in the middle, which the arrow chords
                // then size, so a diagram can be masked without a pointer.
                if self.editor.diagram() {
                    let change = self.editor.add_rect(MaskRect::new(0.35, 0.35, 0.3, 0.3));
                    if let Some(card) = self.editor.card {
                        editor::Host::send_masks(self, card, &change);
                    }
                    true
                } else {
                    false
                }
            }
            Command::WiderRectangle | Command::NarrowerRectangle => {
                let step = if command == Command::WiderRectangle {
                    0.02
                } else {
                    -0.02
                };
                match self.editor.resize_selected(step, 0.0) {
                    Some(change) if self.editor.diagram() => {
                        if let Some(card) = self.editor.card {
                            editor::Host::send_masks(self, card, &change);
                        }
                        true
                    }
                    _ => false,
                }
            }
            Command::TallerRectangle | Command::ShorterRectangle => {
                let step = if command == Command::TallerRectangle {
                    0.02
                } else {
                    -0.02
                };
                match self.editor.resize_selected(0.0, step) {
                    Some(change) if self.editor.diagram() => {
                        if let Some(card) = self.editor.card {
                            editor::Host::send_masks(self, card, &change);
                        }
                        true
                    }
                    _ => false,
                }
            }
            Command::ToggleHistory => {
                self.editor.showing_history = !self.editor.showing_history;
                if self.editor.showing_history
                    && let (Some(card), Some(store)) = (self.editor.card, &self.store)
                {
                    match store.history(card) {
                        Ok(id) => self.history_request = Some((id, card)),
                        Err(error) => self.fail(&error),
                    }
                }
                true
            }
            _ => false,
        }
    }

    fn run_summary(&mut self, command: Command) -> bool {
        if self.run_common(command) {
            return true;
        }
        match command {
            Command::Leave => {
                self.leave_session();
                true
            }
            _ => false,
        }
    }
}

impl settings::Host for State {
    fn settings(&self) -> &Settings {
        &self.library.snapshot().settings
    }

    fn setting_input(&self, key: SettingKey) -> &TextEdit {
        &self.setting_inputs[&key]
    }

    fn setting_input_mut(&mut self, key: SettingKey) -> &mut TextEdit {
        self.setting_inputs.entry(key).or_default()
    }

    fn set_setting(&mut self, key: SettingKey, value: SettingValue) {
        self.store_setting(key, value);
    }

    fn refuse(&mut self, error: &StoreError) {
        self.fail(error);
    }

    fn leave_settings(&mut self) {
        State::leave_settings(self);
    }
}

impl keymap::Host for State {
    fn surface(&self) -> Surface {
        match self.screen {
            Screen::Library => Surface::Library,
            Screen::Editor => Surface::Editor,
            Screen::Session => Surface::Session,
            Screen::Summary => Surface::Summary,
            Screen::Settings => Surface::Settings,
        }
    }

    fn panel(&self) -> &keymap::panel::Panel {
        &self.panel
    }

    fn panel_mut(&mut self) -> &mut keymap::panel::Panel {
        &mut self.panel
    }

    fn palette(&self) -> &keymap::palette::Palette {
        &self.palette
    }

    fn palette_mut(&mut self) -> &mut keymap::palette::Palette {
        &mut self.palette
    }

    /// A keycap needs room; the library's measured width is the only width the app knows.
    fn wide_enough(&self) -> bool {
        self.library.grid_width <= 1.0 || self.library.grid_width >= keymap::KEYCAP_MIN_WIDTH
    }

    fn run_command(&mut self, command: Command) -> bool {
        State::run_command(self, command)
    }
}

fn on_key(s: &mut State, event: &rune::KeyEvent, _tx: &mut Tx) -> bool {
    // An open overlay owns the keyboard: nothing behind a dialog runs on a keystroke.
    if s.panel.open || s.palette.open {
        return overlay_key(s, event);
    }
    let surface = match s.screen {
        Screen::Library => Surface::Library,
        Screen::Editor => Surface::Editor,
        Screen::Session => Surface::Session,
        Screen::Summary => Surface::Summary,
        Screen::Settings => Surface::Settings,
    };
    match typr::keymap::command_for(surface, event) {
        Some(command) => s.run_command(command),
        None => false,
    }
}

/// The keys an open overlay answers itself. Every other key is left to the focused field, so
/// typing a filter never fires the command behind the dialog.
fn overlay_key(s: &mut State, event: &rune::KeyEvent) -> bool {
    if matches!(event.key, rune::Key::Escape) {
        if s.palette.open {
            s.palette.open = false;
        } else {
            s.panel.open = false;
        }
        return true;
    }
    if !s.palette.open {
        return false;
    }
    let surface = s.surface();
    match &event.key {
        rune::Key::Up => {
            keymap::palette::move_up(&mut s.palette);
            let rows = keymap::palette::visible(surface, &s.palette.filter.text).len();
            s.palette.index = s.palette.index.min(rows.saturating_sub(1));
            true
        }
        rune::Key::Down => {
            keymap::palette::move_down(&mut s.palette, surface);
            true
        }
        rune::Key::Enter => keymap::palette::run(s),
        _ => false,
    }
}

/// A folder holding the picture the diagram scenes draw, so a snap shows a diagram rather than a
/// missing image. The folder outlives the scene because the process exits with it.
fn scene_attachments() -> typr::attachments::Attachments {
    let root = std::env::temp_dir()
        .join(format!("typr-scene-{}", std::process::id()))
        .join("attachments");
    std::fs::create_dir_all(&root).expect("a folder for the scene's picture");
    let picture = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/plate.png"))
        .expect("the fixture picture");
    std::fs::write(root.join("plate.png"), picture).expect("the scene's picture");
    // The scene has no window to wake, so its pictures decode on their own.
    typr::attachments::Attachments::new(root, std::sync::Arc::new(|| {}))
}

/// One prompt as a session entry, or `None` when the card cannot ask it.
fn entry_for(card: &CardSummary, key: &str, overdue: i64) -> Option<Entry> {
    // A diagram card is asked one group at a time: the key names the group, and the face carries
    // only that group's rectangles, so the covers cannot spill onto an answer the next prompt will
    // ask about.
    if let Some(group) = key.strip_prefix("mask:") {
        let group: u32 = group.parse().ok()?;
        let masks: Vec<_> = card
            .masks
            .iter()
            .filter(|mask| mask.group == group)
            .map(|mask| mask.rect)
            .collect();
        if masks.is_empty() {
            return None;
        }
        let image = card.image.clone()?;
        return Some(Entry::mask_group(
            card.id,
            key,
            image,
            card.caption.clone(),
            masks,
            overdue,
        ));
    }

    let shape = hidden::shape(&card.markdown);
    // A prompt the card's shape does not claim cannot be asked: masking it would hide nothing, so
    // the question would show the answer. A one-sided card is a note, which is the same rule with
    // an empty key list. The claim set is the shape's, so this and the store's writing of prompts
    // cannot drift apart.
    let claimed = shape.claimed_keys();
    if !claimed.iter().any(|claimed| claimed == key) {
        return None;
    }
    let reverse = key == "rev";
    Some(match &shape {
        hidden::Shape::Sides(sides) if reverse => Entry::reverse(card.id, sides.clone(), overdue),
        hidden::Shape::Sides(sides) => Entry::sides(card.id, sides.clone(), overdue),
        hidden::Shape::Hidden { text, .. } => Entry::masked(
            card.id,
            key,
            hidden::masked(text, &shape, key),
            hidden::shown(text, &shape),
            overdue,
        ),
    })
}

/// The cards a cram run walks, one question apiece.
///
/// A session answers a card once: answering puts the card in `answered` and the walk skips its
/// other entries, so a second question for the same card could never be shown. The filter selects
/// cards, so one entry each is the whole of it; forgetting returns that one entry to the end.
///
/// A card with no key is dropped rather than queued with a question it cannot ask. The store's
/// selection already leaves such cards out (`Db::cards_for_view` keeps only cards with a studyable
/// prompt), so this is the same rule stated again for any other source of cards.
fn cram_entries(cards: &[CardSummary]) -> Vec<Entry> {
    cards
        .iter()
        .filter_map(|card| {
            let key = card.prompt_keys().into_iter().next()?;
            entry_for(card, &key, 0)
        })
        .collect()
}

/// What the summary proposes as a name for a cram run's filter.
///
/// The filter is the part worth naming, so the name says what was selected rather than when it was
/// selected: a tag, the search that was typed, or the drill itself.
fn drill_name(spec: &ViewSpec) -> String {
    if let Some(tag) = spec.tags.first() {
        return format!("{tag} drill");
    }
    if !spec.query.trim().is_empty() {
        return format!("{} drill", spec.query.trim());
    }
    "Cram drill".to_string()
}

fn main() {
    // The study day needs the machine's zone, and `time` only answers on the main thread, so read
    // it before any worker starts.
    let _ = typr::day_offset::local_offset_ms();
    let scenes: &'static [Scene] = &[
        ("library-5000", |state: &mut State| {
            state.library = Library::seeded(5000);
            state.seeded = true;
        }),
        ("editor", |state: &mut State| {
            state.screen = Screen::Editor;
            state.editor = Editor::new(
                None,
                "What is **mitosis**?\n---\nCell division.".to_string(),
            );
        }),
        ("shortcut-panel", |state: &mut State| {
            state.library = Library::seeded(3);
            state.panel.open = true;
            state.seeded = true;
        }),
        ("command-palette", |state: &mut State| {
            state.library = Library::seeded(3);
            keymap::palette::open(&mut state.palette);
            state.seeded = true;
        }),
        ("library-empty", |state: &mut State| {
            state.screen = Screen::Library;
        }),
        ("library-grid", |state: &mut State| {
            state.library = Library::seeded(9);
            state.seeded = true;
        }),
        ("library-grid-wide", |state: &mut State| {
            state.library = Library::seeded_tiles();
            state.seeded = true;
        }),
        ("library-grid-empty", |state: &mut State| {
            state.library = Library::seeded(0);
            state.seeded = true;
        }),
        ("library-nested", |state: &mut State| {
            state.library = Library::seeded_nested();
            state.seeded = true;
        }),
        ("library-views", |state: &mut State| {
            state.library = Library::seeded_nested();
            state.library.seed_views(vec![
                SavedView {
                    id: 1,
                    name: "Exam week".to_string(),
                    spec: ViewSpec {
                        pinned_only: true,
                        sort: Sort::FirstSide,
                        dir: SortDir::Ascending,
                        ..ViewSpec::default()
                    },
                },
                SavedView {
                    id: 2,
                    name: "Verbs".to_string(),
                    spec: ViewSpec {
                        query: "verb".to_string(),
                        sort: Sort::Reviews,
                        ..ViewSpec::default()
                    },
                },
                SavedView {
                    id: 3,
                    name: "Archived".to_string(),
                    spec: ViewSpec {
                        mode: CardMode::Archived,
                        sort: Sort::Created,
                        ..ViewSpec::default()
                    },
                },
            ]);
            state.library.pin(2);
            state.library.select_view(1);
            state.seeded = true;
        }),
        ("library-archived", |state: &mut State| {
            state.library = Library::seeded_mixed();
            state.library.set_mode(CardMode::Archived);
            state.seeded = true;
        }),
        ("library-trash", |state: &mut State| {
            state.library = Library::seeded_mixed();
            state.library.set_mode(CardMode::Trash);
            state.seeded = true;
        }),
        ("history", |state: &mut State| {
            state.screen = Screen::Editor;
            state.editor = Editor::new(
                Some(1),
                "What is **mitosis**?\n---\nCell division.".to_string(),
            );
            state.editor.showing_history = true;
            let row = |phase, grade, at, due_at| typr_core::store::ReviewRow {
                card: 1,
                key: "fwd".to_string(),
                phase,
                at,
                grade,
                interval_ms: 600_000,
                due_at,
                duration_ms: None,
            };
            state.editor.history.set(
                1,
                vec![
                    row(typr_core::store::Phase::Learn, Grade::Forgot, 1_000, None),
                    row(
                        typr_core::store::Phase::Review,
                        Grade::Remembered,
                        2_000,
                        Some(86_400_000),
                    ),
                    row(
                        typr_core::store::Phase::Review,
                        Grade::Forgot,
                        3_000,
                        Some(3_600_000),
                    ),
                ],
                2,
            );
        }),
        ("session-new", |state: &mut State| {
            let entries = vec![Entry::sides(
                4,
                ["What is **osmosis**?", "Water across a membrane."],
                0,
            )];
            state.session = Session::new(Queue::New, entries);
            state.screen = Screen::Session;
        }),
        ("session", |state: &mut State| {
            let entries = vec![
                Entry::sides(1, ["What is **mitosis**?", "Cell division."], 1),
                Entry::sides(2, ["What is **osmosis**?", "Water across a membrane."], 1),
            ];
            state.session = Session::new(Queue::Due, entries);
            state.session.reveal();
            state.screen = Screen::Session;
        }),
        ("session-reverse", |state: &mut State| {
            // The same cards asked from their answers: the side that was the answer is the question.
            let entries = vec![
                Entry::reverse(1, ["What is **mitosis**?", "Cell division."], 1),
                Entry::reverse(2, ["What is **osmosis**?", "Water across a membrane."], 1),
            ];
            state.session = Session::new(Queue::Due, entries);
            state.session.reveal();
            state.screen = Screen::Session;
        }),
        ("summary", |state: &mut State| {
            state.session = Session::empty();
            state.due_now = 7;
            state.screen = Screen::Summary;
        }),
        ("failure", |state: &mut State| {
            state.library = Library::seeded(3);
            state.seeded = true;
            state.failure = Some("The library stopped: a command panicked".to_string());
        }),
        ("review-cloze", |state: &mut State| {
            state.screen = Screen::Session;
            state.session = Session::new(
                Queue::New,
                vec![Entry::masked(
                    1,
                    "cloze:0",
                    "The [ … ] is the gland that makes insulin.",
                    "The pancreas is the gland that makes insulin.",
                    0,
                )],
            );
        }),
        ("review-diagram", |state: &mut State| {
            state.editor.attachments = scene_attachments();
            // Two groups on one picture: the first is masked, so the review covers both of its
            // rectangles and leaves the other group's in sight.
            state.screen = Screen::Session;
            state.session = Session::new(
                Queue::New,
                vec![Entry::mask_group(
                    1,
                    "mask:1",
                    "plate.png",
                    Some("The plate: name the shaded regions".to_string()),
                    vec![
                        MaskRect::new(0.08, 0.12, 0.3, 0.28),
                        MaskRect::new(0.62, 0.12, 0.3, 0.28),
                    ],
                    0,
                )],
            );
        }),
        ("editor-diagram", |state: &mut State| {
            state.screen = Screen::Editor;
            state.editor = Editor::new(None, String::new());
            state.editor.attachments = scene_attachments();
            state.editor.set_kind(CardKind::Diagram);
            state.editor.image = Some("plate.png".to_string());
            state.editor.caption.replace_text("The plate".to_string());
            state.editor.masks = vec![
                MaskRow {
                    group: 1,
                    rect: MaskRect::new(0.08, 0.12, 0.3, 0.28),
                },
                MaskRow {
                    group: 2,
                    rect: MaskRect::new(0.6, 0.5, 0.3, 0.28),
                },
            ];
            state.editor.next_group = 3;
        }),
        ("editor-template", |state: &mut State| {
            state.screen = Screen::Editor;
            state.editor = Editor::new(Some(1), String::new());
            state.editor.load(
                &CardSummary {
                    id: 1,
                    deck: 2,
                    template: Some("<<Word>>\n---\n<<Meaning>>".to_string()),
                    template_from: Some(2),
                    fields: vec![
                        ("Word".to_string(), "gato".to_string()),
                        ("Meaning".to_string(), "cat".to_string()),
                    ],
                    ..CardSummary::default()
                },
                Some("Spanish"),
            );
        }),
        ("editor-leave", |state: &mut State| {
            state.screen = Screen::Editor;
            state.editor = Editor::new(
                None,
                "What is **mitosis**?\n---\nCell division.".to_string(),
            );
            state.editor.buffer.insert(" Edited.");
            state.editor.request_leave();
        }),
        ("session-cram", |state: &mut State| {
            // A run over a filter: the cards the library had selected, asked once each with no
            // due date behind them, so the screen has to say where the answers go.
            let entries = vec![
                Entry::sides(1, ["What is **mitosis**?", "Cell division."], 0),
                Entry::sides(2, ["What is **osmosis**?", "Water across a membrane."], 0),
            ];
            state.session = Session::cram(entries);
            state.screen = Screen::Session;
        }),
        ("editor-backlinks", |state: &mut State| {
            let text = "The citric acid cycle\n---\nRuns in the mitochondrion.".to_string();
            state.screen = Screen::Editor;
            state.editor = Editor::new(Some(1), text.clone());
            // Three referrers, so the panel shows the whole list rather than one lucky row.
            state.editor.load(
                &CardSummary {
                    id: 1,
                    deck: 2,
                    name: "The citric acid cycle".to_string(),
                    markdown: text,
                    backlinks: vec![
                        CardRef {
                            id: 2,
                            name: "Krebs cycle".to_string(),
                        },
                        CardRef {
                            id: 3,
                            name: "Notes".to_string(),
                        },
                        CardRef {
                            id: 4,
                            name: "Glycolysis".to_string(),
                        },
                    ],
                    ..CardSummary::default()
                },
                Some("Biology"),
            );
        }),
        ("card-embed", |state: &mut State| {
            // The preview of a card that embeds another: the named card's body is drawn inside a
            // bordered inset, captioned with the name, with the card's own text around it.
            state.screen = Screen::Editor;
            state.editor = Editor::new(
                Some(1),
                "The stages of mitosis\n\n![[Mitosis]]\n---\nCell division.".to_string(),
            );
            state.library.set_snapshot(std::sync::Arc::new(Snapshot {
                decks: vec![Deck {
                    id: 1,
                    parent: None,
                    name: "Biology".to_string(),
                    archived: false,
                    reverse: None,
                    template: None,
                }],
                cards: vec![
                    CardSummary {
                        id: 1,
                        deck: 1,
                        name: "The stages of mitosis".to_string(),
                        markdown: "The stages of mitosis\n\n![[Mitosis]]\n---\nCell division."
                            .to_string(),
                        ..CardSummary::default()
                    },
                    CardSummary {
                        id: 2,
                        deck: 1,
                        name: "Mitosis".to_string(),
                        markdown: "Mitosis is how one cell becomes two identical cells."
                            .to_string(),
                        ..CardSummary::default()
                    },
                ],
                ..Snapshot::default()
            }));
            // What a store with this library would have resolved the embed's name to, so the
            // preview does not also report it as a reference that names no card.
            state.editor.references = vec![CardRef {
                id: 2,
                name: "Mitosis".to_string(),
            }];
        }),
    ];
    // Without this Rune ends a window close with `_exit`, so both the worker's flush and the quit
    // record below would be skipped: data would survive only because each command commits as it is
    // answered, and nothing would ever write down how a run ended.
    rune::orderly_exit();
    rune::run(AppSpec {
        scenes,
        setup: Some(|state: &mut State, waker: Waker| state.setup(waker)),
        poll: Some(poll),
        on_key: Some(on_key),
        ..AppSpec::new("typr", (960.0, 640.0), State::default, view)
    });

    // The worker's end is only knowable after its handle went with the state, inside `rune::run`.
    finish(WORKER.get());
}

/// The card on screen is the one an embed resolves from, so the deck it sits in is preferred the
/// way a reference's is.
impl State {
    fn embed_body(&self, name: &str) -> Option<String> {
        let from = match self.screen {
            Screen::Editor => self.editor.card,
            Screen::Session => self.session.current().map(|entry| entry.card),
            _ => None,
        };
        typr::embed::lookup(self.library.snapshot(), from, name)
    }
}

#[cfg(test)]
mod tests;
