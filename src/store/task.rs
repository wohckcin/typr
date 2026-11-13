//! The store task: one worker thread owns the `Connection`, the UI only ever sends commands.
//!
//! Every command carries an id and comes back as exactly one [`Event`], so "the grade is written
//! before the next card is shown" is a property of the protocol rather than a hope. When a
//! command arrives, the worker applies it, re-publishes the snapshot, records the event and wakes
//! the frame loop. `poll` returns only the events that arrived since the last call, which is what
//! lets `AppSpec::poll` report "nothing happened" and cost an idle frame nothing.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use rune::Waker;

use crate::clock::{Clock, SystemClock};

use super::{Command, Db, Event, Grade, Id, Snapshot, Store, StoreError};

/// Everything the worker publishes for the UI thread.
struct Shared {
    snapshot: Mutex<Snapshot>,
    events: Mutex<Vec<Event>>,
    waker: Waker,
}

/// The UI's end of the store task.
pub struct Handle {
    tx: Mutex<Option<Sender<Message>>>,
    worker: Mutex<Option<JoinHandle<()>>>,
    shared: Arc<Shared>,
    next_id: AtomicU64,
}

struct Message {
    id: u64,
    command: Command,
}

impl Handle {
    /// Start the worker on an open database.
    pub fn spawn(db: Db, clock: Box<dyn Clock + Send + Sync>, waker: Waker) -> Result<Self, StoreError> {
        let snapshot = db.snapshot()?;
        let shared = Arc::new(Shared {
            snapshot: Mutex::new(snapshot),
            events: Mutex::new(Vec::new()),
            waker,
        });
        let (tx, rx) = mpsc::channel();
        let worker = {
            let shared = Arc::clone(&shared);
            std::thread::Builder::new()
                .name("typr-store".to_string())
                .spawn(move || work(db, rx, shared, clock))?
        };
        Ok(Self {
            tx: Mutex::new(Some(tx)),
            worker: Mutex::new(Some(worker)),
            shared,
            next_id: AtomicU64::new(0),
        })
    }

    /// Start the worker on the library in the process environment.
    pub fn open(clock: Box<dyn Clock + Send + Sync>, waker: Waker) -> Result<Self, StoreError> {
        Self::spawn(super::open_from_environment()?, clock, waker)
    }

    /// A handle over the system clock, for the app.
    pub fn open_system(waker: Waker) -> Result<Self, StoreError> {
        Self::open(Box::new(SystemClock), waker)
    }

    /// Send the last flush and wait for it: the one place the frame thread blocks, because a
    /// grade acknowledged but not yet written is a lost grade.
    pub fn close(&self) {
        drop(self.tx.lock().expect("store sender").take());
        let worker = self.worker.lock().expect("store worker").take();
        if let Some(worker) = worker {
            let _ = worker.join();
        }
    }

    fn submit(&self, command: Command) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed) + 1;
        let sender = self.tx.lock().expect("store sender");
        if let Some(tx) = sender.as_ref() {
            // A closed worker means a quit in progress; the command is already lost either way,
            // so an error here is not worth reporting.
            let _ = tx.send(Message { id, command });
        }
        id
    }
}

impl Store for Handle {
    fn snapshot(&self) -> Snapshot {
        self.shared.snapshot.lock().expect("snapshot").clone()
    }

    fn poll(&self) -> Vec<Event> {
        std::mem::take(&mut *self.shared.events.lock().expect("events"))
    }

    fn create_deck(&self, parent: Option<Id>, name: &str) -> u64 {
        self.submit(Command::CreateDeck { parent, name: name.to_string() })
    }

    fn rename_deck(&self, id: Id, name: &str) -> u64 {
        self.submit(Command::RenameDeck { id, name: name.to_string() })
    }

    fn archive_deck(&self, id: Id, archived: bool) -> u64 {
        self.submit(Command::ArchiveDeck { id, archived })
    }

    fn move_deck(&self, id: Id, parent: Option<Id>) -> u64 {
        self.submit(Command::MoveDeck { id, parent })
    }

    fn create_card(&self, deck: Id, markdown: &str) -> u64 {
        self.submit(Command::CreateCard { deck, markdown: markdown.to_string() })
    }

    fn save_card(&self, id: Id, markdown: &str) -> u64 {
        self.submit(Command::SaveCard { id, markdown: markdown.to_string() })
    }

    fn move_card(&self, id: Id, deck: Id) -> u64 {
        self.submit(Command::MoveCard { id, deck })
    }

    fn trash_card(&self, id: Id) -> u64 {
        self.submit(Command::TrashCard { id })
    }

    fn restore_card(&self, id: Id) -> u64 {
        self.submit(Command::RestoreCard { id })
    }

    fn set_draft(&self, card: Id, text: Option<&str>) -> u64 {
        self.submit(Command::SetDraft { card, text: text.map(str::to_string) })
    }

    fn grade(&self, card: Id, key: &str, grade: Grade, duration_ms: Option<i64>) -> u64 {
        self.submit(Command::Grade { card, key: key.to_string(), grade, duration_ms })
    }

    fn reset_history(&self, card: Id, key: &str) -> u64 {
        self.submit(Command::ResetHistory { card, key: key.to_string() })
    }

    fn set_setting(&self, key: &str, value: &str) -> u64 {
        self.submit(Command::SetSetting { key: key.to_string(), value: value.to_string() })
    }

    fn history(&self, card: Id) -> u64 {
        self.submit(Command::History { card })
    }

    fn stats(&self, from_ms: i64, to_ms: i64) -> u64 {
        self.submit(Command::Stats { from_ms, to_ms })
    }
}

fn work(mut db: Db, rx: Receiver<Message>, shared: Arc<Shared>, clock: Box<dyn Clock + Send + Sync>) {
    while let Ok(message) = rx.recv() {
        let id = message.id;
        let mut result = db.apply(&message.command, clock.now_ms()).map_err(|e| e.to_string());
        if result.is_ok() {
            match db.snapshot() {
                Ok(mut snapshot) => {
                    let mut published = shared.snapshot.lock().expect("snapshot");
                    snapshot.revision = published.revision + 1;
                    *published = snapshot;
                }
                Err(error) => result = Err(error.to_string()),
            }
        }
        shared.events.lock().expect("events").push(Event { id, result });
        (shared.waker)();
    }
}
