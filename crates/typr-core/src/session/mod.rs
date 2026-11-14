//! One study session: the queue it walks, what has been shown, and what has been graded.

use std::collections::HashSet;

use crate::clock::LEARNING_STEP_MS;
use crate::screen::Queue;
use crate::store::{Grade, Id};

/// One face of a card: what is on screen before it is unmasked, and what it reads as afterwards.
///
/// A side is its own question, so both strings hold the same text. Hidden text differs: the hidden
/// form carries a mask where the answer would be.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Face {
    pub hidden: String,
    pub shown: String,
}

impl Face {
    /// A face that asks a question by being itself: a side, or a whole hidden-text sentence.
    pub fn question(text: impl Into<String>) -> Face {
        let text = text.into();
        Face {
            hidden: text.clone(),
            shown: text,
        }
    }

    /// Whether unmasking this face changes what is on screen.
    pub fn unmasks(&self) -> bool {
        self.hidden != self.shown
    }
}

/// A card waiting its turn, with the faces its prompt asks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub card: Id,
    /// The prompt this entry reviews, such as `fwd` or `cloze:1`.
    pub prompt: String,
    pub faces: Vec<Face>,
    /// How many study days overdue this card was when the session started. Entries with a larger
    /// number stay in front of the others, so a reshuffle never promotes a card that is not due.
    pub overdue: i64,
}

impl Entry {
    /// A two-or-more-sided card: every side is its own question.
    pub fn sides<I, T>(card: Id, sides: I, overdue: i64) -> Entry
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        Entry {
            card,
            prompt: "fwd".to_string(),
            faces: sides.into_iter().map(Face::question).collect(),
            overdue,
        }
    }

    /// A two-or-more-sided card asked from its last side: the same faces, read the other way round,
    /// so the side that was the answer is the question.
    pub fn reverse<I, T>(card: Id, sides: I, overdue: i64) -> Entry
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        let mut faces: Vec<Face> = sides.into_iter().map(Face::question).collect();
        faces.reverse();
        Entry {
            card,
            prompt: "rev".to_string(),
            faces,
            overdue,
        }
    }

    /// A hidden-text group: one face, masked until it is unmasked.
    pub fn masked<H, S>(
        card: Id,
        prompt: impl Into<String>,
        hidden: H,
        shown: S,
        overdue: i64,
    ) -> Entry
    where
        H: Into<String>,
        S: Into<String>,
    {
        Entry {
            card,
            prompt: prompt.into(),
            faces: vec![Face {
                hidden: hidden.into(),
                shown: shown.into(),
            }],
            overdue,
        }
    }
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
    /// The grade sent to the store and not yet acknowledged, with the card it was given for: the
    /// answer names a card, never a position, because the user may move on before it lands.
    awaiting: Option<(u64, Id, Grade)>,
    /// A grade chosen but not yet sent.
    pending: Option<(Id, Grade)>,
    /// Cards whose relearning step is still in the future, and when it lands.
    waiting: Vec<(i64, Entry)>,
    /// Whether the current card is being revisited read-only.
    pub read_only: bool,
    /// Cards answered in this session. They stay in the queue so `backward` can show them again,
    /// and are skipped everywhere the queue is read forward.
    answered: HashSet<Id>,
    /// Cards that joined this session with a schedule already behind them. They keep the relearning
    /// route after a lapse even when the session itself was opened on the new-card queue.
    scheduled: HashSet<Id>,
}

impl Session {
    /// A session with nothing queued, as the app holds before one starts.
    pub fn empty() -> Session {
        Session::new(Queue::Due, Vec::new())
    }

    pub fn new(queue: Queue, entries: Vec<Entry>) -> Session {
        let mut session = Session {
            queue,
            entries,
            at: 0,
            revealed: 0,
            grades: Vec::new(),
            awaiting: None,
            pending: None,
            waiting: Vec::new(),
            read_only: false,
            answered: HashSet::new(),
            scheduled: HashSet::new(),
        };
        session.show_first_side();
        session
    }

