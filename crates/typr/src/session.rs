//! One study session: the queue it walks, what has been shown, and what has been graded.

use typr_core::clock::LEARNING_STEP_MS;
use crate::screen::Queue;
use crate::store::{Grade, Id};

/// A card waiting its turn, with the sides as they will be revealed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub card: Id,
    pub sides: Vec<String>,
    /// How many study days overdue this card was when the session started. Entries with a larger
    /// number stay in front of the others, so a reshuffle never promotes a card that is not due.
    pub overdue: i64,
}

/// The cards this session is walking, in order.
#[derive(Debug)]
pub struct Session {
    pub queue: Queue,
    entries: Vec<Entry>,
    at: usize,
    revealed: usize,
    /// Grades recorded in this session, in the order they were answered.
    pub grades: Vec<Grade>,
    /// The grade sent to the store and not yet acknowledged.
    awaiting: Option<(u64, Grade)>,
    /// A grade chosen but not yet sent.
    pending: Option<Grade>,
    /// Cards whose relearning step is still in the future, and when it lands.
    waiting: Vec<(i64, Entry)>,
    /// Whether the current card is being revisited read-only.
    pub read_only: bool,
}

impl Session {
    /// A session with nothing queued, as the app holds before one starts.
    pub fn empty() -> Session {
        Session::new(Queue::Due, Vec::new())
    }

    pub fn new(queue: Queue, entries: Vec<Entry>) -> Session {
        Session {
            queue,
            entries,
            at: 0,
            revealed: 0,
            grades: Vec::new(),
            awaiting: None,
            pending: None,
            waiting: Vec::new(),
            read_only: false,
        }
    }

    /// The card on screen, if there is one.
    pub fn current(&self) -> Option<&Entry> {
        self.entries.get(self.at)
    }

    /// How many sides of the current card have been shown.
    pub fn revealed(&self) -> usize {
        self.revealed
    }

    /// The sides still hidden.
    pub fn hidden(&self) -> usize {
        self.current()
            .map_or(0, |entry| entry.sides.len().saturating_sub(self.revealed))
    }

    /// The next side, which revealing would show.
    pub fn next_side(&self) -> Option<&str> {
        let entry = self.current()?;
        entry.sides.get(self.revealed).map(String::as_str)
    }

    /// Whether the grade is offered: only after the last side, and not while revisiting.
    pub fn gradeable(&self) -> bool {
        match self.current() {
            Some(entry) => {
                !self.read_only && self.revealed >= entry.sides.len() && self.awaiting.is_none()
            }
            None => false,
        }
    }

    /// Show the next side.
    pub fn reveal(&mut self) -> bool {
        if self.read_only || self.hidden() == 0 {
            return false;
        }
        self.revealed += 1;
        true
    }

    /// Choose a grade for the card on screen. The caller sends it and reports the id back.
    pub fn grade(&mut self, grade: Grade) -> Option<(Id, Grade)> {
        if !self.gradeable() || self.pending.is_some() {
            return None;
        }
        self.pending = Some(grade);
        Some((self.current()?.card, grade))
    }

    /// The store has taken the grade under `id`.
    pub fn sent(&mut self, id: u64) {
        if let Some(grade) = self.pending.take() {
            self.awaiting = Some((id, grade));
        }
    }

    /// The grade never reached the store, so no answer is coming: drop it and leave the card
    /// answerable instead of waiting for an ack that will not arrive.
    pub fn unsent(&mut self) {
        self.pending = None;
    }

    /// The store did not take the grade: the card stays as it was, so it can be answered again.
    pub fn refuse(&mut self, id: u64) {
        if let Some((awaiting, _)) = self.awaiting {
            if awaiting != id {
                return;
            }
            self.awaiting = None;
        }
        self.pending = None;
    }

