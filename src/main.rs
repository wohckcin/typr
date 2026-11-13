//! `typr` — a local-first flashcard app on Rune.

use rune::machine::Seq;
use rune::{AppSpec, Cx, El, Tx, Waker, col, text};
use typr::attachments::Attachments;
use typr::editor::{self, Editor};
use typr::library::{self, Library};
use typr::screen::{Queue, Screen};
use typr::session::{Entry, Session};
use typr::clock::Clock;
use typr::store::{self, Grade, Handle, Id, Outcome, Store};
use typr::study::{self, Host};
use typr::{day, sides};

struct State {
    /// Drives `screen::step`; the library screen has no events of its own yet, so nothing reads
    /// it until the editor and session screens arrive.
    #[allow(dead_code)]
    seq: Seq,
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
}

impl Default for State {
    fn default() -> Self {
        Self {
            seq: Seq::default(),
            screen: Screen::Library,
            library: Library::default(),
            editor: Editor::default(),
            session: Session::empty(),
            store: None,
            due_now: 0,
            wanted: None,
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
}

impl editor::Host for State {
    fn editor(&self) -> &Editor {
        &self.editor
    }

    fn editor_mut(&mut self) -> &mut Editor {
        &mut self.editor
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
    /// Open the library, and draw whatever the editor was left holding.
    fn setup(&mut self, waker: Waker) {
        let draw = waker.clone();
        let opened = store::attachments_root_from_environment()
            .and_then(|root| Ok((root, Handle::open_system(waker)?)));
        match opened {
            Ok((root, handle)) => {
                self.library.set_snapshot(handle.snapshot());
                self.editor.attachments = Attachments::new(root, draw);
                self.store = Some(handle);
            }
            Err(error) => eprintln!("typr: the library could not be opened: {error}"),
        }
    }

    /// Ask the store for the queue the user chose. The session opens when the answer arrives.
    fn start_session(&mut self, queue: Queue) {
        let Some(store) = &self.store else { return };
        let now = typr::clock::SystemClock.now_ms();
        let offset = day::local_offset_ms();
        let seed = now as u64;
        match queue {
            Queue::Due => store.due_prompts(now, offset, seed),
            Queue::New => store.new_prompts(now, offset, seed),
        };
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
                    sides: sides::split(&card.markdown).into_iter().map(str::to_string).collect(),
                })
            })
            .collect();
        if entries.is_empty() {
            return;
        }
        self.due_now = snapshot
            .cards
            .iter()
            .filter(|_card| queue == Queue::Due)
            .count();
        self.session = Session::new(queue, entries);
        self.screen = Screen::Session { queue };
    }

    /// Answer what the store has said. A grade the store refused is dropped, so the card stays.
    fn drain(&mut self) -> bool {
        let Some(store) = &self.store else { return false };
        let events = store.poll();
        let now = typr::clock::SystemClock.now_ms();
        let mut changed = false;
        for event in events {
            changed = true;
            match event.result {
                Ok(Outcome::Graded { .. }) => {
                    self.session.acknowledge(event.id, now);
                    if self.session.finished() {
                        self.screen = Screen::Summary;
                    }
                }
                Ok(Outcome::Due(prompts)) => {
                    let queue = self.wanted.take().unwrap_or(Queue::Due);
                    let prompts = prompts.into_iter().map(|p| (p.card, p.key)).collect();
                    self.open_session(queue, prompts);
                }
                Ok(Outcome::New(prompts)) => {
                    let queue = self.wanted.take().unwrap_or(Queue::New);
                    let prompts = prompts.into_iter().map(|p| (p.card, p.key)).collect();
                    self.open_session(queue, prompts);
                }
                Ok(_) => {}
                Err(_) => {}
            }
        }
        if changed && let Some(store) = &self.store {
            self.library.set_snapshot(store.snapshot());
        }
        changed
    }
}

fn view(s: &State, _: &mut Cx<State>) -> El {
    match s.screen {
        Screen::Library => library::view::<State>(s),
        Screen::Editor { .. } => editor::view::<State>(s),
        Screen::Session { .. } => study::view::<State>(s),
        Screen::Summary => study::summary::<State>(s),
    }
}

fn poll(s: &mut State, _dt: f64, _tx: &mut Tx) -> bool {
    s.drain()
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
    let scenes: &'static [(&'static str, fn(&mut State))] = &[
        ("library-5000", |state: &mut State| state.library = Library::seeded(5000)),
        ("editor", |state: &mut State| {
            state.screen = Screen::Editor { card: None };
            state.editor = Editor::new(None, "What is **mitosis**?\n---\nCell division.".to_string());
        }),
        ("session", |state: &mut State| {
            let entries = vec![
                Entry { card: 1, sides: vec!["What is **mitosis**?".to_string(), "Cell division.".to_string()] },
                Entry { card: 2, sides: vec!["What is **osmosis**?".to_string(), "Water across a membrane.".to_string()] },
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
            state.editor = Editor::new(None, "What is **mitosis**?\n---\nCell division.".to_string());
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
