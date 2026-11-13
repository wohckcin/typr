//! The synchronous half of the store: one `Connection`, one command at a time.
//!
//! Every method here runs on the store task's thread and is exercised by tests directly, which is
//! what keeps the threaded part (`task.rs`) down to plumbing.

use std::collections::HashMap;
use std::path::Path;

use rusqlite::{Connection, OpenFlags, OptionalExtension, params};

use crate::schedule::{FixedMultiplier, PromptState, Scheduler};

use super::{
    CardSummary, Command, Deck, DuePrompt, Grade, Id, NewPrompt, Outcome, Phase, ReviewRow,
    Settings, Snapshot, Stats, StoreError, schema,
};
use unicode_normalization::UnicodeNormalization;

/// A record as a later build would have written it, for the extra keys a future version schedules.
#[cfg(test)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct Record {
    pub(crate) card: Id,
    pub(crate) key: &'static str,
    pub(crate) phase: Phase,
    pub(crate) grade: Grade,
    pub(crate) at: i64,
    pub(crate) interval_ms: i64,
    pub(crate) due_at: Option<i64>,
}

/// A prompt as its records describe it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Folded {
    pub phase: Phase,
    pub interval_ms: i64,
    pub due_at: Option<i64>,
    pub reps: i64,
    pub lapses: i64,
}

/// A database file, owned by exactly one thread.
#[derive(Debug)]
pub struct Db {
    conn: Connection,
}

