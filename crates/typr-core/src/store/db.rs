//! The synchronous half of the store: one `Connection`, one command at a time.
//!
//! Every method here runs on the store task's thread and is exercised by tests directly, which is
//! what keeps the threaded part (`task.rs`) down to plumbing.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags, OptionalExtension, params};

use crate::hidden::{self, Shape};
use crate::schedule::{FixedMultiplier, PromptState, Scheduler};
use crate::template;
use crate::transfer::{self, mochi};

use super::{
    CardKind, CardMode, CardSummary, Command, Deck, DuePrompt, Grade, Id, MaskRect, MaskRow,
    NewPrompt, Outcome, Phase, ReviewRow, SavedView, Settings, Snapshot, Sort, SortDir, Stats,
    StoreError, ViewSpec,
    schema::{self, SchemaFacts},
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
    schema: SchemaFacts,
    path: PathBuf,
}

impl Db {
    /// Open (creating if needed) the database at `path` and bring its schema up to date.
    ///
    /// A path that is not a database is an error: the file is read for its header before anything
    /// is written, so an unrelated file is reported rather than overwritten.
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        // A failure here is about this file, so the name of the file travels with the reason: a
        // reader with "file is not a database" and no path has nothing to act on.
        let named = |error: StoreError| match error {
            StoreError::Sqlite(source) => StoreError::Unreadable {
                path: path.to_path_buf(),
                source,
            },
            other => other,
        };
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|error| named(error.into()))?;
        let mut db = Self {
            conn,
            schema: SchemaFacts {
                found: 0,
                written: 0,
                applied: 0,
            },
            path: path.to_path_buf(),
        };
        db.conn
            .pragma_update(None, "journal_mode", "WAL")
            .map_err(|error| named(error.into()))?;
        db.conn
            .pragma_update(None, "foreign_keys", true)
            .map_err(|error| named(error.into()))?;
        db.schema = schema::migrate(&mut db.conn).map_err(named)?;
        Ok(db)
    }

    /// What the file recorded about its schema before it was opened, and what it records now.
    pub fn schema_facts(&self) -> SchemaFacts {
        self.schema
    }

    /// The file this library lives in, for a log line or an error message.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Apply one command and report what it did. `now_ms` is the caller's clock, never this
    /// module's.
    ///
    /// The whole command is one transaction: a command that fails halfway leaves nothing behind,
    /// and a command that writes several rows pays one fsync.
    pub fn apply(&self, command: &Command, now_ms: i64) -> Result<Outcome, StoreError> {
        // `VACUUM INTO` refuses to run inside a transaction, so the whole-library export is the
        // one command that does its work before the transaction rather than in it.
        if let Command::ExportLibrary { target } = command {
            transfer::export_library(&self.path, target)?;
            return Ok(Outcome::Exported);
        }
        let tx = self.conn.unchecked_transaction()?;
        let outcome = self.dispatch(command, now_ms)?;
        tx.commit()?;
        Ok(outcome)
    }

    /// Run one command inside the transaction the caller already holds. The import uses this so a
    /// forty-card archive pays one commit, and a failure halfway leaves no deck behind.
    pub(crate) fn staged(&self, command: &Command, now_ms: i64) -> Result<Outcome, StoreError> {
        self.dispatch(command, now_ms)
    }

    /// Write a row whose name is unique, turning SQLite's refusal into something the reader
    /// understands: "a view called X already exists" is a sentence, the constraint is not.
    ///
    /// Only the extended code for a unique violation is translated: a foreign key or a check
    /// failure stays a database error, because it means the caller asked for something else.
    fn named_write(
        &self,
        sql: &str,
        params: impl rusqlite::Params,
        what: &'static str,
        name: &str,
    ) -> Result<usize, StoreError> {
        self.conn.execute(sql, params).map_err(|error| {
            if error.sqlite_extended_error_code() == Some(rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE) {
                StoreError::DuplicateName {
                    what,
                    name: name.to_string(),
                }
            } else {
                StoreError::Sqlite(error)
            }
        })
    }

    /// The body of one command, run inside the transaction [`Db::apply`] opens.
    fn dispatch(&self, command: &Command, now_ms: i64) -> Result<Outcome, StoreError> {
        match command {
            Command::CreateDeck { parent, name } => {
                self.named_write(
                    "INSERT INTO deck (parent_id, name) VALUES (?1, ?2)",
                    params![parent, name],
                    "deck under that parent",
                    name,
                )?;
                Ok(Outcome::Deck(self.conn.last_insert_rowid()))
            }
            Command::RenameDeck { id, name } => {
                let changed = self.named_write(
                    "UPDATE deck SET name = ?2 WHERE id = ?1",
                    params![id, name],
                    "deck under that parent",
                    name,
                )?;
                // `<<Deck>>` is the card's own deck name, so every card under this one says
                // something different now.
                for card in self.subtree_cards(*id)? {
                    self.rerender(card, now_ms)?;
                }
                Ok(Outcome::Changed(changed))
            }
            Command::ArchiveDeck { id, archived } => {
                let changed = self.conn.execute(
                    "UPDATE deck SET archived = ?2 WHERE id = ?1",
                    params![id, i64::from(*archived)],
                )?;
                Ok(Outcome::Changed(changed))
            }
            Command::MoveDeck { id, parent } => self.move_deck(*id, *parent, now_ms),
            Command::CreateCard { deck, markdown } => {
                let shape = hidden::shape(markdown);
                self.conn.execute(
                    "INSERT INTO card (deck_id, markdown, sides, search_text, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                    params![deck, markdown, hidden::faces(markdown), folded(markdown), now_ms],
                )?;
                let id = self.conn.last_insert_rowid();
                self.sync_prompts(id, &shape)?;
                // A card created straight into a templated deck says what its template says, not
                // what the caller typed; without fields it renders empty, which is the truth.
                self.rerender(id, now_ms)?;
                self.conn.execute(
                    "DELETE FROM editor_draft WHERE card_id IN (0, ?1)",
                    params![id],
                )?;
                Ok(Outcome::Card(id))
            }
            Command::CreateDiagram {
                deck,
                image,
                caption,
            } => {
                self.conn.execute(
                    "INSERT INTO card
                        (deck_id, kind, image, caption, markdown, sides, search_text,
                         created_at, updated_at)
                     VALUES (?1, 'diagram', ?2, ?3, '', 0, '', ?4, ?4)",
                    params![deck, image, caption, now_ms],
                )?;
                let id = self.conn.last_insert_rowid();
                self.sync_prompts(id, &hidden::shape(""))?;
                self.conn.execute(
                    "DELETE FROM editor_draft WHERE card_id IN (0, ?1)",
                    params![id],
                )?;
                Ok(Outcome::Card(id))
            }
            Command::SetDiagram {
                card,
                image,
                caption,
            } => {
                // Text already on the card would be hidden rather than kept, so the switch waits
                // until it is cleared.
                let holds_text = self.conn.query_row(
                    "SELECT markdown <> '' OR has_cloze = 1 FROM card WHERE id = ?1",
                    params![card],
                    |row| row.get::<_, bool>(0),
                )?;
                if holds_text {
                    return Err(StoreError::NotEmpty {
                        card: *card,
                        holds: "text",
                    });
                }
                let changed = self.conn.execute(
                    "UPDATE card SET kind = 'diagram', image = ?2, caption = ?3, updated_at = ?4
                     WHERE id = ?1",
                    params![card, image, caption, now_ms],
                )?;
                if changed == 0 {
                    return Err(StoreError::Missing {
                        what: "card",
                        id: *card,
                    });
                }
                self.sync_prompts(*card, &self.shape_of(*card)?)?;
                Ok(Outcome::Changed(changed))
            }
            Command::AddMask { card, group, rect } => {
                check_mask(*card, *rect)?;
                self.place_mask(*card, *rect, *group)?;
                self.sync_prompts(*card, &self.shape_of(*card)?)?;
                Ok(Outcome::Changed(1))
            }
            Command::SetMaskGroup { card, rect, group } => {
                check_mask(*card, *rect)?;
                let changed = self.conn.execute(
                    "UPDATE mask SET group_no = ?2 WHERE card_id = ?1 AND x = ?3 AND y = ?4 AND w = ?5 AND h = ?6",
                    params![card, group, rect.x, rect.y, rect.w, rect.h],
                )?;
                if changed == 0 {
                    return Err(StoreError::Missing {
                        what: "mask",
                        id: *card,
                    });
                }
                self.sync_prompts(*card, &self.shape_of(*card)?)?;
                Ok(Outcome::Changed(changed))
            }
            Command::RemoveMask { card, rect } => {
                let changed = self.conn.execute(
                    "DELETE FROM mask WHERE card_id = ?1 AND x = ?2 AND y = ?3 AND w = ?4 AND h = ?5",
                    params![card, rect.x, rect.y, rect.w, rect.h],
                )?;
                if changed == 0 {
                    return Err(StoreError::Missing {
                        what: "mask",
                        id: *card,
                    });
                }
                self.sync_prompts(*card, &self.shape_of(*card)?)?;
                Ok(Outcome::Changed(changed))
            }
            Command::MergeMasks { card, rects, group } => {
                let mut changed = 0;
                for rect in rects {
                    check_mask(*card, *rect)?;
                    changed += self.place_mask(*card, *rect, *group)?;
                }
                if changed == 0 {
                    return Err(StoreError::Missing {
                        what: "mask",
                        id: *card,
                    });
                }
                self.sync_prompts(*card, &self.shape_of(*card)?)?;
                Ok(Outcome::Changed(changed))
            }
            Command::MaskGroups { card } => Ok(Outcome::MaskGroups(self.mask_groups(*card)?)),
            Command::Masks { card } => Ok(Outcome::Masks(self.masks(*card)?)),
            Command::SaveCard { id, markdown } => {
                // A diagram's prompt comes from its rectangles, so text written over one would be
                // stored and never asked.
                if self.kind_of(*id)? == "diagram" {
                    return Err(StoreError::NotEmpty {
                        card: *id,
                        holds: "a picture",
                    });
                }
                // A templated card's text is rendered from its fields, so a hand-written one would
                // survive only until the next field edit overwrote it.
                if self.template_of(*id)?.is_some() {
                    return Err(StoreError::NotEmpty {
                        card: *id,
                        holds: "a template",
                    });
                }
                let shape = hidden::shape(markdown);
                let changed = self.conn.execute(
                    "UPDATE card SET markdown = ?2, sides = ?3, search_text = ?4, updated_at = ?5 WHERE id = ?1",
                    params![id, markdown, hidden::faces(markdown), folded(markdown), now_ms],
                )?;
                self.sync_prompts(*id, &shape)?;
                self.conn
                    .execute("DELETE FROM editor_draft WHERE card_id = ?1", params![id])?;
                self.rename(*id)?;
                Ok(Outcome::Changed(changed))
            }
            Command::MoveCard { id, deck } => {
                let changed = self.conn.execute(
                    "UPDATE card SET deck_id = ?2, updated_at = ?3 WHERE id = ?1",
                    params![id, deck, now_ms],
                )?;
                // The card may have landed under a deck that sets a template, or left one behind.
                self.rerender(*id, now_ms)?;
                Ok(Outcome::Changed(changed))
            }
            Command::SetFields { card, fields } => self.set_fields(*card, fields, now_ms),
            Command::SetCardTemplate {
                card,
                template,
                name,
            } => self.set_card_template(*card, template.as_deref(), name.as_deref(), now_ms),
            Command::SetDeckTemplate { deck, template } => {
                self.set_deck_template(*deck, template.as_deref(), now_ms)
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
                let known = match key.as_str() {
                    "new_per_day" => value.parse::<u32>().is_ok(),
                    // The two levels that override it are columns, not settings; this one is the
                    // value every card without an override inherits.
                    "reverse_default" => matches!(value.as_str(), "0" | "1"),
                    _ => true,
                };
                if !known {
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
            Command::SetCardReverse { card, reverse } => {
                // NULL is "inherit", so the nearest deck's value applies again.
                let changed = self.conn.execute(
                    "UPDATE card SET reverse = ?2 WHERE id = ?1",
                    params![card, reverse.map(i64::from)],
                )?;
                if changed == 0 {
                    return Err(StoreError::Missing {
                        what: "card",
                        id: *card,
                    });
                }
                Ok(Outcome::Changed(changed))
            }
            Command::SetDeckReverse { deck, reverse } => {
                let changed = self.conn.execute(
                    "UPDATE deck SET reverse = ?2 WHERE id = ?1",
                    params![deck, reverse.map(i64::from)],
                )?;
                if changed == 0 {
                    return Err(StoreError::Missing {
                        what: "deck",
                        id: *deck,
                    });
                }
                Ok(Outcome::Changed(changed))
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
                deck,
            } => Ok(Outcome::Due(
                self.due_prompts(*now_ms, *offset_ms, *seed, *deck)?,
            )),
            Command::NewPrompts {
                now_ms,
                offset_ms,
                seed,
                deck,
            } => Ok(Outcome::New(
                self.new_prompts(*now_ms, *offset_ms, *seed, *deck)?,
            )),
            Command::CardsForView { spec } => Ok(Outcome::Cards(self.cards_for_view(spec)?)),
            Command::CreateView { spec, name } => {
                let name = name.trim();
                let changed = self.named_write(
                    "INSERT INTO view (name, deck, query, tags, mode, pinned_only, sort_key, sort_dir)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        name,
                        spec.deck,
                        spec.query,
                        encode_tags(&spec.tags),
                        mode_text(spec.mode),
                        i64::from(spec.pinned_only),
                        spec.sort.as_str(),
                        spec.dir.as_str(),
                    ],
                    "view",
                    name,
                )?;
                debug_assert_eq!(changed, 1, "one row is one view");
                Ok(Outcome::View(self.conn.last_insert_rowid()))
            }
            Command::RenameView { id, name } => {
                let name = name.trim();
                let changed = self.named_write(
                    "UPDATE view SET name = ?2 WHERE id = ?1",
                    params![id, name],
                    "view",
                    name,
                )?;
                Ok(Outcome::Changed(changed))
            }
            Command::DeleteView { id } => Ok(Outcome::Changed(
                self.conn
                    .execute("DELETE FROM view WHERE id = ?1", params![id])?,
            )),
            Command::UpdateView { id, spec } => Ok(Outcome::Changed(self.conn.execute(
                "UPDATE view SET deck = ?2, query = ?3, tags = ?4, mode = ?5, pinned_only = ?6,
                                sort_key = ?7, sort_dir = ?8
                 WHERE id = ?1",
                params![
                    id,
                    spec.deck,
                    spec.query,
                    encode_tags(&spec.tags),
                    mode_text(spec.mode),
                    i64::from(spec.pinned_only),
                    spec.sort.as_str(),
                    spec.dir.as_str(),
                ],
            )?)),
            Command::SetPinned { card, pinned } => Ok(Outcome::Changed(self.conn.execute(
                "UPDATE card SET pinned = ?2 WHERE id = ?1",
                params![card, i64::from(*pinned)],
            )?)),
            Command::History { card } => Ok(Outcome::History {
                rows: self.history(*card)?,
                reviews: self.review_count(*card)?,
            }),
            Command::Stats { from_ms, to_ms } => Ok(Outcome::Stats(self.stats(*from_ms, *to_ms)?)),
            Command::ImportArchive {
                archive,
                attachments,
            } => Ok(Outcome::Imported(mochi::import(
                self,
                archive,
                attachments,
                now_ms,
            )?)),
            // Export normally never reaches here: `apply` runs it before opening a transaction.
            // The arm keeps the match total and gives a clear failure if it is ever staged.
            Command::ExportLibrary { target } => {
                transfer::export_library(&self.path, target)?;
                Ok(Outcome::Exported)
            }
            Command::ImportCsv { deck, source } => Ok(Outcome::Imported(transfer::import_csv(
                self, *deck, source, now_ms,
            )?)),
            Command::ExportCsv { deck, target } => {
                transfer::export_csv(self, *deck, target)?;
                Ok(Outcome::Exported)
            }
            Command::ImportMarkdown { deck, source } => Ok(Outcome::Imported(
                transfer::import_markdown(self, *deck, source, now_ms)?,
            )),
            Command::ExportMarkdown { deck, target } => {
                transfer::export_markdown(self, *deck, target)?;
                Ok(Outcome::Exported)
            }
        }
    }

    /// Everything a view may read without a round trip.
    pub fn snapshot(&self) -> Result<Snapshot, StoreError> {
        let decks = {
            let mut stmt = self.conn.prepare(
                "SELECT id, parent_id, name, archived, reverse, template FROM deck
                 ORDER BY name COLLATE NOCASE, id",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok(Deck {
                    id: row.get(0)?,
                    parent: row.get(1)?,
                    name: row.get(2)?,
                    archived: row.get::<_, i64>(3)? != 0,
                    reverse: flag(row.get(4)?),
                    template: row.get(5)?,
                })
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };
        let cards = {
            let mut stmt = self.conn.prepare(
                "SELECT c.id, c.deck_id, c.sides, c.archived, c.trashed_at, c.updated_at, c.markdown,
                        c.search_text, COUNT(sp.card_id), SUM(sp.card_id IS NOT NULL AND sp.due_at IS NULL), MIN(sp.due_at),
                        c.reverse, c.kind, c.image, c.caption, c.template, c.pinned, c.created_at,
                        (SELECT COUNT(*) FROM review_log r WHERE r.card_id = c.id AND r.phase <> 'learn'),
                        c.name
                 FROM card c
                 LEFT JOIN studyable_prompt sp ON sp.card_id = c.id
                 GROUP BY c.id
                 ORDER BY c.updated_at DESC, c.id",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok(CardSummary {
                    id: row.get(0)?,
                    deck: row.get(1)?,
                    kind: CardKind::parse(&row.get::<_, String>(12)?),
                    image: row.get(13)?,
                    caption: row.get(14)?,
                    masks: Vec::new(),
                    sides: count_of(row.get(2)?),
                    tags: Vec::new(),
                    archived: row.get::<_, i64>(3)? != 0,
                    reverse: flag(row.get(11)?),
                    trashed_at: row.get(4)?,
                    updated_at: row.get(5)?,
                    created_at: row.get(17)?,
                    reviews: count_of(row.get(18)?),
                    pinned: row.get::<_, i64>(16)? != 0,
                    preview: preview(&row.get::<_, String>(6)?),
                    name: row.get(19)?,
                    markdown: row.get(6)?,
                    search_text: row.get(7)?,
                    prompts: count_of(row.get(8)?),
                    unscheduled: count_of(row.get(9)?),
                    due_at: row.get(10)?,
                    fields: Vec::new(),
                    // A card's own template, if it has one; the cards that inherit are filled in
                    // below, once every deck is in hand.
                    template: row.get(15)?,
                    template_from: None,
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

            // Only a diagram card carries rectangles, so the second pass is keyed by the cards that
            // need them and leaves every text card alone.
            let mut stmt = self.conn.prepare(
                "SELECT card_id, group_no, x, y, w, h FROM mask ORDER BY card_id, group_no, y, x",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, Id>(0)?,
                    MaskRow {
                        group: count_of(row.get::<_, i64>(1)?),
                        rect: MaskRect::new(row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?),
                    },
                ))
            })?;
            let mut by_card: HashMap<Id, Vec<MaskRow>> = HashMap::new();
            for row in rows {
                let (card, mask) = row?;
                by_card.entry(card).or_default().push(mask);
            }
            for summary in &mut cards {
                if let Some(masks) = by_card.remove(&summary.id) {
                    summary.masks = masks;
                }
            }

            let mut stmt = self
                .conn
                .prepare("SELECT card_id, name, value FROM field ORDER BY card_id, name")?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, Id>(0)?,
                    (row.get::<_, String>(1)?, row.get::<_, String>(2)?),
                ))
            })?;
            let mut by_card: HashMap<Id, Vec<(String, String)>> = HashMap::new();
            for row in rows {
                let (card, field) = row?;
                by_card.entry(card).or_default().push(field);
            }
            for summary in &mut cards {
                if let Some(fields) = by_card.remove(&summary.id) {
                    summary.fields = fields;
                }
            }

            // The template a card without one of its own inherits: the nearest deck above it that
            // sets one, resolved here rather than by a query per card.
            let chains: HashMap<Id, (Option<Id>, Option<String>)> = decks
                .iter()
                .map(|deck| (deck.id, (deck.parent, deck.template.clone())))
                .collect();
            for summary in &mut cards {
                if summary.template.is_some() {
                    continue;
                }
                let mut cursor = Some(summary.deck);
                let mut hops = 0;
                while let Some(id) = cursor {
                    hops += 1;
                    if hops > 64 {
                        break;
                    }
                    let Some((parent, template)) = chains.get(&id) else {
                        break;
                    };
                    if let Some(template) = template {
                        summary.template = Some(template.clone());
                        summary.template_from = Some(id);
                        break;
                    }
                    cursor = *parent;
                }
            }
            cards
        };
        let views = {
            let mut stmt = self.conn.prepare(
                "SELECT id, name, deck, query, tags, mode, pinned_only, sort_key, sort_dir
                 FROM view ORDER BY name COLLATE NOCASE, id",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok(SavedView {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    spec: ViewSpec {
                        deck: row.get(2)?,
                        query: row.get(3)?,
                        tags: decode_tags(&row.get::<_, String>(4)?),
                        mode: mode_parse(&row.get::<_, String>(5)?),
                        pinned_only: row.get::<_, i64>(6)? != 0,
                        sort: Sort::parse(&row.get::<_, String>(7)?).unwrap_or_default(),
                        dir: SortDir::parse(&row.get::<_, String>(8)?).unwrap_or_default(),
                    },
                })
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };
        Ok(Snapshot {
            decks,
            cards,
            views,
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
        let default: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'reverse_default'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        let reverse_default = match default.as_deref() {
            None | Some("0") => false,
            Some("1") => true,
            Some(value) => {
                return Err(StoreError::Setting {
                    key: "reverse_default".to_string(),
                    value: value.to_string(),
                });
            }
        };
        Ok(Settings {
            new_per_day,
            reverse_default,
        })
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
    /// The groups a diagram card asks about, in ascending order. A group exists when a rectangle
    /// names it, so removing the last rectangle of a group takes the group away with it.
    pub fn mask_groups(&self, card: Id) -> Result<Vec<u32>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT group_no FROM mask WHERE card_id = ?1 ORDER BY group_no")?;
        let rows = stmt.query_map(params![card], |row| row.get::<_, i64>(0))?;
        Ok(rows
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(count_of)
            .collect())
    }

    /// Every rectangle of a diagram card, ordered by group and then top-left first.
    pub fn masks(&self, card: Id) -> Result<Vec<MaskRow>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT group_no, x, y, w, h FROM mask WHERE card_id = ?1
             ORDER BY group_no, y, x",
        )?;
        let rows = stmt.query_map(params![card], |row| {
            Ok(MaskRow {
                group: count_of(row.get::<_, i64>(0)?),
                rect: MaskRect::new(row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?),
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// The fields a card fills its template from, ordered by name.
    fn fields_of(&self, card: Id) -> Result<Vec<(String, String)>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT name, value FROM field WHERE card_id = ?1 ORDER BY name")?;
        let rows = stmt.query_map(params![card], |row| Ok((row.get(0)?, row.get(1)?)))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// The template a card's text comes from, and the deck it is inherited from when it is not the
    /// card's own: `None` in the second place means the card carries it.
    ///
    /// Inheritance walks up the deck tree the way reverse does, so the nearest deck that sets a
    /// template wins. A card with no template anywhere answers `None`, which is what makes a
    /// library without templates behave exactly as it did before them.
    fn template_of(&self, card: Id) -> Result<Option<(String, Option<Id>)>, StoreError> {
        let row: Option<(Option<String>, Id)> = self
            .conn
            .query_row(
                "SELECT template, deck_id FROM card WHERE id = ?1",
                params![card],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let Some((own, deck)) = row else {
            return Ok(None);
        };
        if let Some(template) = own {
            return Ok(Some((template, None)));
        }
        let mut cursor = Some(deck);
        let mut hops = 0;
        while let Some(id) = cursor {
            // A cycle the schema does not allow still cannot be allowed to hang a render.
            hops += 1;
            if hops > 64 {
                return Ok(None);
            }
            let (template, parent): (Option<String>, Option<Id>) = self.conn.query_row(
                "SELECT template, parent_id FROM deck WHERE id = ?1",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;
            if let Some(template) = template {
                return Ok(Some((template, Some(id))));
            }
            cursor = parent;
        }
        Ok(None)
    }

    /// The name of the deck a card sits in, which `<<Deck>>` renders.
    fn deck_name_of(&self, card: Id) -> Result<Option<String>, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT d.name FROM card c JOIN deck d ON d.id = c.deck_id WHERE c.id = ?1",
                params![card],
                |row| row.get(0),
            )
            .optional()?)
    }

    /// Write a card's text from its template and fields, then name it from the text and fields it
    /// just wrote.
    ///
    /// `card.markdown` is a cache of `render(template, fields)`: every mutation that can change
    /// either calls this, so `sides`, `search_text` and the review can go on reading one plain
    /// string. A card with no template in force keeps the text it holds — which is what makes
    /// clearing a template leave the cards readable — but it is still named here, because a rename
    /// and the references that point at it have to move together.
    fn rerender(&self, card: Id, now_ms: i64) -> Result<(), StoreError> {
        // A diagram is asked from its rectangles, and its text is not what a template is for;
        // writing rendered text over it would queue text prompts it never shows. It has no name
        // either: a `[[reference]]` is text, and a diagram holds a picture.
        if self.kind_of(card)? == "diagram" {
            return Ok(());
        }
        if let Some((template, _from)) = self.template_of(card)? {
            let pairs = self.fields_of(card)?;
            let deck = self.deck_name_of(card)?;
            let text = template::render_card(&template, &pairs, deck.as_deref());
            let shape = hidden::shape(&text);
            // A value the template does not use is still part of what the card says, so search has
            // to reach it.
            let mut search = folded(&text);
            for (_, value) in &pairs {
                search.push(' ');
                search.push_str(&folded(value));
            }
            self.conn.execute(
                "UPDATE card SET markdown = ?2, sides = ?3, search_text = ?4, updated_at = ?5 WHERE id = ?1",
                params![card, text, hidden::faces(&text), search, now_ms],
            )?;
            self.sync_prompts(card, &shape)?;
        }
        self.rename(card)
    }

    /// Give a card the name its text and name template imply.
    ///
    /// Runs inside the save's transaction, after the text is written: the name is what `[[…]]`
    /// resolves against, so a card whose text is new and whose name is old would answer to a name
    /// it no longer says.
    fn rename(&self, card: Id) -> Result<(), StoreError> {
        let (markdown, current, template): (String, String, Option<String>) = self.conn.query_row(
            "SELECT markdown, name, name_template FROM card WHERE id = ?1",
            params![card],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        let first = crate::sides::split(&markdown)
            .first()
            .copied()
            .unwrap_or("");
        let pairs = self.fields_of(card)?;
        let wanted =
            crate::name::for_card(first, template.as_deref(), &template::Fields::new(&pairs));
        let name = self.unique_name(card, &wanted)?;
        if name != current {
            self.conn.execute(
                "UPDATE card SET name = ?2 WHERE id = ?1",
                params![card, name],
            )?;
        }
        Ok(())
    }

    /// `name` made unique among the cards in its deck, which is the scope a reference resolves in
    /// first.
    ///
    /// The card being named is left out of the comparison, so a suffixed name survives its own
    /// saves instead of gaining a second suffix each time.
    fn unique_name(&self, card: Id, name: &str) -> Result<String, StoreError> {
        if name.is_empty() {
            return Ok(String::new());
        }
        let deck: Id = self.conn.query_row(
            "SELECT deck_id FROM card WHERE id = ?1",
            params![card],
            |row| row.get(0),
        )?;
        let mut stmt = self
            .conn
            .prepare("SELECT name FROM card WHERE deck_id = ?1 AND id <> ?2 ORDER BY id")?;
        let rows = stmt.query_map(params![deck, card], |row| row.get::<_, String>(0))?;
        let taken = rows.collect::<Result<Vec<_>, _>>()?;
        let taken = taken.iter().map(String::as_str).collect::<Vec<_>>();
        Ok(crate::name::unique(name, &taken))
    }

    /// Replace a card's fields and its rendered text with them.
    fn set_fields(
        &self,
        card: Id,
        fields: &[(String, String)],
        now_ms: i64,
    ) -> Result<Outcome, StoreError> {
        self.require_card(card)?;
        self.conn
            .execute("DELETE FROM field WHERE card_id = ?1", params![card])?;
        let mut changed = 0;
        for (name, value) in fields {
            let name = name.trim();
            if name.is_empty() {
                continue;
            }
            changed += self.conn.execute(
                "INSERT INTO field (card_id, name, value) VALUES (?1, ?2, ?3)
                 ON CONFLICT (card_id, name) DO UPDATE SET value = excluded.value",
                params![card, name, value],
            )?;
        }
        self.rerender(card, now_ms)?;
        Ok(Outcome::Changed(changed))
    }

    /// Give a card its own text template and name template, or `None` to fall back for either.
    fn set_card_template(
        &self,
        card: Id,
        template: Option<&str>,
        name: Option<&str>,
        now_ms: i64,
    ) -> Result<Outcome, StoreError> {
        let changed = self.conn.execute(
            "UPDATE card SET template = ?2, name_template = ?3, updated_at = ?4 WHERE id = ?1",
            params![card, template, name, now_ms],
        )?;
        if changed == 0 {
            return Err(StoreError::Missing {
                what: "card",
                id: card,
            });
        }
        self.rerender(card, now_ms)?;
        Ok(Outcome::Changed(changed))
    }

    /// Give a deck a template its cards inherit, then re-render every card under it.
    ///
    /// One command, one transaction: a deck template change rewrites every card in that deck, and a
    /// failure halfway would otherwise leave some rendered against the new template and some
    /// against the old.
    fn set_deck_template(
        &self,
        deck: Id,
        template: Option<&str>,
        now_ms: i64,
    ) -> Result<Outcome, StoreError> {
        let changed = self.conn.execute(
            "UPDATE deck SET template = ?2 WHERE id = ?1",
            params![deck, template],
        )?;
        if changed == 0 {
            return Err(StoreError::Missing {
                what: "deck",
                id: deck,
            });
        }
        let cards = self.subtree_cards(deck)?;
        for card in &cards {
            // A card's own template still wins: `rerender` reads the template in force, not this
            // deck's.
            self.rerender(*card, now_ms)?;
        }
        Ok(Outcome::Changed(cards.len()))
    }

    /// Every card in a deck and the decks under it.
    fn subtree_cards(&self, deck: Id) -> Result<Vec<Id>, StoreError> {
        let mut stmt = self.conn.prepare(
            "WITH RECURSIVE under(id) AS (
                 SELECT ?1
                 UNION ALL
                 SELECT d.id FROM deck d JOIN under u ON d.parent_id = u.id
             )
             SELECT c.id FROM card c JOIN under u ON u.id = c.deck_id ORDER BY c.id",
        )?;
        let rows = stmt.query_map(params![deck], |row| row.get::<_, Id>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Refuse a card id that is not there, so a command never reports a write it did not make.
    fn require_card(&self, card: Id) -> Result<(), StoreError> {
        let found = self
            .conn
            .query_row(
                "SELECT 1 FROM card WHERE id = ?1",
                params![card],
                |_| Ok(()),
            )
            .optional()?;
        found.ok_or(StoreError::Missing {
            what: "card",
            id: card,
        })
    }

    /// The card's kind, or `text` for a card that is no longer there: a missing card is reported by
    /// the command that asked, not by this read.
    fn kind_of(&self, card: Id) -> Result<String, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT kind FROM card WHERE id = ?1",
                params![card],
                |row| row.get(0),
            )
            .optional()?
            .unwrap_or_else(|| "text".to_string()))
    }

    /// Whether the card is a diagram, and what it hides, in one read.
    fn shape_of(&self, card: Id) -> Result<Shape, StoreError> {
        let markdown: String = self.conn.query_row(
            "SELECT markdown FROM card WHERE id = ?1",
            params![card],
            |row| row.get(0),
        )?;
        Ok(hidden::shape(&markdown))
    }

    /// Put one rectangle in one group. A rectangle belongs to one group, so an existing row with
    /// the same corners is replaced rather than duplicated under a second group.
    fn place_mask(&self, card: Id, rect: MaskRect, group: u32) -> Result<usize, StoreError> {
        self.conn.execute(
            "DELETE FROM mask WHERE card_id = ?1 AND x = ?2 AND y = ?3 AND w = ?4 AND h = ?5",
            params![card, rect.x, rect.y, rect.w, rect.h],
        )?;
        self.conn.execute(
            "INSERT INTO mask (card_id, group_no, x, y, w, h) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![card, group, rect.x, rect.y, rect.w, rect.h],
        )?;
        Ok(1)
    }

    pub fn prompt_keys(&self, card: Id) -> Result<Vec<String>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT key FROM prompt WHERE card_id = ?1 ORDER BY key")?;
        let rows = stmt.query_map(params![card], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Give a card the prompts its shape asks for: `fwd` for two or more sides, one `cloze:<n>` per
    /// hidden-text group. A prompt that exists is left exactly as it was, so editing text never resets
    /// a schedule, and nothing is deleted: a group whose spans are gone keeps its history, is marked
    /// retired and stops queueing, and asking for it again clears the mark.
    fn sync_prompts(&self, card: Id, shape: &Shape) -> Result<(), StoreError> {
        // A two-sided card is asked from both ends, so `rev` is written beside the face keys and has
        // to appear in the claim set below: a key the shape does not claim is retired by this very
        // call, which would leave the reverse row unanswerable from the moment it was written.
        // Nothing here reads a reverse setting, so toggling reverse never writes a prompt.
        // A diagram card is asked about its groups instead, so its keys come from the mask table and
        // the text shape it carries (which is empty) adds nothing.
        let mut keys = shape.claimed_keys();
        if self.kind_of(card)? == "diagram" {
            keys.extend(
                self.mask_groups(card)?
                    .into_iter()
                    .map(|group| format!("mask:{group}")),
            );
        }
        for key in &keys {
            self.conn.execute(
                "INSERT INTO prompt (card_id, key, phase, due_at, interval_ms, reps, lapses)
                 VALUES (?1, ?2, 'learn', NULL, 0, 0, 0)
                 ON CONFLICT (card_id, key) DO NOTHING",
                params![card, key],
            )?;
        }
        // A key the shape no longer claims cannot be answered: masking it hides nothing, so it would
        // ask the question and print the answer. The row stays for its history.
        self.conn.execute(
            "UPDATE prompt SET retired = 1 WHERE card_id = ?1",
            params![card],
        )?;
        for key in &keys {
            self.conn.execute(
                "UPDATE prompt SET retired = 0 WHERE card_id = ?1 AND key = ?2",
                params![card, key],
            )?;
        }
        self.conn.execute(
            "UPDATE card SET has_cloze = ?2 WHERE id = ?1",
            params![card, i64::from(shape.hides_text())],
        )?;
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
    pub fn move_deck(
        &self,
        id: Id,
        parent: Option<Id>,
        now_ms: i64,
    ) -> Result<Outcome, StoreError> {
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
        if changed > 0 {
            // Every card under the moved deck may now inherit a different template, or none.
            for card in self.subtree_cards(id)? {
                self.rerender(card, now_ms)?;
            }
        }
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
        let outcome = FixedMultiplier.grade(&state, grade, at);
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

    /// Break the library so a snapshot read fails, for the worker's error path.
    #[cfg(test)]
    /// The raw connection, for tests that have to inspect what the schema actually wrote.
    #[cfg(test)]
    pub(crate) fn connection_for_test(&self) -> &Connection {
        &self.conn
    }

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

    /// How many real reviews a card has had: learning cycles are not reviews.
    pub fn review_count(&self, card: Id) -> Result<u32, StoreError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM review_log WHERE card_id = ?1 AND phase <> 'learn'",
            params![card],
            |row| row.get(0),
        )?;
        Ok(count_of(count))
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

    /// The cards a filter selects, for a cram run: the library's list, minus what cannot be asked.
    ///
    /// The filter is the snapshot's own, so the drill and the list it was started from can never
    /// disagree about what is selected. Only the schedule is left out: a cram run has none, which
    /// is the whole point of it.
    pub fn cards_for_view(&self, spec: &ViewSpec) -> Result<Vec<CardSummary>, StoreError> {
        let snapshot = self.snapshot()?;
        Ok(snapshot
            .cards_matching(spec)
            .into_iter()
            .filter(|card| card.prompts > 0)
            .cloned()
            .collect())
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

/// The form search and tag matching compare in: no case, no accents.
pub fn folded(text: &str) -> String {
    text.nfd()
        .filter(|character| !is_combining_mark(*character))
        .flat_map(char::to_lowercase)
        .collect()
}

/// The separator a view's tag set is stored with: ASCII unit separator, which a tag typed by hand
/// cannot hold, so no tag can be confused with two.
const TAG_SEPARATOR: char = '\u{1f}';

fn encode_tags(tags: &[String]) -> String {
    let separator = TAG_SEPARATOR.to_string();
    tags.iter()
        .map(|tag| tag.trim())
        .filter(|tag| !tag.is_empty())
        .collect::<Vec<_>>()
        .join(&separator)
}

fn decode_tags(text: &str) -> Vec<String> {
    text.split(TAG_SEPARATOR)
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(str::to_string)
        .collect()
}

fn mode_text(mode: CardMode) -> &'static str {
    match mode {
        CardMode::Active => "active",
        CardMode::Archived => "archived",
        CardMode::Trash => "trash",
    }
}

fn mode_parse(text: &str) -> CardMode {
    match text {
        "archived" => CardMode::Archived,
        "trash" => CardMode::Trash,
        _ => CardMode::Active,
    }
}

/// The first line of a card, cut to what a list row can show.
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

/// A stored count as the `u32` the API reports. More than four billion records cannot be held in
/// a `Snapshot` on any machine typr runs on, so the count saturates instead of wrapping.
/// A nullable integer flag, where NULL means "inherit".
fn flag(value: Option<i64>) -> Option<bool> {
    value.map(|value| value != 0)
}

fn count_of(value: i64) -> u32 {
    u32::try_from(value.max(0)).unwrap_or(u32::MAX)
}

/// A rectangle has to sit inside the image, or the fraction it stores would draw outside the pane
/// and a cover would land on nothing.
fn check_mask(card: Id, rect: MaskRect) -> Result<(), StoreError> {
    if rect.inside_the_image() {
        Ok(())
    } else {
        Err(StoreError::MaskOutsideImage { card })
    }
}

fn is_combining_mark(character: char) -> bool {
    matches!(
        character as u32,
        0x0300..=0x036F | 0x1AB0..=0x1AFF | 0x1DC0..=0x1DFF | 0x20D0..=0x20FF | 0xFE20..=0xFE2F
    )
}
