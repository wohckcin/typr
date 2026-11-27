//! Async bridge between the background `"typr-store"` worker and the UI thread, plus transfer
//! entry points (import/export).

use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker as TaskWaker};

use nsh_core::file_dialog::{OpenDialog, SaveDialog};
use nsh_core::prelude::*;
use typr_core::clock::{Clock, SystemClock};
use typr_core::store::{
    Command, Db, Handle, Outcome, Snapshot, Store, StoreError, UNPROMPTED, Waker,
};

use crate::screens::diagnostics::SlowCommand;
use crate::ui::overlay::Toasts;

/// How long a store command may take before the diagnostics screen lists it, in milliseconds.
const SLOW_COMMAND_MS: u32 = 100;

/// Bridges notifications from the background `"typr-store"` thread onto the NSH UI async executor.
pub struct WakeSignal {
    inner: Mutex<WakeInner>,
}

struct WakeInner {
    pending: bool,
    closed: bool,
    waker: Option<TaskWaker>,
}

impl WakeSignal {
    /// A fresh wake bridge with no pending events.
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(WakeInner {
                pending: false,
                closed: false,
                waker: None,
            }),
        }
    }

    /// A [`Waker`] callback suitable for [`Handle::spawn`].
    pub fn waker(self: &Arc<Self>) -> Waker {
        let signal = Arc::clone(self);
        Arc::new(move || signal.notify())
    }

    fn notify(&self) {
        let waker = {
            let mut inner = self
                .inner
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            inner.pending = true;
            inner.waker.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }

    fn close(&self) {
        let waker = {
            let mut inner = self
                .inner
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            inner.closed = true;
            inner.waker.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }

    fn next(self: &Arc<Self>) -> WakeNext {
        WakeNext(Arc::clone(self))
    }
}

impl Default for WakeSignal {
    fn default() -> Self {
        Self::new()
    }
}

struct WakeNext(Arc<WakeSignal>);

impl Future for WakeNext {
    type Output = bool;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<bool> {
        let mut inner = self
            .0
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if inner.pending {
            inner.pending = false;
            return Poll::Ready(true);
        }
        if inner.closed {
            return Poll::Ready(false);
        }
        inner.waker = Some(cx.waker().clone());
        Poll::Pending
    }
}

/// Closes the wake bridge when the shell unmounts so its event loop exits cleanly.
pub(super) struct WakeGuard(Arc<WakeSignal>);

impl Drop for WakeGuard {
    fn drop(&mut self) {
        self.0.close();
    }
}

pub(super) type DoneCallback = Box<dyn FnOnce(Result<Outcome, StoreError>)>;

struct PendingCommand {
    name: &'static str,
    started: std::time::Instant,
    republish: bool,
    on_done: Option<DoneCallback>,
}

/// Owns command submission and worker event draining for one shell instance.
#[derive(Clone)]
pub(super) struct StoreBridge {
    handle: Option<Rc<Handle>>,
    pending: Rc<RefCell<HashMap<u64, PendingCommand>>>,
    snapshot_signal: Signal<Snapshot>,
    stopped: Signal<bool>,
}

impl StoreBridge {
    /// Creates a bridge and starts the UI-thread drain loop when a worker handle is present.
    ///
    /// Returns the guard that keeps the handle and wake loop alive for the lifetime of the shell's
    /// reactive owner.
    pub(super) fn new(
        store: Option<(Rc<Handle>, Arc<WakeSignal>)>,
        snapshot_signal: Signal<Snapshot>,
        stopped: Signal<bool>,
        slow: Signal<Vec<SlowCommand>>,
        results: Toasts,
    ) -> (Self, Option<(Rc<Handle>, WakeGuard)>) {
        let (handle, wake) = match store {
            Some((handle, wake)) => (Some(handle), Some(wake)),
            None => (None, None),
        };
        let pending: Rc<RefCell<HashMap<u64, PendingCommand>>> =
            Rc::new(RefCell::new(HashMap::new()));

        if let (Some(handle_rc), Some(wake_arc)) = (&handle, &wake) {
            let weak = Rc::downgrade(handle_rc);
            let pending = Rc::clone(&pending);
            let wake_loop = Arc::clone(wake_arc);
            let drain = move |handle: &Handle| {
                drain_events(handle, &pending, snapshot_signal, stopped, slow, results);
            };
            spawn(async move {
                while wake_loop.next().await {
                    let Some(handle) = weak.upgrade() else {
                        break;
                    };
                    drain(&handle);
                }
                if let Some(handle) = weak.upgrade() {
                    drain(&handle);
                }
            });
        }

        let owner = handle.clone().zip(wake.map(WakeGuard));
        let bridge = Self {
            handle,
            pending,
            snapshot_signal,
            stopped,
        };
        (bridge, owner)
    }

    /// Submits one command to the background store worker, returning its command id when accepted.
    pub(super) fn submit(
        &self,
        command: Command,
        republish: bool,
        on_done: Option<DoneCallback>,
    ) -> Option<u64> {
        let handle = self.handle.as_ref()?;
        let name = command.name();
        let started = std::time::Instant::now();
        match handle.submit(command) {
            Ok(id) => {
                self.pending.borrow_mut().insert(
                    id,
                    PendingCommand {
                        name,
                        started,
                        republish,
                        on_done,
                    },
                );
                Some(id)
            }
            Err(error) => {
                if error.is_fatal() {
                    self.stopped.set(true);
                }
                let level = if error.is_fatal() {
                    log::Level::Error
                } else {
                    log::Level::Warn
                };
                log::log!(
                    level,
                    "store refused a command: {}",
                    crate::logging::describe(&error)
                );
                if let Some(callback) = on_done {
                    callback(Err(error));
                }
                None
            }
        }
    }

    /// Submits a command and refreshes `snapshot_signal` when it finishes writing.
    pub(super) fn apply(&self, command: Command) {
        self.submit(command, true, None);
    }

    /// Submits a command without republishing the snapshot signal when it finishes.
    pub(super) fn quiet_apply(&self, command: Command) {
        self.submit(command, false, None);
    }

    /// Republishes the worker's latest snapshot immediately.
    pub(super) fn reopen(&self) {
        if let Some(handle) = self.handle.as_ref() {
            self.snapshot_signal.set((*handle.snapshot()).clone());
        }
    }

    /// Opens the save dialog and exports the library on the store worker.
    pub(super) fn spawn_export(&self, results: Toasts) {
        let bridge = self.clone();
        spawn(async move {
            match pick_save_path(SaveDialog::new().file_name("typr-library.sqlite3")).await {
                Ok(Some(path)) => {
                    let target = path.clone();
                    bridge.submit(
                        Command::ExportLibrary { target: path },
                        false,
                        Some(Box::new(move |result| {
                            results.push(export_line(result, &target).into());
                        })),
                    );
                }
                Ok(None) => {}
                Err(why) => results.push(why.into()),
            }
        });
    }

    /// Opens the file picker and imports a Mochi archive on the store worker.
    pub(super) fn spawn_import(&self, attachments: PathBuf, results: Toasts) {
        let bridge = self.clone();
        spawn(async move {
            match pick_paths(OpenDialog::files()).await {
                Ok(Some(paths)) => {
                    let Some(archive) = paths.into_iter().next() else {
                        return;
                    };
                    let label_path = archive.clone();
                    // Through the one store worker: a stoppage is flagged and the snapshot re-read.
                    bridge.submit(
                        import_command(&archive, &attachments),
                        true,
                        Some(Box::new(move |result| {
                            results.push(import_line(result, &label_path).into());
                        })),
                    );
                }
                Ok(None) => {}
                Err(why) => results.push(why.into()),
            }
        });
    }
}

fn drain_events(
    handle: &Handle,
    pending: &RefCell<HashMap<u64, PendingCommand>>,
    snapshot_signal: Signal<Snapshot>,
    stopped: Signal<bool>,
    slow: Signal<Vec<SlowCommand>>,
    results: Toasts,
) {
    for event in handle.poll() {
        if event.id == UNPROMPTED {
            if let Err(error) = event.result {
                let why = crate::logging::describe(&error);
                if error.is_fatal() {
                    stopped.set(true);
                    log::error!("store worker reported an error: {why}");
                } else {
                    // A snapshot that could not be re-read: the worker keeps running.
                    log::warn!("store worker reported an error: {why}");
                    results.push(format!("The library could not be re-read: {why}").into());
                }
            }
            continue;
        }
        let entry = pending.borrow_mut().remove(&event.id);
        let (republish, on_done) = match entry {
            Some(entry) => {
                match &event.result {
                    Ok(_) => {
                        let millis =
                            u32::try_from(entry.started.elapsed().as_millis()).unwrap_or(u32::MAX);
                        if millis >= SLOW_COMMAND_MS {
                            slow.update(|list| {
                                list.push(SlowCommand {
                                    name: entry.name.into(),
                                    millis,
                                })
                            });
                        }
                    }
                    Err(error) => {
                        if error.is_fatal() {
                            stopped.set(true);
                        }
                        let level = if error.is_fatal() {
                            log::Level::Error
                        } else {
                            log::Level::Warn
                        };
                        log::log!(
                            level,
                            "store refused a command: {}",
                            crate::logging::describe(error)
                        );
                    }
                }
                (entry.republish, entry.on_done)
            }
            None => (true, None),
        };
        if republish && event.result.as_ref().is_ok_and(Outcome::writes) {
            snapshot_signal.set((*handle.snapshot()).clone());
        }
        if let Some(callback) = on_done {
            callback(event.result);
        }
    }
}

/// Exports the library to `target` and gives the line the diagnostics screen states.
pub fn export_to(db: &Db, target: &Path) -> String {
    export_line(
        db.apply(
            &Command::ExportLibrary {
                target: target.to_path_buf(),
            },
            SystemClock.now_ms(),
        ),
        target,
    )
}

fn export_line(result: Result<Outcome, StoreError>, target: &Path) -> String {
    match result {
        Ok(_) => format!("Exported to {}", target.display()),
        Err(error) => crate::logging::describe(&error),
    }
}

/// Imports the Mochi archive at `archive` and gives the line the diagnostics screen states: what
/// arrived, or why the file was refused.
pub fn import_from(db: &Db, archive: &Path, attachments: &Path) -> String {
    let command = import_command(archive, attachments);
    import_line(db.apply(&command, SystemClock.now_ms()), archive)
}

fn import_command(archive: &Path, attachments: &Path) -> Command {
    Command::ImportArchive {
        archive: archive.to_path_buf(),
        attachments: attachments.to_path_buf(),
    }
}

fn import_line(result: Result<Outcome, StoreError>, archive: &Path) -> String {
    match result {
        Ok(Outcome::Imported(report)) => format!(
            "Imported {} decks, {} cards, {} reviews and {} pictures from {}",
            report.decks,
            report.cards,
            report.reviews,
            report.images,
            archive
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("the archive")
        ),
        Ok(_) => "Imported".to_owned(),
        Err(error) => crate::logging::describe(&error),
    }
}
