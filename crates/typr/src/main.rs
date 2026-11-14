//! `typr` — a local-first flashcard app on Rune.

use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use rune::tok::{self, type_};
use rune::{AppSpec, Cx, El, Tx, Waker, col, row, text};
use typr::attachments::Attachments;
use typr::day_offset;
use typr::editor::{self, Editor};
use typr::history::{self, History};
use typr::library::{self, Library};
use typr::logging;
use typr::paths;
use typr::screen::Screen;
use typr::study::{self, Host};
use typr_core::clock::{Clock, SystemClock};
use typr_core::screen::Queue;
use typr_core::session::{Entry, Session};
use typr_core::store::{CardMode, Grade, Handle, Id, Outcome, Store, StoreError, WorkerStatus};
use typr_core::{day, sides};

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
    deck_request: Option<u64>,
    /// Every command that went out and has not been answered: what it was, and when it left.
    sent: HashMap<u64, (&'static str, Instant)>,
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
            seed: 1,
            pending_new_card: None,
            deck_request: None,
            sent: HashMap::new(),
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
}

/// How many whole study days a prompt is past its due date, which is what decides whether a
/// shuffle may move it. A prompt that is not due yet counts as zero.
fn overdue_days(now_ms: i64, due_at_ms: i64, offset_ms: i64) -> i64 {
    (day::study_day(now_ms, offset_ms) - day::study_day(due_at_ms, offset_ms)).max(0)
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
        self.editor = Editor::new(Some(card), summary.markdown.clone());
        self.editor.load_tags(&summary.tags);
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
    fn session(&self) -> &Session {
        &self.session
    }

    fn session_mut(&mut self) -> &mut Session {
        &mut self.session
    }

    fn submit_grade(&mut self, card: Id, grade: Grade) -> Result<u64, StoreError> {
        let Some(store) = &self.store else {
            self.fail(&StoreError::Closed);
            return Err(StoreError::Closed);
        };
        let sent = store.grade(card, "fwd", grade, None);
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
            .filter(|(_, key, _)| key == "fwd")
            .filter_map(|(id, _, due_at)| {
                let card = snapshot.cards.iter().find(|card| card.id == id)?;
                let overdue = due_at.map_or(0, |due_at| overdue_days(now, due_at, offset));
                Some(Entry {
                    card: card.id,
                    sides: sides::split(&card.markdown)
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    overdue,
                })
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
                Ok(Outcome::Card(id)) if self.card_request == Some(event.id) => {
                    self.card_request = None;
                    self.editor.card = Some(id);
                    // The card exists now, so the buffer is saved if it still holds what was sent.
                    if self.pending_new_card.as_deref() == Some(self.editor.buffer.text.as_str()) {
                        self.editor.save();
                    }
                    self.pending_new_card = None;
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
                    if let (Some(text), Some(store)) = (self.pending_new_card.take(), &self.store) {
                        let sent = store.create_card(id, &text);
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
                Ok(_) => {}
                Err(error) => {
                    self.session.refuse(event.id);
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

fn view(s: &State, _: &mut Cx<State>) -> El {
    let screen = match s.screen {
        Screen::Library => library::view::<State>(s),
        Screen::Editor => editor::view::<State>(s),
        Screen::Session => study::view::<State>(s),
        Screen::Summary => study::summary::<State>(s),
    };
    match &s.failure {
        None => screen,
        Some(failure) => col()
            .w_full()
            .h_full()
            .child(screen.grow(1.0))
            .child(status_bar(failure)),
    }
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

fn on_key(s: &mut State, event: &rune::KeyEvent, _tx: &mut Tx) -> bool {
    match s.screen {
        Screen::Session => study::key(s, event),
        Screen::Summary => {
            if event.key == rune::Key::Escape {
                s.leave_session();
                true
            } else {
                false
            }
        }
        // Two ways into a session from the library, so the keyboard can start one.
        Screen::Editor => match &event.key {
            // Escape leaves, asking first when the buffer has unsaved work.
            rune::Key::Escape => {
                editor::Host::leave_editor(s);
                true
            }
            // `h` shows this card's review history beside the preview, or hides it again.
            rune::Key::Text(t) if t == "h" => {
                s.editor.showing_history = !s.editor.showing_history;
                if s.editor.showing_history
                    && let (Some(card), Some(store)) = (s.editor.card, &s.store)
                {
                    match store.history(card) {
                        Ok(id) => s.history_request = Some((id, card)),
                        Err(error) => s.fail(&error),
                    }
                }
                true
            }
            _ if event.is_command('s') => {
                s.persist();
                true
            }
            _ => false,
        },
        Screen::Library if s.session.finished() => match &event.key {
            rune::Key::Text(t) if t == "s" => {
                s.start_session(Queue::Due);
                true
            }
            rune::Key::Text(t) if t == "n" => {
                s.start_session(Queue::New);
                true
            }
            _ => false,
        },
        _ => false,
    }
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
            let entries = vec![Entry {
                card: 4,
                sides: vec![
                    "What is **osmosis**?".to_string(),
                    "Water across a membrane.".to_string(),
                ],
                overdue: 0,
            }];
            state.session = Session::new(Queue::New, entries);
            state.screen = Screen::Session;
        }),
        ("session", |state: &mut State| {
            let entries = vec![
                Entry {
                    card: 1,
                    sides: vec![
                        "What is **mitosis**?".to_string(),
                        "Cell division.".to_string(),
                    ],
                    overdue: 1,
                },
                Entry {
                    card: 2,
                    sides: vec![
                        "What is **osmosis**?".to_string(),
                        "Water across a membrane.".to_string(),
                    ],
                    overdue: 1,
                },
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
        ("editor-leave", |state: &mut State| {
            state.screen = Screen::Editor;
            state.editor = Editor::new(
                None,
                "What is **mitosis**?\n---\nCell division.".to_string(),
            );
            state.editor.buffer.insert(" Edited.");
            state.editor.request_leave();
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

#[cfg(test)]
mod tests;
