//! The store task: one worker thread owns the `Connection`, the UI only ever sends commands.
//!
//! Every command carries an id and comes back as exactly one [`Event`], so "the grade is written
//! before the next card is shown" is a property of the protocol rather than a hope. When a
//! command arrives, the worker applies it, re-publishes the snapshot, records the event and wakes
//! the frame loop. `poll` returns only the events that arrived since the last call, which is what
//! lets `AppSpec::poll` report "nothing happened" and cost an idle frame nothing.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;

use crate::clock::Clock;
use crate::command::{Command, Event, Outcome, Store};
use crate::error::StoreError;
use crate::library_view::ViewMode;
use crate::model::{
    Grade, Id, MaskRect, SchedulerKind, SettingKey, SettingValue, Snapshot, ViewSpec,
};

use super::db::Db;

/// What the worker calls once its answer is published and readable.
pub type Waker = Arc<dyn Fn() + Send + Sync>;

/// The id an [`Event`] carries when it answers no request: the worker says what went wrong without
/// a caller having asked. The UI shows its error and matches it against nothing.
pub const UNPROMPTED: u64 = 0;

/// Everything the worker publishes for the UI thread.
struct Shared {
    snapshot: Mutex<Arc<Snapshot>>,
    events: Mutex<Vec<Event>>,
    waker: Waker,
}

/// How the worker's thread ended, readable after the [`Handle`] is gone.
///
/// `Drop for Handle` runs during the app's teardown, so nothing can ask the handle anything
/// afterwards. The application keeps a clone of this instead, and records it at quit.
#[derive(Clone, Default)]
pub struct WorkerStatus(Arc<Mutex<Option<Arc<StoreError>>>>);

impl WorkerStatus {
    /// The error the worker ended with, if it ended abnormally.
    pub fn ended(&self) -> Option<Arc<StoreError>> {
        lock(&self.0).as_ref().map(Arc::clone)
    }

    fn set(&self, error: StoreError) {
        *lock(&self.0) = Some(Arc::new(error));
    }
}

/// The UI's end of the store task.
pub struct Handle {
    tx: Mutex<Option<Sender<Message>>>,
    worker: Mutex<Option<JoinHandle<()>>>,
    shared: Arc<Shared>,
    status: WorkerStatus,
    next_id: AtomicU64,
}

struct Message {
    id: u64,
    command: Command,
}

impl Handle {
    /// Start the worker on an open database.
    pub fn spawn(
        db: Db,
        clock: Box<dyn Clock + Send + Sync>,
        waker: Waker,
    ) -> Result<Self, StoreError> {
        let snapshot = db.snapshot()?;
        let shared = Arc::new(Shared {
            snapshot: Mutex::new(Arc::new(snapshot)),
            events: Mutex::new(Vec::new()),
            waker,
        });
        let (tx, rx) = mpsc::channel();
        let status = WorkerStatus::default();
        let worker = {
            let shared = Arc::clone(&shared);
            let status = status.clone();
            std::thread::Builder::new()
                .name("typr-store".to_string())
                .spawn(move || work(db, rx, shared, clock, status))?
        };
        Ok(Self {
            tx: Mutex::new(Some(tx)),
            worker: Mutex::new(Some(worker)),
            shared,
            status,
            next_id: AtomicU64::new(0),
        })
    }

    /// How the worker's thread ends, observable after this handle is dropped.
    pub fn status(&self) -> WorkerStatus {
        self.status.clone()
    }

    /// Send the last flush and wait for it: the one place the frame thread blocks, because a
    /// grade acknowledged but not yet written is a lost grade.
    pub fn close(&self) {
        drop(lock(&self.tx).take());
        let worker = lock(&self.worker).take();
        if let Some(worker) = worker
            && let Err(payload) = worker.join()
        {
            // A panic that escaped the boundary inside the loop: record it rather than swallow it.
            self.status.set(StoreError::WorkerPanicked {
                message: panic_message(&payload),
            });
        }
    }

    /// Send a command and answer its id.
    ///
    /// A closed worker means a quit is in progress: the command cannot land, so the caller is told
    /// rather than left waiting for an answer that will never come.
    pub fn submit(&self, command: Command) -> Result<u64, StoreError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed) + 1;
        let sender = lock(&self.tx);
        let tx = sender.as_ref().ok_or(StoreError::Closed)?;
        #[expect(
            clippy::map_err_ignore,
            reason = "a SendError only means the worker is gone, which is Closed"
        )]
        tx.send(Message { id, command })
            .map_err(|_| StoreError::Closed)?;
        Ok(id)
    }
}

impl Drop for Handle {
    /// Dropping the handle is a quit: the worker drains what it was sent and the join waits for
    /// it. The app has no quit hook, so nothing else would ever call [`Handle::close`].
    fn drop(&mut self) {
        self.close();
    }
}

