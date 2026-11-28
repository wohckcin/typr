//! Review recording, scheduling queries, history folding, stats, and settings writes.

use rusqlite::{OptionalExtension, params};

use crate::command::{Command, Outcome};
use crate::error::StoreError;
use crate::fsrs::Memory;
use crate::model::{
    DuePrompt, Folded, Grade, Id, NewPrompt, Phase, ReviewRow, SchedulerKind, SettingKey,
    SettingValue, Stats,
};
use crate::schedule::{FixedMultiplier, Fsrs, Policy, PromptState, Scheduler};

#[cfg(test)]
use super::core::Record;
use super::core::{Db, count_of};

impl Db {
    pub(super) fn dispatch_review(
        &self,
        command: &Command,
        now_ms: i64,
    ) -> Result<Option<Outcome>, StoreError> {
        let outcome = match command {
            Command::SetSetting { key, value } => self.write_setting(*key, value)?,
            Command::SetScheduler { kind } => {
                self.write_setting(SettingKey::Scheduler, &SettingValue::Scheduler(*kind))?
            }
            Command::SetRetention { percent } => {
                self.write_setting(SettingKey::Retention, &SettingValue::Number(*percent))?
            }
            Command::SetWeights { weights } => {
                self.write_setting(SettingKey::Weights, &SettingValue::Weights(weights.clone()))?
            }
            Command::Grade {
                card,
                key,
                grade,
                duration_ms,
            } => self.grade(*card, key, *grade, *duration_ms, now_ms)?,
            Command::ResetHistory { card, key } => self.reset_history(*card, key)?,
            Command::DuePrompts {
                now_ms,
                offset_ms,
                seed,
                deck,
            } => Outcome::Due(self.due_prompts(*now_ms, *offset_ms, *seed, *deck)?),
            Command::NewPrompts {
                now_ms,
                offset_ms,
                seed,
                deck,
            } => Outcome::New(self.new_prompts(*now_ms, *offset_ms, *seed, *deck)?),
            Command::History { card } => Outcome::History {
                rows: self.history(*card)?,
                reviews: self.review_count(*card)?,
            },
            Command::Stats { from_ms, to_ms } => Outcome::Stats(self.stats(*from_ms, *to_ms)?),
            _ => return Ok(None),
        };
        Ok(Some(outcome))
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
        self.record_review(card, key, grade, duration_ms, now_ms)
    }