    /// The store has answered `id`; the session can move on.
    pub fn acknowledge(&mut self, id: u64, now_ms: i64) -> bool {
        let Some((awaiting, grade)) = self.awaiting else {
            return false;
        };
        if awaiting != id {
            return false;
        }
        self.awaiting = None;
        self.grades.push(grade);
        match (self.queue, grade) {
            // A forgotten new card goes back in the queue: nothing is scheduled yet.
            (Queue::New, Grade::Forgot) => {
                if self.at < self.entries.len() {
                    let entry = self.entries.remove(self.at);
                    self.entries.push(entry);
                }
                self.revealed = 0;
            }
            // A forgotten due card is in a relearning step: it comes back when that step lands.
            (Queue::Due, Grade::Forgot) => {
                if self.at < self.entries.len() {
                    let entry = self.entries.remove(self.at);
                    self.waiting.push((now_ms + LEARNING_STEP_MS, entry));
                }
                self.revealed = 0;
            }
            _ => {
                self.at += 1;
                self.revealed = 0;
            }
        }
        true
    }

    /// Move to the next card without answering this one; it stays queued.
    pub fn forward(&mut self) -> bool {
        if self.read_only {
            self.read_only = false;
            self.at += 1;
            self.revealed = 0;
            return true;
        }
        if self.current().is_none() {
            return false;
        }
        let entry = self.entries.remove(self.at);
        self.entries.push(entry);
        self.revealed = 0;
        true
    }

    /// The cards still to answer, in the order they will come.
    pub fn order(&self) -> Vec<Id> {
        self.entries
            .get(self.at.min(self.entries.len())..)
            .unwrap_or_default()
            .iter()
            .map(|entry| entry.card)
            .collect()
    }

    /// Reorder the cards still to answer, keeping the more overdue ones in front.
    ///
    /// Only cards sharing the same overdue day may swap places, which is what keeps a reshuffle
    /// from breaking the order the scheduler asked for.
    pub fn reshuffle(&mut self, seed: u64) {
        let mut shuffle = typr_core::shuffle::Shuffle::new(seed);
        let mut index = self.at;
        while index < self.entries.len() {
            let overdue = self.entries[index].overdue;
            let mut end = index;
            while end < self.entries.len() && self.entries[end].overdue == overdue {
                end += 1;
            }
            shuffle.reorder(&mut self.entries[index..end]);
            index = end;
        }
    }

    /// Revisit the previous card read-only: its answer is already recorded.
    pub fn backward(&mut self) -> bool {
        if self.read_only || self.at == 0 {
            return false;
        }
        self.at -= 1;
        self.revealed = self.current().map_or(0, |entry| entry.sides.len());
        self.read_only = true;
        true
    }

    /// A prompt that fell due while the session was running joins it.
    pub fn arrive(&mut self, entry: Entry) {
        self.entries.push(entry);
    }

    /// Relearning steps whose moment has come rejoin the queue.
    pub fn advance_clock(&mut self, now_ms: i64) -> usize {
        let mut landed = 0;
        let mut still_waiting = Vec::new();
        for (due_at, entry) in self.waiting.drain(..) {
            if due_at <= now_ms {
                self.entries.push(entry);
                landed += 1;
            } else {
                still_waiting.push((due_at, entry));
            }
        }
        self.waiting = still_waiting;
        landed
    }

    /// How long until the next relearning step lands, in milliseconds.
    pub fn next_wake(&self, now_ms: i64) -> Option<i64> {
        self.waiting
            .iter()
            .map(|(due_at, _)| (due_at - now_ms).max(0))
            .min()
    }

    /// Cards left to answer, including those still in a relearning step.
    pub fn remaining(&self) -> usize {
        self.entries.len().saturating_sub(self.at) + self.waiting.len()
    }

    /// Whether the session has walked everything that was queued.
    pub fn finished(&self) -> bool {
        self.at >= self.entries.len() && self.waiting.is_empty()
    }
}

#[cfg(test)]
mod tests;