impl Store for Handle {
    fn snapshot(&self) -> Arc<Snapshot> {
        // Cloning the Arc keeps a grade from copying the whole library on the frame thread.
        Arc::clone(&lock(&self.shared.snapshot))
    }

    fn poll(&self) -> Vec<Event> {
        std::mem::take(&mut *lock(&self.shared.events))
    }

    fn due_prompts(
        &self,
        now_ms: i64,
        offset_ms: i64,
        seed: u64,
        deck: Option<Id>,
    ) -> Result<u64, StoreError> {
        self.submit(Command::DuePrompts {
            now_ms,
            offset_ms,
            seed,
            deck,
        })
    }

    fn new_prompts(
        &self,
        now_ms: i64,
        offset_ms: i64,
        seed: u64,
        deck: Option<Id>,
    ) -> Result<u64, StoreError> {
        self.submit(Command::NewPrompts {
            now_ms,
            offset_ms,
            seed,
            deck,
        })
    }

    fn cards_for_view(&self, spec: &ViewSpec) -> Result<u64, StoreError> {
        self.submit(Command::CardsForView { spec: spec.clone() })
    }

    fn create_deck(&self, parent: Option<Id>, name: &str) -> Result<u64, StoreError> {
        self.submit(Command::CreateDeck {
            parent,
            name: name.to_string(),
        })
    }

    fn rename_deck(&self, id: Id, name: &str) -> Result<u64, StoreError> {
        self.submit(Command::RenameDeck {
            id,
            name: name.to_string(),
        })
    }

    fn archive_deck(&self, id: Id, archived: bool) -> Result<u64, StoreError> {
        self.submit(Command::ArchiveDeck { id, archived })
    }

    fn move_deck(&self, id: Id, parent: Option<Id>) -> Result<u64, StoreError> {
        self.submit(Command::MoveDeck { id, parent })
    }

    fn create_card(&self, deck: Id, markdown: &str) -> Result<u64, StoreError> {
        self.submit(Command::CreateCard {
            deck,
            markdown: markdown.to_string(),
        })
    }

    fn save_card(&self, id: Id, markdown: &str) -> Result<u64, StoreError> {
        self.submit(Command::SaveCard {
            id,
            markdown: markdown.to_string(),
        })
    }

    fn move_card(&self, id: Id, deck: Id) -> Result<u64, StoreError> {
        self.submit(Command::MoveCard { id, deck })
    }

    fn set_tags(&self, card: Id, tags: &[String]) -> Result<u64, StoreError> {
        self.submit(Command::SetTags {
            card,
            tags: tags.to_vec(),
        })
    }

    fn archive_card(&self, id: Id, archived: bool) -> Result<u64, StoreError> {
        self.submit(Command::ArchiveCard { id, archived })
    }

    fn trash_card(&self, id: Id) -> Result<u64, StoreError> {
        self.submit(Command::TrashCard { id })
    }

    fn restore_card(&self, id: Id) -> Result<u64, StoreError> {
        self.submit(Command::RestoreCard { id })
    }

    fn set_draft(&self, card: Id, text: Option<&str>) -> Result<u64, StoreError> {
        self.submit(Command::SetDraft {
            card,
            text: text.map(str::to_string),
        })
    }

    fn draft(&self, card: Id) -> Result<u64, StoreError> {
        self.submit(Command::Draft { card })
    }

    fn grade(
        &self,
        card: Id,
        key: &str,
        grade: Grade,
        duration_ms: Option<i64>,
    ) -> Result<u64, StoreError> {
        self.submit(Command::Grade {
            card,
            key: key.to_string(),
            grade,
            duration_ms,
        })
    }

    fn reset_history(&self, card: Id, key: &str) -> Result<u64, StoreError> {
        self.submit(Command::ResetHistory {
            card,
            key: key.to_string(),
        })
    }

    fn empty_trash(&self) -> Result<u64, StoreError> {
        self.submit(Command::EmptyTrash)
    }

    fn set_setting(&self, key: SettingKey, value: SettingValue) -> Result<u64, StoreError> {
        self.submit(Command::SetSetting { key, value })
    }

    fn set_scheduler(&self, kind: SchedulerKind) -> Result<u64, StoreError> {
        self.submit(Command::SetScheduler { kind })
    }

    fn set_retention(&self, percent: u32) -> Result<u64, StoreError> {
        self.submit(Command::SetRetention { percent })
    }

    fn set_weights(&self, weights: &[f64]) -> Result<u64, StoreError> {
        self.submit(Command::SetWeights {
            weights: weights.to_vec(),
        })
    }

    fn set_card_reverse(&self, card: Id, reverse: Option<bool>) -> Result<u64, StoreError> {
        self.submit(Command::SetCardReverse { card, reverse })
    }