    /// Append one review at `at` and leave the prompt agreeing with it. `at` is a parameter
    /// because an import replays records at the time each happened, not at the time of the import.
    pub(crate) fn record_review(
        &self,
        card: Id,
        key: &str,
        grade: Grade,
        duration_ms: Option<i64>,
        at: i64,
    ) -> Result<Outcome, StoreError> {
        let state = self.prompt_state(card, key)?;
        let (scheduler, policy) = self.scheduler()?;
        let outcome = scheduler.grade(&state, grade, at, policy);
        self.conn.execute(
            "INSERT INTO review_log (card_id, key, phase, at, grade, interval_ms, due_at, duration_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                card,
                key,
                outcome.phase.as_str(),
                at,
                grade.as_str(),
                outcome.interval_ms,
                outcome.due_at,
                duration_ms
            ],
        )?;
        self.refresh_prompt(card, key)?;
        // The scheduler's memory is not derivable from the log, so it is written beside the
        // prompt. A scheduler with no memory (the fixed one) leaves what is there untouched: an
        // empty column would make a switch back re-bootstrap from a guess.
        if let Some(memory) = outcome.memory {
            self.conn.execute(
                "UPDATE prompt SET stability = ?3, difficulty = ?4 WHERE card_id = ?1 AND key = ?2",
                params![card, key, memory.stability, memory.difficulty],
            )?;
        }
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
            "UPDATE prompt SET phase = 'learn', due_at = NULL, interval_ms = 0, reps = 0, lapses = 0,
                 stability = NULL, difficulty = NULL
             WHERE card_id = ?1 AND key = ?2",
            params![card, key],
        )?;
        Ok(Outcome::Changed(removed))
    }

    /// The raw connection, for tests that have to inspect what the schema actually wrote.
    #[cfg(test)]
    pub(crate) fn connection_for_test(&self) -> &rusqlite::Connection {
        &self.conn
    }

    /// Break the library so a snapshot read fails, for the worker's error path.
    #[cfg(test)]
    pub(crate) fn drop_card_table_for_test(&self) -> Result<(), StoreError> {
        self.conn.execute_batch("DROP TABLE card")?;
        Ok(())
    }

    /// Every record in the log, for tests that prove an emptied card takes its history with it.
    #[cfg(test)]
    pub fn record_total(&self) -> Result<i64, StoreError> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM review_log", [], |row| row.get(0))?)
    }

    /// For tests of the slow-command record: makes every later review take far longer than the
    /// app's slow-command threshold, by a trigger that does busywork on each `review_log` insert.
    pub fn slow_down_reviews(&self) -> Result<(), StoreError> {
        self.conn.execute_batch(
            "CREATE TRIGGER IF NOT EXISTS slow_review AFTER INSERT ON review_log BEGIN
                 SELECT count(*) FROM (WITH RECURSIVE c(x) AS
                     (SELECT 1 UNION ALL SELECT x + 1 FROM c WHERE x < 1000000) SELECT x FROM c);
             END;",
        )?;
        Ok(())
    }

    /// How many real reviews a card has had: learning cycles are not reviews.
    pub fn review_count(&self, card: Id) -> Result<u32, StoreError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM review_log WHERE card_id = ?1 AND phase <> 'learn'",
            params![card],
            |row| row.get(0),
        )?;
        Ok(count_of(count))
    }

    /// Write one setting, refusing a value the reader would later refuse.
    ///
    /// The refusal happens before the write, so a library never holds a value that would make the
    /// next snapshot fail. Every command that changes a setting comes through here, and the key
    /// and value it accepts are the same pair [`SettingKey::parse_value`] reads back.
    fn write_setting(&self, key: SettingKey, value: &SettingValue) -> Result<Outcome, StoreError> {
        key.validate(value)?;
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            params![key.as_str(), value.text()],
        )?;
        Ok(Outcome::Ok)
    }

    /// The scheduler the library asked for, rebuilt from its settings on every grade so a settings
    /// change needs no restart.
    fn scheduler(&self) -> Result<(Box<dyn Scheduler>, Policy), StoreError> {
        let settings = self.settings()?;
        let policy = Policy::from_settings(&settings);
        let scheduler: Box<dyn Scheduler> = match settings.scheduler {
            SchedulerKind::Fixed => Box::new(FixedMultiplier),
            SchedulerKind::Fsrs => Box::new(Fsrs {
                weights: settings.weights.clone(),
                retention: f64::from(settings.retention) / 100.0,
            }),
        };
        Ok((scheduler, policy))
    }

    fn prompt_state(&self, card: Id, key: &str) -> Result<PromptState, StoreError> {
        self.conn
            .query_row(
                "SELECT phase, interval_ms, due_at, stability, difficulty,
                     (SELECT max(at) FROM review_log WHERE card_id = ?1 AND key = ?2)
                 FROM prompt WHERE card_id = ?1 AND key = ?2",
                params![card, key],
                |row| {
                    let stability: Option<f64> = row.get(3)?;
                    let difficulty: Option<f64> = row.get(4)?;
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, Option<i64>>(2)?,
                        stability
                            .zip(difficulty)
                            .map(|(stability, difficulty)| Memory {
                                stability,
                                difficulty,
                            }),
                        row.get::<_, Option<i64>>(5)?,
                    ))
                },
            )
            .map(
                |(phase, interval_ms, due_at, memory, last_at)| PromptState {
                    phase: Phase::parse(&phase).unwrap_or(Phase::Review),
                    interval_ms,
                    due_at,
                    last_at,
                    memory,
                },
            )
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
        deck: Option<Id>,
    ) -> Result<Vec<DuePrompt>, StoreError> {
        let mut stmt = self.conn.prepare(
            "WITH RECURSIVE subtree(id) AS (
                 SELECT ?2 WHERE ?2 IS NOT NULL
                 UNION
                 SELECT d.id FROM deck d JOIN subtree s ON d.parent_id = s.id
             )
             SELECT card_id, key, due_at
             FROM studyable_prompt
             WHERE due_at IS NOT NULL AND due_at <= ?1
               AND (?2 IS NULL OR card_id IN (SELECT id FROM card WHERE deck_id IN (SELECT id FROM subtree)))
             ORDER BY due_at, card_id",
        )?;
        let mut due = stmt
            .query_map(params![now_ms, deck], |row| {
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

    /// Cards with a prompt that graduated on one study day, counted across every deck.
    ///
    /// A grade before the prompt ever left the new queue does not count: the cap is on cards
    /// *learned*, not on how many times a new card was answered today. A prompt added to an older
    /// card counts that card on the day the prompt is first learned.
    pub fn learned_on(&self, day: i64, offset_ms: i64) -> Result<u32, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT card_id, MIN(at) FROM review_log WHERE phase <> 'learn' GROUP BY card_id, key",
        )?;
        let firsts = stmt
            .query_map([], |row| Ok((row.get::<_, Id>(0)?, row.get::<_, i64>(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        let count = firsts
            .into_iter()
            .filter(|(_, at)| crate::day::study_day(*at, offset_ms) == day)
            .map(|(card, _)| card)
            .collect::<std::collections::HashSet<_>>()
            .len();
        Ok(u32::try_from(count).unwrap_or(u32::MAX))
    }

    /// The new-card queue: studyable cards with nothing scheduled yet (phase `learn` and no due date), capped by the
    /// settings' daily allowance minus what was already learned today.
    pub fn new_prompts(
        &self,
        now_ms: i64,
        offset_ms: i64,
        seed: u64,
        deck: Option<Id>,
    ) -> Result<Vec<NewPrompt>, StoreError> {
        // "New" is the absence of a schedule, not the absence of history: a card whose first
        // lesson was forgotten has records and still has nothing scheduled.
        let mut stmt = self.conn.prepare(
            "WITH RECURSIVE subtree(id) AS (
                 SELECT ?1 WHERE ?1 IS NOT NULL
                 UNION
                 SELECT d.id FROM deck d JOIN subtree s ON d.parent_id = s.id
             )
             SELECT card_id, key
             FROM studyable_prompt p
             WHERE p.phase = 'learn' AND p.due_at IS NULL
               AND (?1 IS NULL OR card_id IN (SELECT id FROM card WHERE deck_id IN (SELECT id FROM subtree)))
             ORDER BY card_id, key",
        )?;
        let waiting = stmt
            .query_map(params![deck], |row| {
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

        // The cap counts cards, so a card's groups come and go together: admitting one group of a
        // card while its sibling waits would split a card's first lesson across two days.
        let mut admitted: std::collections::HashSet<Id> = std::collections::HashSet::new();
        let mut kept = Vec::new();
        for prompt in waiting {
            if admitted.contains(&prompt.card) {
                kept.push(prompt);
            } else if admitted.len() < room {
                admitted.insert(prompt.card);
                kept.push(prompt);
            }
        }
        Ok(kept)
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
                        reviews: count_of(row.get(0)?),
                        lapses: count_of(row.get(1)?),
                    })
                },
            )
            .map_err(Into::into)
    }

    /// Every record in the library, as the table an external FSRS optimiser reads.
    ///
    /// One row per record, oldest first, so a file walked in order shows each card's reviews in the
    /// order they happened. The columns are all fixed words or numbers — no markdown, no card text
    /// — which is why nothing here needs quoting. The optimiser is not run: this is history out,
    /// weights in.
    ///
    /// `review_rating` is the optimiser's number rather than our word for it, and `review_state` is
    /// the state the prompt was in *before* the grade — `review_log.phase` holds where the grade
    /// left it, so copying that column would shift every state by one review.
    pub fn export_reviews(&self) -> Result<String, StoreError> {
        use std::fmt::Write as _;

        let mut stmt = self.conn.prepare(
            "SELECT card_id, at, grade,
                    LAG(phase) OVER (PARTITION BY card_id, key ORDER BY at, rowid),
                    duration_ms
             FROM review_log ORDER BY at, rowid",
        )?;
        let mut rows = stmt.query([])?;
        let mut out =
            String::from("card_id,review_time,review_rating,review_state,review_duration\n");
        while let Some(row) = rows.next()? {
            let card: Id = row.get(0)?;
            let at: i64 = row.get(1)?;
            let grade: String = row.get(2)?;
            let before: Option<String> = row.get(3)?;
            let duration: Option<i64> = row.get(4)?;
            let rating =
                Grade::parse(&grade)
                    .map(crate::fsrs::rating_of)
                    .ok_or(StoreError::Text {
                        column: "review_log.grade",
                        value: grade,
                    })?;
            let state = state_number(before.as_deref());
            let duration = duration.map(|ms| ms.to_string()).unwrap_or_default();
            writeln!(out, "{card},{at},{rating},{state},{duration}").map_err(|_| {
                StoreError::Text {
                    column: "review_log",
                    value: "row".to_string(),
                }
            })?;
        }
        Ok(out)
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
}

/// The optimiser's name for the state a prompt was in: New, Learning, Review, Relearning.
fn state_number(before: Option<&str>) -> u8 {
    match before.and_then(Phase::parse) {
        None => 0,
        Some(Phase::Learn) => 1,
        Some(Phase::Review) => 2,
        Some(Phase::Relearn) => 3,
    }
}
