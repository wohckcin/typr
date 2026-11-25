//! One study session: the queue it walks, what has been shown, and what has been graded.

use std::collections::HashSet;

use crate::clock::LEARNING_STEP_MS;
use crate::hidden;
use crate::model::{CardSummary, Grade, Id, MaskRect};
use crate::screen::Queue;
use crate::sides;

/// One face of a card: what is on screen before it is unmasked, and what it reads as afterwards.
///
/// A side is its own question, so both strings hold the same text. Hidden text differs: the hidden
/// form carries a mask where the answer would be. A picture face has no text at all: it is covered
/// over the group being asked and uncovered on reveal.
#[derive(Clone, Debug, PartialEq)]
pub enum Face {
    /// A face made of text: a side, or a sentence with its answer hidden inside it.
    Text { hidden: String, shown: String },
    /// A face made of a picture: the rectangles of the group being asked are covered until the
    /// answer is revealed. The corners are fractions of the image, so a face holds at any size.
    Image {
        image: String,
        caption: Option<String>,
        masks: Vec<MaskRect>,
    },
}

impl Face {
    /// A face that asks a question by being itself: a side, or a whole hidden-text sentence.
    pub fn question(text: impl Into<String>) -> Face {
        let text = text.into();
        Face::Text {
            hidden: text.clone(),
            shown: text,
        }
    }

    /// A hidden-text face: the same text twice, once with the group masked.
    pub fn masked(hidden: impl Into<String>, shown: impl Into<String>) -> Face {
        Face::Text {
            hidden: hidden.into(),
            shown: shown.into(),
        }
    }

    /// A group of rectangles on a picture: one answer, whatever the group holds.
    pub fn group(image: impl Into<String>, caption: Option<String>, masks: Vec<MaskRect>) -> Face {
        Face::Image {
            image: image.into(),
            caption,
            masks,
        }
    }

    /// Whether unmasking this face changes what is on screen.
    pub fn unmasks(&self) -> bool {
        match self {
            Face::Text { hidden, shown } => hidden != shown,
            Face::Image { masks, .. } => !masks.is_empty(),
        }
    }

    /// The text of this face, if it is made of text.
    pub fn text(&self) -> Option<(&str, &str)> {
        match self {
            Face::Text { hidden, shown } => Some((hidden, shown)),
            Face::Image { .. } => None,
        }
    }

    /// The picture and the rectangles this face covers, if it is made of a picture.
    pub fn picture(&self) -> Option<(&str, Option<&str>, &[MaskRect])> {
        match self {
            Face::Image {
                image,
                caption,
                masks,
            } => Some((image, caption.as_deref(), masks)),
            Face::Text { .. } => None,
        }
    }
}

/// A card waiting its turn, with the faces its prompt asks.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
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

    /// A group of rectangles on a diagram: one face, covered until it is uncovered.
    pub fn mask_group(
        card: Id,
        prompt: impl Into<String>,
        image: impl Into<String>,
        caption: Option<String>,
        masks: Vec<MaskRect>,
        overdue: i64,
    ) -> Entry {
        Entry {
            card,
            prompt: prompt.into(),
            faces: vec![Face::group(image, caption, masks)],
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
            faces: vec![Face::masked(hidden, shown)],
            overdue,
        }
    }
}

