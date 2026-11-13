//! The synchronous half of the store: one `Connection`, one command at a time.
//!
//! Every method here runs on the store task's thread and is exercised by tests directly, which is
//! what keeps the threaded part (`task.rs`) down to plumbing.

use std::path::Path;

use rusqlite::{Connection, OpenFlags, OptionalExtension, params};

use crate::schedule::{FixedMultiplier, PromptState, Scheduler};

use super::{
    CardSummary, Command, Deck, DuePrompt, Grade, Id, Outcome, Phase, ReviewRow, Settings, Snapshot,
    Stats, StoreError, schema,
};

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
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        let mut db = Self { conn };
        db.conn.pragma_update(None, "journal_mode", "WAL")?;
        db.conn.pragma_update(None, "foreign_keys", true)?;
        schema::migrate(&mut db.conn)?;
        Ok(db)
    }

    /// Apply one command and report what it did. `now_ms` is the caller's clock, never this
    /// module's.
    pub fn apply(&mut self, command: &Command, now_ms: i64) -> Result<Outcome, StoreError> {
        match command {
            Command::CreateDeck { parent, name } => {
                self.conn.execute(
                    "INSERT INTO deck (parent_id, name) VALUES (?1, ?2)",
                    params![parent, name],
                )?;
                Ok(Outcome::Deck(self.conn.last_insert_rowid()))
            }
            Command::RenameDeck { id, name } => {
                let changed = self.conn.execute(
                    "UPDATE deck SET name = ?2 WHERE id = ?1",
                    params![id, name],
                )?;
                Ok(Outcome::Changed(changed))
            }
            Command::ArchiveDeck { id, archived } => {
                let changed = self.conn.execute(
                    "UPDATE deck SET archived = ?2 WHERE id = ?1",
                    params![id, i64::from(*archived)],
                )?;
                Ok(Outcome::Changed(changed))
            }
            Command::CreateCard { deck, markdown } => {
                let sides = sides_of(markdown);
                self.conn.execute(
                    "INSERT INTO card (deck_id, markdown, sides, search_text, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                    params![deck, markdown, sides as i64, searchable(markdown), now_ms],
                )?;
                let id = self.conn.last_insert_rowid();
                self.sync_prompt(id, sides)?;
                Ok(Outcome::Card(id))
            }
            Command::SaveCard { id, markdown } => {
                let sides = sides_of(markdown);
                let changed = self.conn.execute(
                    "UPDATE card SET markdown = ?2, sides = ?3, search_text = ?4, updated_at = ?5 WHERE id = ?1",
                    params![id, markdown, sides as i64, searchable(markdown), now_ms],
                )?;
                self.sync_prompt(*id, sides)?;
                Ok(Outcome::Changed(changed))
            }
            Command::TrashCard { id } => {
                let changed = self.conn.execute(
                    "UPDATE card SET trashed_at = ?2 WHERE id = ?1",
                    params![id, now_ms],
                )?;
                Ok(Outcome::Changed(changed))
            }
            Command::RestoreCard { id } => {
                let changed = self
                    .conn
                    .execute("UPDATE card SET trashed_at = NULL WHERE id = ?1", params![id])?;
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
            Command::SetSetting { key, value } => {
                self.conn.execute(
                    "INSERT INTO settings (key, value) VALUES (?1, ?2)
                     ON CONFLICT (key) DO UPDATE SET value = excluded.value",
                    params![key, value],
                )?;
                Ok(Outcome::Ok)
            }
            Command::Grade { card, key, grade, duration_ms } => {
                self.grade(*card, key, *grade, *duration_ms, now_ms)
            }
            Command::ResetHistory { card, key } => self.reset_history(*card, key),
            Command::History { card } => Ok(Outcome::History(self.history(*card)?)),
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
                "SELECT id, deck_id, sides, archived, trashed_at, updated_at FROM card ORDER BY updated_at DESC, id",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok(CardSummary {
                    id: row.get(0)?,
                    deck: row.get(1)?,
                    sides: row.get::<_, i64>(2)?.max(0) as u32,
                    archived: row.get::<_, i64>(3)? != 0,
                    trashed_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };
        Ok(Snapshot { decks, cards, settings: self.settings()?, revision: 0 })
    }

    pub fn settings(&self) -> Result<Settings, StoreError> {
        let limit: Option<String> = self
            .conn
            .query_row("SELECT value FROM settings WHERE key = 'new_per_day'", [], |row| row.get(0))
            .optional()?;
        Ok(Settings {
            new_per_day: limit.and_then(|v| v.parse().ok()).unwrap_or(Settings::DEFAULT_NEW_PER_DAY),
        })
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

    /// Append one grade and leave the prompt agreeing with it.
    ///
    /// The recorded phase is where the grade *put* the prompt: a first success graduates the card
    /// and is recorded as `review`, a failure in the review phase is recorded as `relearn`.
    pub fn grade(
        &mut self,
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
        Ok(Outcome::Graded { phase: outcome.phase, due_at: outcome.due_at })
    }

    /// Drop a prompt's whole history and put it back to new, which is the only place records are
    /// ever removed: individually they are append-only.
    pub fn reset_history(&mut self, card: Id, key: &str) -> Result<Outcome, StoreError> {
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
                |row| {
                    Ok(PromptState {
                        phase: Phase::parse(&row.get::<_, String>(0)?),
                        interval_ms: row.get(1)?,
                    })
                },
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => {
                    StoreError::NoPrompt { card, key: key.to_string() }
                }
                other => StoreError::Sqlite(other),
            })
    }

    /// Recompute a prompt from its records, so the prompt row never says more than the history.
    fn refresh_prompt(&mut self, card: Id, key: &str) -> Result<(), StoreError> {
        let Some(folded) = self.fold_records(card, key)? else { return Ok(()) };
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
    pub(crate) fn append_record_for_test(
        &mut self,
        card: Id,
        key: &str,
        phase: Phase,
        grade: Grade,
        at: i64,
        interval_ms: i64,
        due_at: Option<i64>,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO review_log (card_id, key, phase, at, grade, interval_ms, due_at, duration_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL)",
            params![card, key, phase.as_str(), at, grade.as_str(), interval_ms, due_at],
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
               AND c.deck_id NOT IN (SELECT id FROM archived_deck)",
        )?;
        let mut due = stmt
            .query_map(params![now_ms], |row| {
                Ok(DuePrompt { card: row.get(0)?, key: row.get(1)?, due_at: row.get(2)? })
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
                .position(|prompt| today - crate::day::study_day(prompt.due_at, offset_ms) != overdue)
                .unwrap_or(rest.len());
            let mut group: Vec<DuePrompt> = rest[..same].to_vec();
            shuffle.reorder(&mut group);
            ordered.extend(group);
            rest = &rest[same..];
        }
        Ok(ordered)
    }

    /// One prompt row as stored.
    pub fn prompt(&self, card: Id, key: &str) -> Result<Option<Folded>, StoreError> {
        self.conn
            .query_row(
                "SELECT phase, interval_ms, due_at, reps, lapses
                 FROM prompt WHERE card_id = ?1 AND key = ?2",
                params![card, key],
                |row| {
                    Ok(Folded {
                        phase: Phase::parse(&row.get::<_, String>(0)?),
                        interval_ms: row.get(1)?,
                        due_at: row.get(2)?,
                        reps: row.get(3)?,
                        lapses: row.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    /// Rewrite every prompt from its records, which is what makes the prompt a cache rather than a
    /// second source of truth. Returns how many prompts were rewritten.
    pub fn rebuild_prompts(&mut self) -> Result<usize, StoreError> {
        let pairs = {
            let mut stmt = self
                .conn
                .prepare("SELECT DISTINCT card_id, key FROM review_log ORDER BY card_id, key")?;
            let rows = stmt.query_map([], |row| Ok((row.get::<_, Id>(0)?, row.get::<_, String>(1)?)))?;
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
        let Some(last) = counted.last() else { return Ok(None) };
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
        self.conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(grade = 'forgot'), 0) FROM review_log
             WHERE phase <> 'learn' AND at >= ?1 AND at < ?2",
            params![from_ms, to_ms],
            |row| {
                Ok(Stats {
                    reviews: row.get::<_, i64>(0)?.max(0) as u32,
                    lapses: row.get::<_, i64>(1)?.max(0) as u32,
                })
            },
        ).map_err(Into::into)
    }

    /// Every grade a card ever got, oldest first, learning cycles included.
    pub fn history(&self, card: Id) -> Result<Vec<ReviewRow>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT card_id, key, phase, at, grade, interval_ms, due_at, duration_ms
             FROM review_log WHERE card_id = ?1 ORDER BY at, rowid",
        )?;
        let rows = stmt.query_map(params![card], |row| {
            Ok(ReviewRow {
                card: row.get(0)?,
                key: row.get(1)?,
                phase: Phase::parse(&row.get::<_, String>(2)?),
                at: row.get(3)?,
                grade: Grade::parse(&row.get::<_, String>(4)?),
                interval_ms: row.get(5)?,
                due_at: row.get(6)?,
                duration_ms: row.get(7)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
}

fn sides_of(markdown: &str) -> usize {
    crate::sides::split(markdown).len()
}

/// What search compares against: the text in the case a `LIKE` can fold.
fn searchable(text: &str) -> String {
    text.to_lowercase()
}