    fn set_deck_reverse(&self, deck: Id, reverse: Option<bool>) -> Result<u64, StoreError> {
        self.submit(Command::SetDeckReverse { deck, reverse })
    }

    fn set_deck_view(&self, deck: Id, view: ViewMode) -> Result<u64, StoreError> {
        self.submit(Command::SetDeckView { deck, view })
    }

    fn move_card_to(&self, card: Id, position: u32) -> Result<u64, StoreError> {
        self.submit(Command::MoveCardTo { card, position })
    }

    fn create_diagram(&self, deck: Id, image: &str, caption: &str) -> Result<u64, StoreError> {
        self.submit(Command::CreateDiagram {
            deck,
            image: image.to_string(),
            caption: caption.to_string(),
        })
    }

    fn set_fields(&self, card: Id, fields: &[(String, String)]) -> Result<u64, StoreError> {
        self.submit(Command::SetFields {
            card,
            fields: fields.to_vec(),
        })
    }

    fn set_card_template(
        &self,
        card: Id,
        template: Option<&str>,
        name: Option<&str>,
    ) -> Result<u64, StoreError> {
        self.submit(Command::SetCardTemplate {
            card,
            template: template.map(str::to_string),
            name: name.map(str::to_string),
        })
    }

    fn set_deck_template(&self, deck: Id, template: Option<&str>) -> Result<u64, StoreError> {
        self.submit(Command::SetDeckTemplate {
            deck,
            template: template.map(str::to_string),
        })
    }

    fn set_diagram(&self, card: Id, image: &str, caption: &str) -> Result<u64, StoreError> {
        self.submit(Command::SetDiagram {
            card,
            image: image.to_string(),
            caption: caption.to_string(),
        })
    }

    fn add_mask(&self, card: Id, group: u32, rect: MaskRect) -> Result<u64, StoreError> {
        self.submit(Command::AddMask { card, group, rect })
    }

    fn set_mask_group(&self, card: Id, rect: MaskRect, group: u32) -> Result<u64, StoreError> {
        self.submit(Command::SetMaskGroup { card, rect, group })
    }

    fn remove_mask(&self, card: Id, rect: MaskRect) -> Result<u64, StoreError> {
        self.submit(Command::RemoveMask { card, rect })
    }

    fn remove_mask_group(&self, card: Id, group: u32) -> Result<u64, StoreError> {
        self.submit(Command::RemoveMaskGroup { card, group })
    }

    fn clear_diagram(&self, card: Id) -> Result<u64, StoreError> {
        self.submit(Command::ClearDiagram { card })
    }

    fn merge_masks(&self, card: Id, rects: &[MaskRect], group: u32) -> Result<u64, StoreError> {
        self.submit(Command::MergeMasks {
            card,
            rects: rects.to_vec(),
            group,
        })
    }

    fn mask_groups(&self, card: Id) -> Result<u64, StoreError> {
        self.submit(Command::MaskGroups { card })
    }

    fn masks(&self, card: Id) -> Result<u64, StoreError> {
        self.submit(Command::Masks { card })
    }

    fn create_view(&self, spec: &ViewSpec, name: &str) -> Result<u64, StoreError> {
        self.submit(Command::CreateView {
            spec: spec.clone(),
            name: name.to_string(),
        })
    }

    fn rename_view(&self, id: Id, name: &str) -> Result<u64, StoreError> {
        self.submit(Command::RenameView {
            id,
            name: name.to_string(),
        })
    }

    fn delete_view(&self, id: Id) -> Result<u64, StoreError> {
        self.submit(Command::DeleteView { id })
    }

    fn update_view(&self, id: Id, spec: &ViewSpec) -> Result<u64, StoreError> {
        self.submit(Command::UpdateView {
            id,
            spec: spec.clone(),
        })
    }

    fn set_pinned(&self, card: Id, pinned: bool) -> Result<u64, StoreError> {
        self.submit(Command::SetPinned { card, pinned })
    }

    fn history(&self, card: Id) -> Result<u64, StoreError> {
        self.submit(Command::History { card })
    }

    fn stats(&self, from_ms: i64, to_ms: i64) -> Result<u64, StoreError> {
        self.submit(Command::Stats { from_ms, to_ms })
    }

    fn import_archive(
        &self,
        archive: &std::path::Path,
        attachments: &std::path::Path,
    ) -> Result<u64, StoreError> {
        self.submit(Command::ImportArchive {
            archive: archive.to_path_buf(),
            attachments: attachments.to_path_buf(),
        })
    }

    fn export_library(&self, target: &std::path::Path) -> Result<u64, StoreError> {
        self.submit(Command::ExportLibrary {
            target: target.to_path_buf(),
        })
    }