impl Db {
    /// Open (creating if needed) the database at `path` and bring its schema up to date.
    ///
    /// A path that is not a database is an error: the file is read for its header before anything
    /// is written, so an unrelated file is reported rather than overwritten.
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        let mut db = Self { conn };
        db.conn.pragma_update(None, "journal_mode", "WAL")?;
        db.conn.pragma_update(None, "foreign_keys", true)?;
        schema::migrate(&mut db.conn)?;
        Ok(db)
    }

    /// Apply one command and report what it did. `now_ms` is the caller's clock, never this
    /// module's.
    ///
    /// The whole command is one transaction: a command that fails halfway leaves nothing behind,
    /// and a command that writes several rows pays one fsync.
    pub fn apply(&self, command: &Command, now_ms: i64) -> Result<Outcome, StoreError> {
        let tx = self.conn.unchecked_transaction()?;
        let outcome = self.dispatch(command, now_ms)?;
        tx.commit()?;
        Ok(outcome)
    }

    /// The body of one command, run inside the transaction [`Db::apply`] opens.
    fn dispatch(&self, command: &Command, now_ms: i64) -> Result<Outcome, StoreError> {
        match command {
            Command::CreateDeck { parent, name } => {
                self.conn.execute(
                    "INSERT INTO deck (parent_id, name) VALUES (?1, ?2)",
                    params![parent, name],
                )?;
                Ok(Outcome::Deck(self.conn.last_insert_rowid()))
            }
            Command::RenameDeck { id, name } => {
                let changed = self
                    .conn
                    .execute("UPDATE deck SET name = ?2 WHERE id = ?1", params![id, name])?;
                Ok(Outcome::Changed(changed))
            }
            Command::ArchiveDeck { id, archived } => {
                let changed = self.conn.execute(
                    "UPDATE deck SET archived = ?2 WHERE id = ?1",
                    params![id, i64::from(*archived)],
                )?;
                Ok(Outcome::Changed(changed))
            }
            Command::MoveDeck { id, parent } => self.move_deck(*id, *parent),
            Command::CreateCard { deck, markdown } => {
                let sides = sides_of(markdown);
                self.conn.execute(
                    "INSERT INTO card (deck_id, markdown, sides, search_text, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                    params![deck, markdown, sides as i64, folded(markdown), now_ms],
                )?;
                let id = self.conn.last_insert_rowid();
                self.sync_prompt(id, sides)?;
                self.conn.execute(
                    "DELETE FROM editor_draft WHERE card_id IN (0, ?1)",
                    params![id],
                )?;
                Ok(Outcome::Card(id))
            }
            Command::SaveCard { id, markdown } => {
                let sides = sides_of(markdown);
                let changed = self.conn.execute(
                    "UPDATE card SET markdown = ?2, sides = ?3, search_text = ?4, updated_at = ?5 WHERE id = ?1",
                    params![id, markdown, sides as i64, folded(markdown), now_ms],
                )?;
                self.sync_prompt(*id, sides)?;
                self.conn
                    .execute("DELETE FROM editor_draft WHERE card_id = ?1", params![id])?;
                Ok(Outcome::Changed(changed))
            }
            Command::MoveCard { id, deck } => {
                let changed = self.conn.execute(
                    "UPDATE card SET deck_id = ?2, updated_at = ?3 WHERE id = ?1",
                    params![id, deck, now_ms],
                )?;
                Ok(Outcome::Changed(changed))
            }
            Command::SetTags { card, tags } => self.set_tags(*card, tags),
            Command::ArchiveCard { id, archived } => {
                let changed = self.conn.execute(
                    "UPDATE card SET archived = ?2, updated_at = ?3 WHERE id = ?1",
                    params![id, i64::from(*archived), now_ms],
                )?;
                Ok(Outcome::Changed(changed))
            }
            Command::EmptyTrash => {
                let removed = self
                    .conn
                    .execute("DELETE FROM card WHERE trashed_at IS NOT NULL", [])?;
                Ok(Outcome::Changed(removed))
            }
            Command::TrashCard { id } => {
                let changed = self.conn.execute(
                    "UPDATE card SET trashed_at = ?2 WHERE id = ?1",
                    params![id, now_ms],
                )?;
                Ok(Outcome::Changed(changed))
            }
            Command::RestoreCard { id } => {
                let changed = self.conn.execute(
                    "UPDATE card SET trashed_at = NULL WHERE id = ?1",
                    params![id],
                )?;
                Ok(Outcome::Changed(changed))
            }
            Command::SetDraft { card, text } => match text {
                Some(text) => {
                    self.conn.execute(
                        "INSERT INTO editor_draft (card_id, text, updated_at) VALUES (?1, ?2, ?3)
                         ON CONFLICT (card_id) DO UPDATE SET text = excluded.text, updated_at = excluded.updated_at",
                        params![card, text, now_ms],
                    )?;
                    Ok(Outcome::Ok)
                }
                None => {
                    self.conn
                        .execute("DELETE FROM editor_draft WHERE card_id = ?1", params![card])?;
                    Ok(Outcome::Ok)
                }
            },
            Command::Draft { card } => Ok(Outcome::Draft(self.draft(*card)?)),
            Command::SetSetting { key, value } => {
                if key == "new_per_day" && value.parse::<u32>().is_err() {
                    return Err(StoreError::Setting {
                        key: key.clone(),
                        value: value.clone(),
                    });
                }
                self.conn.execute(
                    "INSERT INTO settings (key, value) VALUES (?1, ?2)
                     ON CONFLICT (key) DO UPDATE SET value = excluded.value",
                    params![key, value],
                )?;
                Ok(Outcome::Ok)
            }
            Command::Grade {
                card,
                key,
                grade,
                duration_ms,
            } => self.grade(*card, key, *grade, *duration_ms, now_ms),
            Command::ResetHistory { card, key } => self.reset_history(*card, key),
            Command::DuePrompts {
                now_ms,
                offset_ms,
                seed,
            } => Ok(Outcome::Due(self.due_prompts(*now_ms, *offset_ms, *seed)?)),
            Command::NewPrompts {
                now_ms,
                offset_ms,
                seed,
            } => Ok(Outcome::New(self.new_prompts(*now_ms, *offset_ms, *seed)?)),
            Command::Search { query, deck } => Ok(Outcome::Search(self.search(query, *deck)?)),
            Command::History { card } => Ok(Outcome::History {
                rows: self.history(*card)?,
                reviews: self.review_count(*card)?,
            }),
            Command::Stats { from_ms, to_ms } => Ok(Outcome::Stats(self.stats(*from_ms, *to_ms)?)),
        }
    }

    /// Everything a view may read without a round trip.
    pub fn snapshot(&self) -> Result<Snapshot, StoreError> {
        let decks = {
            let mut stmt = self.conn.prepare(
                "SELECT id, parent_id, name, archived FROM deck ORDER BY name COLLATE NOCASE, id",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok(Deck {
                    id: row.get(0)?,
                    parent: row.get(1)?,
                    name: row.get(2)?,
                    archived: row.get::<_, i64>(3)? != 0,
                })
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };
        let cards = {
            let mut stmt = self.conn.prepare(
                "SELECT id, deck_id, sides, archived, trashed_at, updated_at, markdown, search_text
                 FROM card ORDER BY updated_at DESC, id",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok(CardSummary {
                    id: row.get(0)?,
                    deck: row.get(1)?,
                    sides: row.get::<_, i64>(2)?.max(0) as u32,
                    tags: Vec::new(),
                    archived: row.get::<_, i64>(3)? != 0,
                    trashed_at: row.get(4)?,
                    updated_at: row.get(5)?,
                    preview: preview(&row.get::<_, String>(6)?),
                    markdown: row.get(6)?,
                    search_text: row.get(7)?,
                })
            })?;
            let mut cards = rows.collect::<Result<Vec<_>, _>>()?;

            let mut stmt = self.conn.prepare(
                "SELECT ct.card_id, t.name FROM card_tag ct JOIN tag t ON t.id = ct.tag_id
                 ORDER BY t.name COLLATE NOCASE",
            )?;
            let pairs = stmt.query_map([], |row| {
                Ok((row.get::<_, Id>(0)?, row.get::<_, String>(1)?))
            })?;
            let mut by_card: HashMap<Id, Vec<String>> = HashMap::new();
            for pair in pairs {
                let (card, name) = pair?;
                by_card.entry(card).or_default().push(name);
            }
            for summary in &mut cards {
                if let Some(names) = by_card.remove(&summary.id) {
                    summary.tags = names;
                }
            }
            cards
        };
        Ok(Snapshot {
            decks,
            cards,
            settings: self.settings()?,
            revision: 0,
        })
    }

    pub fn settings(&self) -> Result<Settings, StoreError> {
        let limit: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'new_per_day'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        let new_per_day = match limit {
            None => Settings::DEFAULT_NEW_PER_DAY,
            Some(value) => value.parse().map_err(|_| StoreError::Setting {
                key: "new_per_day".to_string(),
                value,
            })?,
        };
        Ok(Settings { new_per_day })
    }

    /// The text as it was left, if it was left unsaved.
    pub fn draft(&self, card: Id) -> Result<Option<String>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT text FROM editor_draft WHERE card_id = ?1")?;
        let mut rows = stmt.query(params![card])?;
        match rows.next()? {
            Some(row) => Ok(Some(row.get(0)?)),
            None => Ok(None),
        }
    }

    /// The prompt keys a card holds. A card that became a note keeps them: its schedule, due date
    /// and history stay intact and come back when it has two sides again.
    pub fn prompt_keys(&self, card: Id) -> Result<Vec<String>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT key FROM prompt WHERE card_id = ?1 ORDER BY key")?;
        let rows = stmt.query_map(params![card], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Give a two-sided card its `fwd` prompt. A prompt that exists is left exactly as it was, so
    /// editing text never resets a schedule; a one-sided card keeps the prompt it has and simply
    /// stops queueing (a note is not studyable).
    fn sync_prompt(&self, card: Id, sides: usize) -> Result<(), StoreError> {
        if sides >= 2 {
            self.conn.execute(
                "INSERT INTO prompt (card_id, key, phase, due_at, interval_ms, reps, lapses)
                 VALUES (?1, 'fwd', 'learn', NULL, 0, 0, 0)
                 ON CONFLICT (card_id, key) DO NOTHING",
                params![card],
            )?;
        }
        Ok(())
    }

    /// Replace a card's tags. Matching is case-insensitive and the first casing ever written wins,
    /// so `Grammar` stays `Grammar` even when a later card types `grammar`.
    pub fn set_tags(&self, card: Id, tags: &[String]) -> Result<Outcome, StoreError> {
        self.conn
            .execute("DELETE FROM card_tag WHERE card_id = ?1", params![card])?;
        for tag in tags {
            let name = tag.trim();
            if name.is_empty() {
                continue;
            }
            self.conn.execute(
                "INSERT INTO tag (name) VALUES (?1) ON CONFLICT DO NOTHING",
                params![name],
            )?;
            let id: Id = self.conn.query_row(
                "SELECT id FROM tag WHERE name = ?1",
                params![name],
                |row| row.get(0),
            )?;
            self.conn.execute(
                "INSERT OR IGNORE INTO card_tag (card_id, tag_id) VALUES (?1, ?2)",
                params![card, id],
            )?;
        }
        Ok(Outcome::Ok)
    }

    /// The tags currently in use, each in its first-written casing.
    pub fn tag_names(&self) -> Result<Vec<String>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT name FROM tag ORDER BY name COLLATE NOCASE")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Cards carrying a tag, matched without regard to case.
    pub fn cards_with_tag(&self, tag: &str) -> Result<Vec<Id>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT ct.card_id FROM card_tag ct JOIN tag t ON t.id = ct.tag_id
             WHERE t.name = ?1 ORDER BY ct.card_id",
        )?;
        let rows = stmt.query_map(params![tag.trim()], |row| row.get::<_, Id>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Move a deck under another, refusing a move that would put it inside its own subtree.
    pub fn move_deck(&self, id: Id, parent: Option<Id>) -> Result<Outcome, StoreError> {
        if let Some(parent) = parent {
            let mut cursor = Some(parent);
            while let Some(deck) = cursor {
                if deck == id {
                    return Err(StoreError::Cycle { deck: id });
                }
                cursor = self
                    .conn
                    .query_row(
                        "SELECT parent_id FROM deck WHERE id = ?1",
                        params![deck],
                        |row| row.get::<_, Option<Id>>(0),
                    )
                    .optional()?
                    .flatten();
            }
        }
        let changed = self.conn.execute(
            "UPDATE deck SET parent_id = ?2 WHERE id = ?1",
            params![id, parent],
        )?;
        Ok(Outcome::Changed(changed))
    }

    /// Append one grade and leave the prompt agreeing with it.
    ///
    /// The recorded phase is where the grade *put* the prompt: a first success graduates the card
    /// and is recorded as `review`, a failure in the review phase is recorded as `relearn`.
    pub fn grade(
        &self,
        card: Id,
        key: &str,
        grade: Grade,
        duration_ms: Option<i64>,
        now_ms: i64,
    ) -> Result<Outcome, StoreError> {
        let state = self.prompt_state(card, key)?;
        let outcome = FixedMultiplier.grade(&state, grade, now_ms);
        self.conn.execute(
            "INSERT INTO review_log (card_id, key, phase, at, grade, interval_ms, due_at, duration_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                card,
                key,
                outcome.phase.as_str(),
                now_ms,
                grade.as_str(),
                outcome.interval_ms,
                outcome.due_at,
                duration_ms
            ],
        )?;
        self.refresh_prompt(card, key)?;
        Ok(Outcome::Graded {
            phase: outcome.phase,
            due_at: outcome.due_at,
        })
    }

    /// Drop a prompt's whole history and put it back to new, which is the only place records are
    /// ever removed: individually they are append-only.
    pub fn reset_history(&self, card: Id, key: &str) -> Result<Outcome, StoreError> {
        let removed = self.conn.execute(
            "DELETE FROM review_log WHERE card_id = ?1 AND key = ?2",
            params![card, key],
        )?;
        self.conn.execute(
            "UPDATE prompt SET phase = 'learn', due_at = NULL, interval_ms = 0, reps = 0, lapses = 0
             WHERE card_id = ?1 AND key = ?2",
            params![card, key],
        )?;
        Ok(Outcome::Changed(removed))
    }

    /// Every record in the log, for tests that prove an emptied card takes its history with it.
    #[cfg(test)]
    pub fn record_total(&self) -> Result<i64, StoreError> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM review_log", [], |row| row.get(0))?)
    }

    /// How many real reviews a card has had: learning cycles are not reviews.
    pub fn review_count(&self, card: Id) -> Result<u32, StoreError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM review_log WHERE card_id = ?1 AND phase <> 'learn'",
            params![card],
            |row| row.get(0),
        )?;
        Ok(count as u32)
    }

    fn prompt_state(&self, card: Id, key: &str) -> Result<PromptState, StoreError> {
        self.conn
            .query_row(
                "SELECT phase, interval_ms FROM prompt WHERE card_id = ?1 AND key = ?2",
                params![card, key],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
            )
            .map(|(phase, interval_ms)| PromptState {
                phase: Phase::parse(&phase).unwrap_or(Phase::Review),
                interval_ms,
            })
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => StoreError::NoPrompt {
                    card,
                    key: key.to_string(),
                },
                other => StoreError::Sqlite(other),
            })
    }

    /// Recompute a prompt from its records, so the prompt row never says more than the history.
    fn refresh_prompt(&self, card: Id, key: &str) -> Result<(), StoreError> {
        let Some(folded) = self.fold_records(card, key)? else {
            return Ok(());
        };
        self.conn.execute(
            "UPDATE prompt SET phase = ?3, due_at = ?4, interval_ms = ?5, reps = ?6, lapses = ?7
             WHERE card_id = ?1 AND key = ?2",
            params![
                card,
                key,
                folded.phase.as_str(),
                folded.due_at,
                folded.interval_ms,
                folded.reps,
                folded.lapses
            ],
        )?;
        Ok(())
    }

    /// Append a record for a key this build does not schedule yet, standing in for the rows a
    /// later version writes for reverse, cloze and mask prompts.
    #[cfg(test)]
    pub(crate) fn append_record_for_test(&mut self, record: Record) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO review_log (card_id, key, phase, at, grade, interval_ms, due_at, duration_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL)",
            params![
                record.card,
                record.key,
                record.phase.as_str(),
                record.at,
                record.grade.as_str(),
                record.interval_ms,
                record.due_at
            ],
        )?;
        Ok(())
    }

    /// Prompts that are due now, most overdue study day first and shuffled within a day.
    ///
    /// The bucket is whole study days, not seconds: two prompts that became due on the same day are
    /// equally urgent, and a timestamp is not an urgency, so the order inside a day is the shuffle's.
    pub fn due_prompts(
        &self,
        now_ms: i64,
        offset_ms: i64,
        seed: u64,
    ) -> Result<Vec<DuePrompt>, StoreError> {
        let mut stmt = self.conn.prepare(
            "WITH RECURSIVE archived_deck(id) AS (
                 SELECT id FROM deck WHERE archived = 1
                 UNION
                 SELECT d.id FROM deck d JOIN archived_deck a ON d.parent_id = a.id
             )
             SELECT p.card_id, p.key, p.due_at
             FROM prompt p JOIN card c ON c.id = p.card_id
             WHERE p.due_at IS NOT NULL
               AND p.due_at <= ?1
               AND c.trashed_at IS NULL
               AND c.archived = 0
               AND c.sides >= 2
               AND c.deck_id NOT IN (SELECT id FROM archived_deck)
             ORDER BY p.due_at, p.card_id",
        )?;
        let mut due = stmt
            .query_map(params![now_ms], |row| {
                Ok(DuePrompt {
                    card: row.get(0)?,
                    key: row.get(1)?,
                    due_at: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let today = crate::day::study_day(now_ms, offset_ms);
        // Stable sort by overdue study days, so equal days keep the query's order until shuffled.
        due.sort_by_key(|prompt| -(today - crate::day::study_day(prompt.due_at, offset_ms)));

        let mut shuffle = crate::shuffle::Shuffle::new(seed);
        let mut ordered = Vec::with_capacity(due.len());
        let mut rest = due.as_slice();
        while let Some(first) = rest.first() {
            let overdue = today - crate::day::study_day(first.due_at, offset_ms);
            let same = rest
                .iter()
                .position(|prompt| {
                    today - crate::day::study_day(prompt.due_at, offset_ms) != overdue
                })
                .unwrap_or(rest.len());
            let mut group: Vec<DuePrompt> = rest[..same].to_vec();
            shuffle.reorder(&mut group);
            ordered.extend(group);
            rest = &rest[same..];
        }
        Ok(ordered)
    }

    /// Cards that graduated on one study day, counted across every deck.
    ///
    /// A grade before the card ever left the new queue does not count: the cap is on cards
    /// *learned*, not on how many times a new card was answered today.
    pub fn learned_on(&self, day: i64, offset_ms: i64) -> Result<u32, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT MIN(at) FROM review_log WHERE phase <> 'learn' GROUP BY card_id")?;
        let firsts = stmt
            .query_map([], |row| row.get::<_, i64>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        let count = firsts
            .into_iter()
            .filter(|at| crate::day::study_day(*at, offset_ms) == day)
            .count();
        Ok(count as u32)
    }

    /// The new-card queue: studyable cards whose prompt has never been graded, capped by the
    /// settings' daily allowance minus what was already learned today.
    pub fn new_prompts(
        &self,
        now_ms: i64,
        offset_ms: i64,
        seed: u64,
    ) -> Result<Vec<NewPrompt>, StoreError> {
        let mut stmt = self.conn.prepare(
            "WITH RECURSIVE archived_deck(id) AS (
                 SELECT id FROM deck WHERE archived = 1
                 UNION
                 SELECT d.id FROM deck d JOIN archived_deck a ON d.parent_id = a.id
             )
             SELECT p.card_id, p.key
             FROM prompt p JOIN card c ON c.id = p.card_id
             WHERE c.trashed_at IS NULL
               AND c.archived = 0
               AND c.sides >= 2
               AND c.deck_id NOT IN (SELECT id FROM archived_deck)
               AND NOT EXISTS (
                   SELECT 1 FROM review_log r WHERE r.card_id = p.card_id AND r.key = p.key
               )
             ORDER BY p.card_id, p.key",
        )?;
        let waiting = stmt
            .query_map([], |row| {
                Ok(NewPrompt {
                    card: row.get(0)?,
                    key: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let allowance = self.settings()?.new_per_day as usize;
        let today = crate::day::study_day(now_ms, offset_ms);
        let learned = self.learned_on(today, offset_ms)? as usize;
        let room = allowance.saturating_sub(learned);

        let mut shuffle = crate::shuffle::Shuffle::new(seed);
        let mut waiting = waiting;
        shuffle.reorder(&mut waiting);
        waiting.truncate(room);
        Ok(waiting)
    }

    /// Grades inside one study day, so a summary and a cap agree about which day it is.
    pub fn stats_for_day(&self, day: i64, offset_ms: i64) -> Result<Stats, StoreError> {
        let (from, to) = crate::day::study_day_bounds(day, offset_ms);
        self.stats(from, to)
    }

    /// One prompt row as stored.
    pub fn prompt(&self, card: Id, key: &str) -> Result<Option<Folded>, StoreError> {
        self.conn
            .query_row(
                "SELECT phase, interval_ms, due_at, reps, lapses
                 FROM prompt WHERE card_id = ?1 AND key = ?2",
                params![card, key],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, Option<i64>>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, i64>(4)?,
                    ))
                },
            )
            .optional()
            .map_err(StoreError::from)
            .map(|row| {
                row.map(|(phase, interval_ms, due_at, reps, lapses)| Folded {
                    phase: Phase::parse(&phase).unwrap_or(Phase::Review),
                    interval_ms,
                    due_at,
                    reps,
                    lapses,
                })
            })
    }

    /// Rewrite every prompt from its records, which is what makes the prompt a cache rather than a
    /// second source of truth. Returns how many prompts were rewritten.
    pub fn rebuild_prompts(&self) -> Result<usize, StoreError> {
        let pairs = {
            let mut stmt = self
                .conn
                .prepare("SELECT DISTINCT card_id, key FROM review_log ORDER BY card_id, key")?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, Id>(0)?, row.get::<_, String>(1)?))
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };
        for (card, key) in &pairs {
            self.conn.execute(
                "INSERT INTO prompt (card_id, key, phase, due_at, interval_ms, reps, lapses)
                 VALUES (?1, ?2, 'learn', NULL, 0, 0, 0)
                 ON CONFLICT (card_id, key) DO NOTHING",
                params![card, key],
            )?;
            self.refresh_prompt(*card, key)?;
        }
        Ok(pairs.len())
    }

    /// What a prompt's records say about it: the last record's decision and the counts above it.
    ///
    /// This is the fold every future algorithm agrees with — recomputing a prompt from history
    /// must give back the row that history produced.
    pub fn fold_records(&self, card: Id, key: &str) -> Result<Option<Folded>, StoreError> {
        let rows = self.history(card)?;
        let counted: Vec<&ReviewRow> = rows.iter().filter(|row| row.key == key).collect();
        let Some(last) = counted.last() else {
            return Ok(None);
        };
        Ok(Some(Folded {
            phase: last.phase,
            interval_ms: last.interval_ms,
            due_at: last.due_at,
            reps: counted.len() as i64,
            lapses: counted
                .iter()
                .filter(|row| row.grade == Grade::Forgot && row.phase != Phase::Learn)
                .count() as i64,
        }))
    }

    /// Grades recorded between two instants, counting real reviews only.
    pub fn stats(&self, from_ms: i64, to_ms: i64) -> Result<Stats, StoreError> {
        self.conn
            .query_row(
                "SELECT COUNT(*), COALESCE(SUM(grade = 'forgot'), 0) FROM review_log
             WHERE phase <> 'learn' AND at >= ?1 AND at < ?2",
                params![from_ms, to_ms],
                |row| {
                    Ok(Stats {
                        reviews: row.get::<_, i64>(0)?.max(0) as u32,
                        lapses: row.get::<_, i64>(1)?.max(0) as u32,
                    })
                },
            )
            .map_err(Into::into)
    }

    /// Every grade a card ever got, oldest first, learning cycles included.
    pub fn history(&self, card: Id) -> Result<Vec<ReviewRow>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT card_id, key, phase, at, grade, interval_ms, due_at, duration_ms
             FROM review_log WHERE card_id = ?1 ORDER BY at, rowid",
        )?;
        let rows = stmt.query_map(params![card], |row| {
            Ok((
                row.get::<_, Id>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, Option<i64>>(6)?,
                row.get::<_, Option<i64>>(7)?,
            ))
        })?;
        rows.map(|row| {
            let (card, key, phase, at, grade, interval_ms, due_at, duration_ms) = row?;
            Ok(ReviewRow {
                card,
                key,
                phase: Phase::parse(&phase).ok_or_else(|| StoreError::Text {
                    column: "review_log.phase",
                    value: phase.clone(),
                })?,
                at,
                grade: Grade::parse(&grade).ok_or_else(|| StoreError::Text {
                    column: "review_log.grade",
                    value: grade.clone(),
                })?,
                interval_ms,
                due_at,
                duration_ms,
            })
        })
        .collect()
    }
    /// Cards whose folded text contains the folded query, restricted to `deck`'s subtree when given.
    /// Trashed cards never match.
    pub fn search(&self, query: &str, deck: Option<Id>) -> Result<Vec<Id>, StoreError> {
        let needle = escape_like(&folded(query));
        if needle.is_empty() {
            return Ok(Vec::new());
        }
        let mut stmt = self.conn.prepare(
            "WITH RECURSIVE subtree(id) AS (
                 SELECT ?2
                 UNION
                 SELECT d.id FROM deck d JOIN subtree s ON d.parent_id = s.id
             )
             SELECT c.id FROM card c
             WHERE c.trashed_at IS NULL
               AND c.search_text LIKE '%' || ?1 || '%' ESCAPE '\\'
               AND (?2 IS NULL OR c.deck_id IN (SELECT id FROM subtree))
             ORDER BY c.id",
        )?;
        let ids = stmt
            .query_map(params![needle, deck], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ids)
    }
}

/// The first line that has words in it, cut to fit a row.
fn preview(markdown: &str) -> String {
    let line = markdown
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("");
    let mut cut: String = line.chars().take(80).collect();
    if line.chars().count() > 80 {
        cut.push('…');
    }
    cut
}

fn sides_of(markdown: &str) -> usize {
    crate::sides::split(markdown).len()
}

/// A LIKE pattern's metacharacters, escaped so a typed `%` or `_` matches itself.
fn escape_like(needle: &str) -> String {
    let mut escaped = String::with_capacity(needle.len());
    for character in needle.chars() {
        if matches!(character, '\\' | '%' | '_') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

/// What search compares against: the text in the case a `LIKE` can fold.
pub fn folded(text: &str) -> String {
    text.nfd()
        .filter(|character| !is_combining_mark(*character))
        .flat_map(char::to_lowercase)
        .collect()
}

/// Diacritics live in these blocks after decomposition; dropping them is what makes `cancion`
/// find `Canción`.
fn is_combining_mark(character: char) -> bool {
    matches!(
        character as u32,
        0x0300..=0x036F | 0x1AB0..=0x1AFF | 0x1DC0..=0x1DFF | 0x20D0..=0x20FF | 0xFE20..=0xFE2F
    )
}
