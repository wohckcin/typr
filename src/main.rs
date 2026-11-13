//! `typr` — a local-first flashcard app on Rune.

use rune::tok::{self, type_};
use rune::{AppSpec, Cx, El, Tx, Waker, col, row, text};
use typr::attachments::Attachments;
use typr::clock::Clock;
use typr::editor::{self, Editor};
use typr::history::{self, History};
use typr::library::{self, Library};
use typr::screen::{Queue, Screen};
use typr::session::{Entry, Session};
use typr::store::{self, Grade, Handle, Id, Outcome, Store};
use typr::study::{self, Host};
use typr::{day, sides};

/// A named starting state for a headless `--scene` run.
type Scene = (&'static str, fn(&mut State));

struct State {
    /// Drives `screen::step`; the library screen has no events of its own yet, so nothing reads
    /// it until the editor and session screens arrive.
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
    /// The history request in flight, with the card it was made for.
    history_request: Option<(u64, Id)>,
    /// The create-card request in flight, whose answer carries the new id.
    card_request: Option<u64>,
    /// The last thing that went wrong, shown until something else replaces it.
    failure: Option<String>,
    /// A scene brought its own library, so opening the real one must not replace it.
    seeded: bool,
    /// A new card waiting for the deck that has to be created for it.
    pending_new_card: Option<String>,
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
            history_request: None,
            card_request: None,
            failure: None,
            seeded: false,
            pending_new_card: None,
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
}

impl library::Host for State {
    fn library(&self) -> &Library {
        &self.library
    }

    fn library_mut(&mut self) -> &mut Library {
        &mut self.library
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

    fn submit_grade(&mut self, card: Id, grade: Grade) -> u64 {
        match &self.store {
            Some(store) => store.grade(card, "fwd", grade, None),
            None => 0,
        }
    }

    fn due_now(&self) -> usize {
        self.due_now
    }

    fn leave_session(&mut self) {
        self.screen = Screen::Library;
        self.session = Session::empty();
    }
}

impl State {
    /// Open the library, unless this run is a headless snapshot or script: those draw fixtures
    /// and must never touch the real one.
    fn setup(&mut self, waker: Waker) {
        if rune::tooling::is_headless() {
            return;
        }
        let draw = waker.clone();
        let opened = store::attachments_root_from_environment()
            .and_then(|root| Handle::open_system(waker).map(|handle| (root, handle)));
        match opened {
            Ok((root, handle)) => {
                if !self.seeded {
                    self.library.set_snapshot(handle.snapshot());
                }
                self.editor.attachments = Attachments::new(root, draw);
                self.store = Some(handle);
            }
            Err(error) => {
                self.failure = Some(format!("The library could not be opened: {error}"));
            }
        }
    }

    /// Write the editor's buffer to the library, and mark it saved once the command is sent.
    fn persist(&mut self) -> bool {
        if !self.editor.saveable() {
            self.editor.refused = true;
            return false;
        }
        let text = self.editor.buffer.text.clone();
        match (self.editor.card, self.store.as_ref()) {
            (Some(card), Some(store)) => {
                store.save_card(card, &text);
            }
            (None, Some(store)) => match self.deck_for_new_card() {
                Some(deck) => {
                    self.card_request = Some(store.create_card(deck, &text));
                }
                None => {
                    // A library with no deck yet: the first card brings one with it.
                    self.pending_new_card = Some(text.clone());
                    store.create_deck(None, "Library");
                }
            },
            (_, None) => {
                self.failure = Some("The library is not open, so nothing was saved.".to_string());
                return false;
            }
        }
        self.editor.save()
    }

    /// Where a card that has never been saved goes.
    fn deck_for_new_card(&self) -> Option<Id> {
        self.library.selected_deck.or_else(|| {
            self.library
                .snapshot
                .decks
                .iter()
                .find(|deck| !deck.archived && deck.parent.is_none())
                .map(|deck| deck.id)
        })
    }

    /// Ask the store for the queue the user chose. The session opens when the answer arrives.
    fn start_session(&mut self, queue: Queue) {
        let Some(store) = &self.store else { return };
        let now = typr::clock::SystemClock.now_ms();
        let offset = day::local_offset_ms();
        let seed = now as u64;
        self.due_request = Some(store.due_prompts(now, offset, seed));
        if queue == Queue::New {
            self.new_request = Some(store.new_prompts(now, offset, seed));
        }
        self.wanted = Some(queue);
    }

    /// Open the session over the prompts the store answered with.
    fn open_session(&mut self, queue: Queue, prompts: Vec<(Id, String)>) {
        let snapshot = self.store.as_ref().map(Store::snapshot).unwrap_or_default();
        let entries: Vec<Entry> = prompts
            .into_iter()
            .filter(|(_, key)| key == "fwd")
            .filter_map(|(id, _)| {
                let card = snapshot.cards.iter().find(|card| card.id == id)?;
                Some(Entry {
                    card: card.id,
                    sides: sides::split(&card.markdown)
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                })
            })
            .collect();
        if entries.is_empty() {
            return;
        }
        self.session = Session::new(queue, entries);
        self.screen = Screen::Session { queue };
    }

    /// Answer what the store has said. A grade the store refused is dropped, so the card stays.
    fn drain(&mut self) -> bool {
        let Some(store) = &self.store else {
            return false;
        };
        let events = store.poll();
        let now = typr::clock::SystemClock.now_ms();
        let mut changed = false;
        for event in events {
            changed = true;
            match event.result {
                Ok(Outcome::Graded { .. }) => {
                    let acknowledged = self.session.acknowledge(event.id, now);
                    // Leaving the session first must not be undone by a grade that lands after.
                    if acknowledged
                        && matches!(self.screen, Screen::Session { .. })
                        && self.session.finished()
                    {
                        self.screen = Screen::Summary;
                    }
                }
                Ok(Outcome::Due(prompts)) if self.due_request == Some(event.id) => {
                    self.due_request = None;
                    self.due_now = prompts.iter().filter(|prompt| prompt.key == "fwd").count();
                    if self.wanted == Some(Queue::Due) {
                        self.wanted = None;
                        let prompts = prompts.into_iter().map(|p| (p.card, p.key)).collect();
                        self.open_session(Queue::Due, prompts);
                    }
                }
                Ok(Outcome::History(rows)) => {
                    if let Some((_, card)) = self.history_request.filter(|(id, _)| *id == event.id)
                    {
                        self.history_request = None;
                        self.editor.history.set(card, rows);
                    }
                }
                Ok(Outcome::New(prompts)) if self.new_request == Some(event.id) => {
                    self.new_request = None;
                    if self.wanted.take() == Some(Queue::New) {
                        let prompts = prompts.into_iter().map(|p| (p.card, p.key)).collect();
                        self.open_session(Queue::New, prompts);
                    }
                }
                Ok(Outcome::Card(id)) if self.card_request == Some(event.id) => {
                    self.card_request = None;
                    self.editor.card = Some(id);
                }
                Ok(Outcome::Deck(id)) => {
                    if let Some(text) = self.pending_new_card.take()
                        && let Some(store) = &self.store
                    {
                        self.card_request = Some(store.create_card(id, &text));
                    }
                }
                Ok(_) => {}
                Err(error) => {
                    self.session.refuse(event.id);
                    self.failure = Some(error.to_string());
                }
            }
        }
        if changed && let Some(store) = &self.store {
            self.library.set_snapshot(store.snapshot());
        }
        changed
    }
}

fn view(s: &State, _: &mut Cx<State>) -> El {
    let screen = match s.screen {
        Screen::Library => library::view::<State>(s),
        Screen::Editor { .. } => editor::view::<State>(s),
        Screen::Session { .. } => study::view::<State>(s),
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
    let (landed, wake) = advance_session(s, typr::clock::SystemClock.now_ms());
    if let Some(ms) = wake {
        tx.wake_after(std::time::Duration::from_millis(ms as u64));
    }
    s.drain() || landed > 0
}

fn on_key(s: &mut State, event: &rune::KeyEvent, _tx: &mut Tx) -> bool {
    match s.screen {
        Screen::Session { .. } => study::key(s, event),
        Screen::Summary => {
            if event.key == rune::Key::Escape {
                s.leave_session();
                true
            } else {
                false
            }
        }
        // Two ways into a session from the library, so the keyboard can start one.
        Screen::Editor { .. } => match &event.key {
            // `h` shows this card's review history beside the preview, or hides it again.
            rune::Key::Text(t) if t == "h" => {
                s.editor.showing_history = !s.editor.showing_history;
                if s.editor.showing_history
                    && let (Some(card), Some(store)) = (s.editor.card, &s.store)
                {
                    s.history_request = Some((store.history(card), card));
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
    let _ = typr::day::local_offset_ms();
    let scenes: &'static [Scene] = &[
        ("library-5000", |state: &mut State| {
            state.library = Library::seeded(5000);
            state.seeded = true;
        }),
        ("editor", |state: &mut State| {
            state.screen = Screen::Editor { card: None };
            state.editor = Editor::new(
                None,
                "What is **mitosis**?\n---\nCell division.".to_string(),
            );
        }),
        ("library-empty", |state: &mut State| {
            state.screen = Screen::Library;
        }),
        ("history", |state: &mut State| {
            state.screen = Screen::Editor {
                card: Some(typr::screen::CardId(1)),
            };
            state.editor = Editor::new(
                Some(1),
                "What is **mitosis**?\n---\nCell division.".to_string(),
            );
            state.editor.showing_history = true;
            let row = |phase, grade, at, due_at| typr::store::ReviewRow {
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
                    row(typr::store::Phase::Learn, Grade::Forgot, 1_000, None),
                    row(
                        typr::store::Phase::Review,
                        Grade::Remembered,
                        2_000,
                        Some(86_400_000),
                    ),
                    row(
                        typr::store::Phase::Review,
                        Grade::Forgot,
                        3_000,
                        Some(3_600_000),
                    ),
                ],
            );
        }),
        ("session-new", |state: &mut State| {
            let entries = vec![Entry {
                card: 4,
                sides: vec![
                    "What is **osmosis**?".to_string(),
                    "Water across a membrane.".to_string(),
                ],
            }];
            state.session = Session::new(Queue::New, entries);
            state.screen = Screen::Session { queue: Queue::New };
        }),
        ("session", |state: &mut State| {
            let entries = vec![
                Entry {
                    card: 1,
                    sides: vec![
                        "What is **mitosis**?".to_string(),
                        "Cell division.".to_string(),
                    ],
                },
                Entry {
                    card: 2,
                    sides: vec![
                        "What is **osmosis**?".to_string(),
                        "Water across a membrane.".to_string(),
                    ],
                },
            ];
            state.session = Session::new(Queue::Due, entries);
            state.session.reveal();
            state.screen = Screen::Session { queue: Queue::Due };
        }),
        ("summary", |state: &mut State| {
            state.session = Session::empty();
            state.due_now = 7;
            state.screen = Screen::Summary;
        }),
        ("editor-leave", |state: &mut State| {
            state.screen = Screen::Editor { card: None };
            state.editor = Editor::new(
                None,
                "What is **mitosis**?\n---\nCell division.".to_string(),
            );
            state.editor.buffer.insert(" Edited.");
            state.editor.request_leave();
        }),
    ];
    rune::run(AppSpec {
        scenes,
        setup: Some(|state: &mut State, waker: Waker| state.setup(waker)),
        poll: Some(poll),
        on_key: Some(on_key),
        ..AppSpec::new("typr", (960.0, 640.0), State::default, view)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use typr::clock::LEARNING_STEP_MS;

    /// One due card, revealed to its last side, forgotten, and acknowledged.
    fn forgotten_due_card() -> State {
        let mut session = Session::new(
            Queue::Due,
            vec![Entry {
                card: 7,
                sides: vec!["front".to_string(), "back".to_string()],
            }],
        );
        while session.next_side().is_some() {
            session.reveal();
        }
        assert!(session.grade(Grade::Forgot).is_some(), "graded");
        session.sent(42);
        assert!(session.acknowledge(42, 1_000), "acknowledged");
        State {
            session,
            ..State::default()
        }
    }

    /// A library on disk holding one deck and one two-sided card, and a handle onto it.
    fn library_with_one_card(dir: &tempfile::TempDir) -> Handle {
        let path = dir.path().join("library.sqlite3");
        {
            let db = typr::store::Db::open(&path).expect("open");
            let deck = match db
                .apply(
                    &typr::store::Command::CreateDeck {
                        parent: None,
                        name: "Biology".to_string(),
                    },
                    0,
                )
                .expect("deck")
            {
                Outcome::Deck(id) => id,
                other => panic!("expected a deck, got {other:?}"),
            };
            db.apply(
                &typr::store::Command::CreateCard {
                    deck,
                    markdown: "front\n---\nback".to_string(),
                },
                0,
            )
            .expect("card");
        }
        Handle::spawn(
            typr::store::Db::open(&path).expect("open"),
            Box::new(typr::clock::SystemClock),
            std::sync::Arc::new(|| {}),
        )
        .expect("spawn")
    }

    /// A session showing the library's only card, revealed to its last side.
    fn session_on(card: Id) -> Session {
        let mut session = Session::new(
            Queue::Due,
            vec![Entry {
                card,
                sides: vec!["front".to_string(), "back".to_string()],
            }],
        );
        while session.next_side().is_some() {
            session.reveal();
        }
        session
    }

    #[test]
    fn a_grade_that_lands_after_leaving_does_not_open_a_summary() {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = library_with_one_card(&dir);
        let mut state = State {
            store: Some(store),
            session: session_on(1),
            screen: Screen::Session { queue: Queue::Due },
            ..State::default()
        };
        let grade = state.session.grade(Grade::Remembered).expect("graded").1;
        let id = state.submit_grade(1, grade);
        state.session.sent(id);

        state.leave_session();
        for _ in 0..100 {
            state.drain();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        assert!(
            matches!(state.screen, Screen::Library),
            "the answer arrived after the user left, and the summary is not shown"
        );
        assert!(state.failure.is_none(), "the grade itself was accepted");
    }

    #[test]
    fn a_refused_grade_says_why_and_leaves_the_card_up() {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = library_with_one_card(&dir);
        let mut state = State {
            store: Some(store),
            session: session_on(9),
            screen: Screen::Session { queue: Queue::Due },
            ..State::default()
        };
        let grade = state.session.grade(Grade::Remembered).expect("graded").1;
        let id = state.submit_grade(9, grade);
        state.session.sent(id);

        for _ in 0..100 {
            state.drain();
            if state.failure.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        let failure = state.failure.as_deref().expect("the refusal is reported");
        assert!(failure.contains("has no prompt"), "got {failure}");
        assert_eq!(
            state.session.current().map(|entry| entry.card),
            Some(9),
            "the card stays up instead of wedging the session"
        );
    }

    #[test]
    fn a_failure_is_drawn_at_the_bottom_of_the_screen() {
        let state = State {
            failure: Some("The library could not be opened: no such file".to_string()),
            ..State::default()
        };
        let mut runtime =
            rune::Runtime::new(state, view, (900.0, 600.0), rune::Env::new(true, 1.0));
        runtime.frame(0.0);
        assert!(runtime.find("failure").is_some(), "the line is on screen");
        let drawn = runtime
            .elements()
            .into_iter()
            .any(|element| element.key.as_deref() == Some("failure"));
        assert!(drawn, "the failure row is in the tree, not just a label");
    }

    #[test]
    fn a_save_with_no_library_refuses_and_explains() {
        let mut editor = Editor::new(None, "front\n---\nback".to_string());
        editor.buffer.insert(" edited");
        let mut state = State {
            editor,
            ..State::default()
        };
        assert!(!state.persist(), "nothing was stored");
        let failure = state.failure.as_deref().expect("the refusal is reported");
        assert!(failure.contains("not open"), "got {failure}");
        assert!(state.editor.dirty(), "the buffer is still unsaved");
    }

    #[test]
    fn saving_a_new_card_stores_it_and_adopts_its_id() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("library.sqlite3");
        let waker: Waker = std::sync::Arc::new(|| {});
        let handle = Handle::spawn(
            typr::store::Db::open(&path).expect("open"),
            Box::new(typr::clock::SystemClock),
            waker,
        )
        .expect("spawn");

        let mut state = State {
            store: Some(handle),
            ..State::default()
        };
        state.editor = Editor::new(None, "front\n---\nback".to_string());
        assert!(state.persist(), "the card is stored");

        // The store answers on its own thread; drain until the card's id arrives.
        for _ in 0..200 {
            state.drain();
            if state.editor.card.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let card = state.editor.card.expect("the new card adopted its id");
        assert!(!state.editor.dirty(), "the buffer counts as saved");

        drop(state.store.take());
        let db = typr::store::Db::open(&path).expect("reopen");
        let cards = db.snapshot().expect("snapshot").cards;
        assert_eq!(cards.len(), 1, "one card on disk");
        assert_eq!(cards[0].id, card);
        assert_eq!(cards[0].markdown, "front\n---\nback");
    }

    #[test]
    fn a_forgotten_due_card_returns_when_its_step_lands() {
        let mut state = forgotten_due_card();
        assert!(state.session.current().is_none(), "the card left the queue");

        let (landed, wake) = advance_session(&mut state, 1_000);
        assert_eq!(landed, 0, "not yet");
        assert_eq!(wake, Some(LEARNING_STEP_MS), "the frame loop is told when");

        let (landed, wake) = advance_session(&mut state, 1_000 + LEARNING_STEP_MS);
        assert_eq!(landed, 1, "the card is back");
        assert_eq!(wake, None, "nothing left to wait for");
        assert_eq!(state.session.current().map(|entry| entry.card), Some(7));
        assert!(!state.session.finished(), "the session is not over");
    }
}
