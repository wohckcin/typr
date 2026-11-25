//! Study session startup, relearning wakeup timer, and async grading wiring.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use nsh_core::prelude::*;
use typr_core::clock::{Clock, SystemClock};
use typr_core::day::study_day;
use typr_core::screen::Queue;
use typr_core::session::{Entry, Session};
use typr_core::store::{Command, Grade, Id, Outcome, Snapshot};

use crate::screens::study::{entries_for, study_async};

use super::Screen;
use super::store_bridge::StoreBridge;

/// How many study days before `now` a prompt fell due, which is what a session's `Entry` orders and
/// reshuffles by.
pub fn overdue_days(now_ms: i64, due_at: i64, offset_ms: i64) -> i64 {
    (study_day(now_ms, offset_ms) - study_day(due_at, offset_ms)).max(0)
}

/// Brings the relearning cards of `session` whose wait is over back into its queue. True when any
/// came back, so the screen shows them.
pub fn wake_session(session: &RefCell<Session>, now_ms: i64) -> bool {
    session.borrow_mut().advance_clock(now_ms) > 0
}

/// Coordinates starting study sessions, relearning wakeups, and async grading acknowledgements.
#[derive(Clone)]
pub(super) struct StudyController {
    bridge: StoreBridge,
    snapshot_signal: Signal<Snapshot>,
    screen: Signal<Screen>,
    version: Signal<u32>,
    refused: Signal<bool>,
}

impl StudyController {
    pub(super) fn new(
        bridge: StoreBridge,
        snapshot_signal: Signal<Snapshot>,
        screen: Signal<Screen>,
    ) -> Self {
        let version = signal(0_u32);
        let refused = signal(false);
        // A forgotten card waits out its relearning step inside the session; this tick lets it
        // back in and rebuilds the session screen to show it.
        set_interval(std::time::Duration::from_secs(1), move || {
            if let Screen::Study(session) = screen.get_untracked()
                && wake_session(&session, SystemClock.now_ms())
            {
                version.update(|count| *count += 1);
            }
        });
        Self {
            bridge,
            snapshot_signal,
            screen,
            version,
            refused,
        }
    }

    /// Requests the due or new queue from the store worker and opens the study screen when non-empty.
    pub(super) fn start(&self, queue: Queue) {
        let now = SystemClock.now_ms();
        let offset = crate::day_offset::local_offset_ms();
        let seed = now as u64;
        let command = match queue {
            Queue::Due => Command::DuePrompts {
                now_ms: now,
                offset_ms: offset,
                seed,
                deck: None,
            },
            Queue::New => Command::NewPrompts {
                now_ms: now,
                offset_ms: offset,
                seed,
                deck: None,
            },
        };
        let snapshot_signal = self.snapshot_signal;
        let screen = self.screen;
        let version = self.version;
        let refused = self.refused;
        self.bridge.submit(
            command,
            false,
            Some(Box::new(move |result| {
                let pairs: Vec<(Id, String, i64)> = match result {
                    Ok(Outcome::Due(prompts)) => prompts
                        .into_iter()
                        .map(|p| (p.card, p.key, overdue_days(now, p.due_at, offset)))
                        .collect(),
                    Ok(Outcome::New(prompts)) => {
                        prompts.into_iter().map(|p| (p.card, p.key, 0)).collect()
                    }
                    Ok(_) => Vec::new(),
                    Err(error) => {
                        log::error!(
                            "the queue was refused: {}",
                            crate::logging::describe(&error)
                        );
                        Vec::new()
                    }
                };
                let snapshot = snapshot_signal.get_untracked();
                let entries: Vec<Entry> = entries_for(&snapshot.cards, &pairs);
                if entries.is_empty() {
                    return;
                }
                let step = snapshot.settings.learning_step_ms();
                refused.set(false);
                version.update(|v| *v += 1);
                screen.set(Screen::Study(Rc::new(RefCell::new(
                    Session::new(queue, entries).with_learning_step(step),
                ))));
            })),
        );
    }

    /// Renders an active study session wired to async grading through the store worker.
    pub(super) fn render(&self, session: Rc<RefCell<Session>>) -> impl IntoElement {
        let bridge = self.bridge.clone();
        let bridge_leave = self.bridge.clone();
        let session_for_grade = Rc::clone(&session);
        let version = self.version;
        let refused = self.refused;
        let screen = self.screen;
        study_async(
            session,
            version,
            refused,
            move |card: Id, key: String, grade: Grade| {
                let command = Command::Grade {
                    card,
                    key,
                    grade,
                    duration_ms: None,
                };
                let session_done = Rc::clone(&session_for_grade);
                let id_cell = Rc::new(Cell::new(0_u64));
                let id_for_done = Rc::clone(&id_cell);
                let sent = bridge.submit(
                    command,
                    false,
                    Some(Box::new(move |result| {
                        let cmd_id = id_for_done.get();
                        match result {
                            Ok(_) => {
                                refused.set(false);
                                session_done
                                    .borrow_mut()
                                    .acknowledge(cmd_id, SystemClock.now_ms());
                            }
                            Err(_) => {
                                refused.set(true);
                                session_done.borrow_mut().refuse(cmd_id);
                            }
                        }
                        version.update(|v| *v += 1);
                    })),
                );
                if let Some(cmd_id) = sent {
                    id_cell.set(cmd_id);
                }
                sent
            },
            move || {
                screen.set(Screen::Library);
                bridge_leave.reopen();
            },
            move || screen.set(Screen::Settings),
        )
    }
}