/// The entry one prompt walks: a hidden-text group masked until revealed, a diagram group covered
/// until uncovered, or the sides of a card asked forwards or backwards. `None` for a key the card
/// no longer holds (a prompt retired after an edit).
pub fn entry_for(card: &CardSummary, key: &str, overdue: i64) -> Option<Entry> {
    if let Some(group) = key.strip_prefix("mask:") {
        let group: u32 = group.parse().ok()?;
        let masks: Vec<MaskRect> = card
            .masks
            .iter()
            .filter(|row| row.group == group)
            .map(|row| row.rect)
            .collect();
        if masks.is_empty() {
            return None;
        }
        return Some(Entry::mask_group(
            card.id,
            key,
            card.image.clone().unwrap_or_default(),
            card.caption.clone(),
            masks,
            overdue,
        ));
    }
    let shape = hidden::shape(&card.markdown);
    match key {
        "fwd" => {
            let sides = sides::split(&card.markdown);
            (sides.len() >= 2).then(|| Entry::sides(card.id, sides, overdue))
        }
        "rev" => {
            let sides = sides::split(&card.markdown);
            (sides.len() >= 2).then(|| Entry::reverse(card.id, sides, overdue))
        }
        other if hidden::group_of(other).is_some() => {
            let hidden = hidden::masked(&card.markdown, &shape, other);
            let shown = hidden::shown(&card.markdown, &shape);
            (hidden != shown).then(|| Entry::masked(card.id, other, hidden, shown, overdue))
        }
        _ => None,
    }
}

/// The entries a session walks, from the prompts the store served and the cards it holds. A prompt
/// whose card was edited away is left out rather than asked with nothing on it.
pub fn entries_for(cards: &[CardSummary], prompts: &[(Id, String, i64)]) -> Vec<Entry> {
    prompts
        .iter()
        .filter_map(|(card, key, overdue)| {
            let card = cards.iter().find(|row| row.id == *card)?;
            entry_for(card, key, *overdue)
        })
        .collect()
}

#[cfg(test)]
mod entry_tests {
    use super::*;
    use crate::model::MaskRow;

    #[test]
    fn builds_entries_for_sides_reverse_cloze_and_masks() {
        let sided = CardSummary {
            id: 1,
            markdown: "front\n---\nback".into(),
            ..CardSummary::default()
        };
        let fwd = entry_for(&sided, "fwd", 2).expect("fwd");
        assert_eq!(fwd.prompt, "fwd");
        assert_eq!(fwd.faces[0].text(), Some(("front\n", "front\n")));
        let rev = entry_for(&sided, "rev", 2).expect("rev");
        assert_eq!(rev.prompt, "rev");
        assert_eq!(rev.faces[0].text(), Some(("back", "back")));

        let cloze = CardSummary {
            id: 2,
            markdown: "The {{answer}} hides.".into(),
            ..CardSummary::default()
        };
        assert!(entry_for(&cloze, "cloze:0", 0).is_some());
        assert!(entry_for(&cloze, "cloze:9", 0).is_none());

        let diagram = CardSummary {
            id: 3,
            image: Some("cell.png".into()),
            masks: vec![MaskRow {
                group: 1,
                rect: MaskRect::new(0.1, 0.1, 0.2, 0.2),
            }],
            ..CardSummary::default()
        };
        assert!(entry_for(&diagram, "mask:1", 1).is_some());
        assert!(entry_for(&diagram, "mask:2", 1).is_none());

        let entries = entries_for(
            &[sided, cloze, diagram],
            &[
                (1, "fwd".into(), 0),
                (99, "fwd".into(), 0),
                (3, "mask:1".into(), 1),
            ],
        );
        assert_eq!(entries.len(), 2);
    }
}

/// What a session walks: the two queues the scheduler serves, or a cram run over a filter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Cards nothing has scheduled yet, capped by the day's allowance.
    New,
    /// Prompts whose due date has come.
    Due,
    /// Exactly the cards a filter selects, whatever the schedule says.
    Cram,
}

/// One prompt of one card: what a session answers, since a card can hold several.
type Prompt = (Id, String);

impl Entry {
    fn key(&self) -> Prompt {
        (self.card, self.prompt.clone())
    }
}