    /// A card opens on its first side: the reader is never staring at an empty frame wondering
    /// which key shows the front.
    fn show_first_side(&mut self) {
        // A masked face is on screen before it is unmasked, so it starts with nothing revealed; a side
        // is its own question, so it starts shown. Both are on screen either way.
        self.revealed = match self.current() {
            Some(entry) if entry.faces.first().is_some_and(Face::unmasks) => 0,
            Some(_) => 1,
            None => 0,
        };
        self.read_only = false;
    }

    /// The card on screen, if there is one.
    pub fn current(&self) -> Option<&Entry> {
        self.entries.get(self.at)
    }

    /// Which way the card on screen is being asked: `"forward"`, or `"reverse"` when its prompt is
    /// the one read from the last side. Empty when nothing is on screen.
    pub fn direction(&self) -> &'static str {
        match self.current() {
            Some(entry) if entry.prompt == "rev" => "reverse",
            Some(_) => "forward",
            None => "",
        }
    }

    /// How many sides of the current card have been shown.
    pub fn revealed(&self) -> usize {
        self.revealed
    }

    /// The faces still hidden.
    pub fn hidden(&self) -> usize {
        self.current()
            .map_or(0, |entry| entry.faces.len().saturating_sub(self.revealed))
    }

    /// The next face, which revealing would show.
    pub fn next_side(&self) -> Option<&str> {
        let entry = self.current()?;
        entry
            .faces
            .get(self.revealed)
            .map(|face| face.shown.as_str())
    }

    /// The faces to draw, in order, with whether each is the unmasked answer.
    ///
    /// A masked face is drawn in its hidden form while it is the one being asked, which is what makes
    /// one reveal unmask it; a side is never drawn ahead of its turn, because its question and its
    /// answer are the same string and drawing it early would hand over the answer to the side before.
    pub fn lines(&self) -> Vec<(&str, bool)> {
        let Some(entry) = self.current() else {
            return Vec::new();
        };
        let mut lines = Vec::new();
        for (index, face) in entry.faces.iter().enumerate() {
            if index < self.revealed {
                lines.push((face.shown.as_str(), true));
            } else if index == self.revealed && face.unmasks() {
                lines.push((face.hidden.as_str(), false));
            }
        }
        lines
    }

    /// Whether the grade is offered: only after the last side, and not while revisiting.
    pub fn gradeable(&self) -> bool {
        match self.current() {
            Some(entry) => {
                !self.read_only && self.revealed >= entry.faces.len() && self.awaiting.is_none()
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
        let card = self.current()?.card;
        self.pending = Some((card, grade));
        Some((card, grade))
    }

    /// The store has taken the grade under `id`.
    pub fn sent(&mut self, id: u64) {
        if let Some((card, grade)) = self.pending.take() {
            self.awaiting = Some((id, card, grade));
        }
    }

    /// The grade never reached the store, so no answer is coming: drop it and leave the card
    /// answerable instead of waiting for an ack that will not arrive.
    pub fn unsent(&mut self) {
        self.pending = None;
    }

    /// The store did not take the grade: the card stays as it was, so it can be answered again.
    pub fn refuse(&mut self, id: u64) {
        if let Some((awaiting, _, _)) = self.awaiting {
            if awaiting != id {
                return;
            }
            self.awaiting = None;
        }
        self.pending = None;
    }

    /// The store has answered `id`; the session can move on.
    pub fn acknowledge(&mut self, id: u64, now_ms: i64) -> bool {
        let Some((awaiting, card, grade)) = self.awaiting else {
            return false;
        };
        if awaiting != id {
            return false;
        }
        self.awaiting = None;
        self.grades.push(grade);
        // A card that arrived with a schedule behind it keeps that schedule: it waits out its
        // relearning step instead of being handed straight back as if it were new.
        let queue = if self.scheduled.contains(&card) {
            Queue::Due
        } else {
            self.queue
        };
        match (queue, grade) {
            // A forgotten card is not answered: one leaves the queue for later, the other comes
            // straight back, and neither counts as done.
            (Queue::New, Grade::Forgot) => {
                if let Some(entry) = self.take(card) {
                    let empty = self.current().is_none();
                    self.entries.push(entry);
                    if empty {
                        self.show_first_side();
                    }
                }
            }
            (Queue::Due, Grade::Forgot) => {
                if let Some(entry) = self.take(card) {
                    self.waiting.push((now_ms + LEARNING_STEP_MS, entry));
                }
            }
            _ => self.answer(card),
        }
        true
    }

    /// Record that `card` is answered, and step over it if the cursor is on it.
    fn answer(&mut self, card: Id) {
        self.answered.insert(card);
        if self.entries.get(self.at).map(|entry| entry.card) == Some(card) {
            self.at += 1;
            self.seek();
            self.show_first_side();
        }
    }

    /// Skip answered cards: they are history, not something to offer again.
    fn seek(&mut self) {
        while self
            .entries
            .get(self.at)
            .is_some_and(|entry| self.answered.contains(&entry.card))
        {
            self.at += 1;
        }
    }

    /// Lift a card out of the queue wherever it sits, leaving the cursor on the card the user sees.
    fn take(&mut self, card: Id) -> Option<Entry> {
        let index = self.entries.iter().position(|entry| entry.card == card)?;
        let entry = self.entries.remove(index);
        if index < self.at {
            self.at -= 1;
        } else if index == self.at {
            self.show_first_side();
        }
        Some(entry)
    }

    /// Move to the next card without answering this one; it stays queued.
    pub fn forward(&mut self) -> bool {
        if self.read_only {
            self.at += 1;
            self.show_first_side();
            return true;
        }
        if self.current().is_none() {
            return false;
        }
        let entry = self.entries.remove(self.at);
        self.entries.push(entry);
        self.show_first_side();
        true
    }

    /// The cards still to answer, in the order they will come.
    pub fn order(&self) -> Vec<Id> {
        self.entries
            .get(self.at.min(self.entries.len())..)
            .unwrap_or_default()
            .iter()
            .filter(|entry| !self.answered.contains(&entry.card))
            .map(|entry| entry.card)
            .collect()
    }

    /// Reorder the cards still to answer, keeping the more overdue ones in front.
    ///
    /// Only cards sharing the same overdue day may swap places, which is what keeps a reshuffle
    /// from breaking the order the scheduler asked for.
    pub fn reshuffle(&mut self, seed: u64) {
        let before = self.current().map(|entry| entry.card);
        let mut shuffle = crate::shuffle::Shuffle::new(seed);
        let mut index = self.at;
        while index < self.entries.len() {
            if self.answered.contains(&self.entries[index].card) {
                index += 1;
                continue;
            }
            let overdue = self.entries[index].overdue;
            let mut end = index;
            while end < self.entries.len()
                && !self.answered.contains(&self.entries[end].card)
                && self.entries[end].overdue == overdue
            {
                end += 1;
            }
            shuffle.reorder(&mut self.entries[index..end]);
            index = end;
        }
        // A different card is on screen now, so it opens on its own first side.
        if self.current().map(|entry| entry.card) != before {
            self.show_first_side();
        }
    }

    /// Revisit the previous card read-only: its answer is already recorded.
    pub fn backward(&mut self) -> bool {
        if self.read_only || self.at == 0 {
            return false;
        }
        self.at -= 1;
        self.revealed = self.current().map_or(0, |entry| entry.faces.len());
        self.read_only = true;
        true
    }

    /// Whether this card is already part of the session, queued or waiting.
    pub fn holds(&self, card: Id) -> bool {
        self.entries.iter().any(|entry| entry.card == card)
            || self.waiting.iter().any(|(_, entry)| entry.card == card)
    }

    /// A prompt that fell due while the session was running joins it, with the schedule it had.
    pub fn arrive(&mut self, entry: Entry) {
        let empty = self.current().is_none();
        self.scheduled.insert(entry.card);
        self.entries.push(entry);
        if empty {
            self.show_first_side();
        }
    }

    /// Relearning steps whose moment has come rejoin the queue.
    pub fn advance_clock(&mut self, now_ms: i64) -> usize {
        let empty = self.current().is_none();
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
        if empty && landed > 0 {
            self.show_first_side();
        }
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
        self.order().len() + self.waiting.len()
    }

    /// Whether the session has walked everything that was queued.
    pub fn finished(&self) -> bool {
        self.order().is_empty() && self.waiting.is_empty()
    }
}

#[cfg(test)]
mod tests;
