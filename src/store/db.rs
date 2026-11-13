//! The synchronous half of the store: one `Connection`, one command at a time.
//!
//! Every method here runs on the store task's thread and is exercised by tests directly, which is
//! what keeps the threaded part (`task.rs`) down to plumbing.

use std::path::Path;

use rusqlite::{Connection, OpenFlags, OptionalExtension, params};

use crate::schedule::{FixedMultiplier, PromptState, Scheduler};

use super::{
    CardSummary, Command, Deck, Grade, Id, Outcome, Phase, ReviewRow, Settings, Snapshot, Stats,
    StoreError, schema,
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