/// The cards this session is walking, in order.
#[derive(Debug)]
#[non_exhaustive]
pub struct Session {
    pub queue: Queue,
    mode: Mode,
    entries: Vec<Entry>,
    at: usize,
    revealed: usize,
    /// Grades recorded in this session, in the order they were answered.
    pub grades: Vec<Grade>,
    /// The grade sent to the store and not yet acknowledged, with the prompt it was given for: the
    /// answer names a prompt, never a position, because the user may move on before it lands.
    awaiting: Option<(u64, Prompt, Grade)>,
    /// A grade chosen but not yet sent.
    pending: Option<(Prompt, Grade)>,
    /// Cards whose relearning step is still in the future, and when it lands.
    waiting: Vec<(i64, Entry)>,
    /// Whether the current card is being revisited read-only.
    pub read_only: bool,
    /// Prompts answered in this session. They stay in the queue so `backward` can show them again,
    /// and are skipped everywhere the queue is read forward.
    answered: HashSet<Prompt>,
    /// Prompts that joined this session with a schedule already behind them. They keep the
    /// relearning route after a lapse even when the session itself was opened on the new-card queue.
    scheduled: HashSet<Prompt>,
    /// How long a forgotten due prompt waits before it is asked again.
    learning_step_ms: i64,
}

impl Session {
    /// A session with nothing queued, as the app holds before one starts.
    pub fn empty() -> Session {
        Session::new(Queue::Due, Vec::new())
    }

    pub fn new(queue: Queue, entries: Vec<Entry>) -> Session {
        let mode = match queue {
            Queue::New => Mode::New,
            Queue::Due => Mode::Due,
        };
        Session::with_mode(mode, queue, entries)
    }

    /// A session over exactly these cards: no due date, no allowance, and no store to tell.
    ///
    /// A cram run answers the filter the library is showing, so the queue it would otherwise have
    /// walked is kept on the due one. Nothing reads it: a cram grade is settled inside the
    /// session, never acknowledged by the store.
    pub fn cram(entries: Vec<Entry>) -> Session {
        Session::with_mode(Mode::Cram, Queue::Due, entries)
    }

    fn with_mode(mode: Mode, queue: Queue, entries: Vec<Entry>) -> Session {
        let mut session = Session {
            queue,
            mode,
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
            learning_step_ms: LEARNING_STEP_MS,
        };
        session.show_first_side();
        session
    }

    /// Wait `ms` instead of the default step before a forgotten due prompt comes back.
    pub fn with_learning_step(mut self, ms: i64) -> Session {
        self.learning_step_ms = ms;
        self
    }