    fn import_csv(&self, deck: Id, source: &std::path::Path) -> Result<u64, StoreError> {
        self.submit(Command::ImportCsv {
            deck,
            source: source.to_path_buf(),
        })
    }

    fn export_csv(&self, deck: Id, target: &std::path::Path) -> Result<u64, StoreError> {
        self.submit(Command::ExportCsv {
            deck,
            target: target.to_path_buf(),
        })
    }

    fn import_markdown(&self, deck: Id, source: &std::path::Path) -> Result<u64, StoreError> {
        self.submit(Command::ImportMarkdown {
            deck,
            source: source.to_path_buf(),
        })
    }

    fn export_markdown(&self, deck: Id, target: &std::path::Path) -> Result<u64, StoreError> {
        self.submit(Command::ExportMarkdown {
            deck,
            target: target.to_path_buf(),
        })
    }
}

fn work(
    db: Db,
    rx: Receiver<Message>,
    shared: Arc<Shared>,
    clock: Box<dyn Clock + Send + Sync>,
    status: WorkerStatus,
) {
    while let Ok(message) = rx.recv() {
        let mut batch = Vec::new();
        // The id is copied out before `apply` consumes the message: if that call panics, this is
        // the only place the answer can still be addressed to. It follows the batch, so a panic on
        // the third of five is answered against the third.
        let in_flight = std::cell::Cell::new(message.id);
        // Coalesce: apply everything already queued before paying for the snapshot re-read, so a
        // bulk import costs one read rather than one per row.
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            let mut dirty = apply(&db, &*clock, message, &mut batch);
            while let Ok(next) = rx.try_recv() {
                in_flight.set(next.id);
                dirty |= apply(&db, &*clock, next, &mut batch);
            }
            dirty
        }));
        let dirty = match outcome {
            Ok(wrote) => wrote,
            Err(payload) => {
                // `AssertUnwindSafe` is sound here because the boundary never reuses what the
                // panic may have left half-touched: it answers the ids and stops the loop. The
                // database is consistent because `Db::apply` runs one transaction per command.
                let message = panic_message(&payload);
                batch.push(Event {
                    id: in_flight.get(),
                    result: Err(StoreError::WorkerPanicked {
                        message: message.clone(),
                    }),
                });
                // Everything still queued was accepted, so every one of those sends answered with
                // an id. Those messages die with the receiver, so they are answered here.
                while let Ok(orphan) = rx.try_recv() {
                    batch.push(Event {
                        id: orphan.id,
                        result: Err(StoreError::Closed),
                    });
                }
                // Last, so the failure line ends up saying what actually happened.
                batch.push(Event {
                    id: UNPROMPTED,
                    result: Err(StoreError::WorkerPanicked {
                        message: message.clone(),
                    }),
                });
                lock(&shared.events).extend(batch);
                (shared.waker)();
                status.set(StoreError::WorkerPanicked { message });
                return;
            }
        };
        if dirty && let Some(error) = publish(&db, &shared) {
            batch.push(Event {
                id: UNPROMPTED,
                result: Err(error),
            });
        }
        lock(&shared.events).extend(batch);
        (shared.waker)();
    }
}

/// The text a panic payload carries, or its type when it carries none.
fn panic_message(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        return (*text).to_string();
    }
    if let Some(text) = payload.downcast_ref::<String>() {
        return text.clone();
    }
    // A payload that is neither `&str` nor `String` carries no readable text, and its concrete
    // type is erased behind `dyn Any`, so the record says what it can.
    "panic with a payload that carries no message".to_string()
}

/// Apply one command and record its answer. Returns whether the command wrote to the library.
fn apply(db: &Db, clock: &dyn Clock, message: Message, batch: &mut Vec<Event>) -> bool {
    let result = db.apply(&message.command, clock.now_ms());
    let wrote = result.as_ref().is_ok_and(Outcome::writes);
    batch.push(Event {
        id: message.id,
        result,
    });
    wrote
}

/// Re-read the library and publish it with the next revision.
///
/// A snapshot that cannot be read must not turn a committed command into a failure — the caller
/// would retry work the database has already done — so the error comes back to be reported as an
/// [`UNPROMPTED`] event and the UI shows it.
fn publish(db: &Db, shared: &Shared) -> Option<StoreError> {
    match db.snapshot() {
        Ok(mut snapshot) => {
            let mut published = lock(&shared.snapshot);
            snapshot.revision = published.revision + 1;
            *published = Arc::new(snapshot);
            None
        }
        Err(error) => Some(error),
    }
}

/// Take a lock the worker may have left poisoned.
///
/// A panic while the lock was held would otherwise turn every later frame into a panic too. What
/// sits behind these locks is a published snapshot and a queue of answers, and both are safe to
/// read after a panic: the snapshot is either the last published library or the one being built.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests;