    /// Which queue this session walks, or that it walks a filter instead.
    pub fn mode(&self) -> Mode {
        self.mode
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

    /// The next text face, which revealing would show. A picture face has no text to hand back, so
    /// this answers `None` for one; ask [`Session::asking`] for the face itself.
    pub fn next_side(&self) -> Option<&str> {
        let entry = self.current()?;
        entry
            .faces
            .get(self.revealed)
            .and_then(Face::text)
            .map(|(_, shown)| shown)
    }

    /// The face being asked right now, whether it is made of text or of a picture.
    pub fn asking(&self) -> Option<&Face> {
        self.current()?.faces.get(self.revealed)
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
            // A picture face is drawn by the review screen, which needs the rectangles; only text
            // becomes a line here.
            let Some((hidden, shown)) = face.text() else {
                continue;
            };
            if index < self.revealed {
                lines.push((shown, true));
            } else if index == self.revealed && face.unmasks() {
                lines.push((hidden, false));
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

    /// Choose a grade for the prompt on screen. The caller sends it under that card and prompt
    /// key, and reports the id back.
    pub fn grade(&mut self, grade: Grade) -> Option<(Id, String, Grade)> {
        if !self.gradeable() || self.pending.is_some() {
            return None;
        }
        let (card, prompt) = self.current()?.key();
        self.pending = Some(((card, prompt.clone()), grade));
        Some((card, prompt, grade))
    }

    /// The store has taken the grade under `id`.
    pub fn sent(&mut self, id: u64) {
        if let Some((prompt, grade)) = self.pending.take() {
            self.awaiting = Some((id, prompt, grade));
        }
    }

    /// The grade never reached the store, so no answer is coming: drop it and leave the card
    /// answerable instead of waiting for an ack that will not arrive.
    pub fn unsent(&mut self) {
        self.pending = None;
    }

    /// The store did not take the grade: the card stays as it was, so it can be answered again.
    pub fn refuse(&mut self, id: u64) {
        if let Some((awaiting, _, _)) = &self.awaiting {
            if *awaiting != id {
                return;
            }
            self.awaiting = None;
        }
        self.pending = None;
    }

    /// The grade has been taken inside the session, for a run that writes nothing: a cram answer
    /// has no store to wait for, so it is settled here and now. Answers whether there was one.
    pub fn answer_here(&mut self) -> bool {
        if self.mode != Mode::Cram {
            return false;
        }
        let Some((prompt, grade)) = self.pending.take() else {
            return false;
        };
        self.grades.push(grade);
        self.cram_answer(prompt, grade);
        true
    }

    /// The store has answered `id`; the session can move on.
    pub fn acknowledge(&mut self, id: u64, now_ms: i64) -> bool {
        let Some((_, prompt, grade)) = self.awaiting.take_if(|(awaiting, _, _)| *awaiting == id)
        else {
            return false;
        };
        self.grades.push(grade);
        if self.mode == Mode::Cram {
            // The mode's rule, not the transport's: a cram session never sends a grade, but a
            // grade that did come back must still move the card the cram way.
            self.cram_answer(prompt, grade);
            return true;
        }
        // A card that arrived with a schedule behind it keeps that schedule: it waits out its
        // relearning step instead of being handed straight back as if it were new.
        let queue = if self.scheduled.contains(&prompt) {
            Queue::Due
        } else {
            self.queue
        };
        match (queue, grade) {
            // A forgotten card is not answered: one leaves the queue for later, the other comes
            // straight back, and neither counts as done.
            (Queue::New, Grade::Forgot) => self.requeue(&prompt),
            (Queue::Due, Grade::Forgot) => {
                if let Some(entry) = self.take(&prompt) {
                    self.waiting.push((now_ms + self.learning_step_ms, entry));
                }
            }
            _ => self.answer(prompt),
        }
        true
    }

    /// A cram answer moves the card within the session and nowhere else: remembered is done,
    /// forgot comes round again before the run can finish.
    fn cram_answer(&mut self, prompt: Prompt, grade: Grade) {
        match grade {
            Grade::Forgot => self.requeue(&prompt),
            Grade::Remembered => self.answer(prompt),
        }
    }

    /// Put a prompt back at the end of the queue, so this run asks it again.
    fn requeue(&mut self, prompt: &Prompt) {
        if let Some(entry) = self.take(prompt) {
            let empty = self.current().is_none();
            self.entries.push(entry);
            if empty {
                self.show_first_side();
            }
        }
    }

    /// Record that `prompt` is answered, and step over it if the cursor is on it.
    fn answer(&mut self, prompt: Prompt) {
        let on_screen = self.current().is_some_and(|entry| entry.key() == prompt);
        self.answered.insert(prompt);
        if on_screen {
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
            .is_some_and(|entry| self.answered.contains(&entry.key()))
        {
            self.at += 1;
        }
    }

    /// Lift a prompt out of the queue wherever it sits, leaving the cursor on the one the user sees.
    fn take(&mut self, prompt: &Prompt) -> Option<Entry> {
        let index = self
            .entries
            .iter()
            .position(|entry| entry.key() == *prompt)?;
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
            .filter(|entry| !self.answered.contains(&entry.key()))
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
            if self.answered.contains(&self.entries[index].key()) {
                index += 1;
                continue;
            }
            let overdue = self.entries[index].overdue;
            let mut end = index;
            while end < self.entries.len()
                && !self.answered.contains(&self.entries[end].key())
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
        self.scheduled.insert(entry.key());
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
