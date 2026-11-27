//! Core SQLite handle, connection lifecycle, and shared column-encoding helpers.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};

use crate::error::StoreError;
use crate::model::{CardMode, Id, MaskRect};

use crate::store::schema::{self, SchemaFacts};

/// A record as a later build would have written it, for the extra keys a future version schedules.
#[cfg(test)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct Record {
    pub(crate) card: Id,
    pub(crate) key: &'static str,
    pub(crate) phase: crate::model::Phase,
    pub(crate) grade: crate::model::Grade,
    pub(crate) at: i64,
    pub(crate) interval_ms: i64,
    pub(crate) due_at: Option<i64>,
}

/// A database file, owned by exactly one thread.
#[derive(Debug)]
pub struct Db {
    pub(super) conn: Connection,
    schema: SchemaFacts,
    pub(super) path: PathBuf,
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

    /// Write a row whose name is unique, turning SQLite's refusal into something the reader
    /// understands: "a view called X already exists" is a sentence, the constraint is not.
    ///
    /// Only the extended code for a unique violation is translated: a foreign key or a check
    /// failure stays a database error, because it means the caller asked for something else.
    pub(super) fn named_write(
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
}

/// The separator a view's tag set is stored with: ASCII unit separator, which a tag typed by hand
/// cannot hold, so no tag can be confused with two.
const TAG_SEPARATOR: char = '\u{1f}';

pub(super) fn encode_tags(tags: &[String]) -> String {
    let separator = TAG_SEPARATOR.to_string();
    tags.iter()
        .map(|tag| tag.trim())
        .filter(|tag| !tag.is_empty())
        .collect::<Vec<_>>()
        .join(&separator)
}

pub(super) fn decode_tags(text: &str) -> Vec<String> {
    text.split(TAG_SEPARATOR)
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(str::to_string)
        .collect()
}

pub(super) fn mode_text(mode: CardMode) -> &'static str {
    match mode {
        CardMode::Active => "active",
        CardMode::Archived => "archived",
        CardMode::Trash => "trash",
    }
}

pub(super) fn mode_parse(text: &str) -> Result<CardMode, StoreError> {
    match text {
        "active" => Ok(CardMode::Active),
        "archived" => Ok(CardMode::Archived),
        "trash" => Ok(CardMode::Trash),
        _ => Err(StoreError::Text {
            column: "mode",
            value: text.to_owned(),
        }),
    }
}

/// A nullable integer flag, where NULL means "inherit".
pub(super) fn flag(value: Option<i64>) -> Option<bool> {
    value.map(|value| value != 0)
}

/// A stored count as the `u32` the API reports. More than four billion records cannot be held in
/// a `Snapshot` on any machine typr runs on, so the count saturates instead of wrapping.
pub(super) fn count_of(value: i64) -> u32 {
    u32::try_from(value.max(0)).unwrap_or(u32::MAX)
}

/// A rectangle has to sit inside the image, or the fraction it stores would draw outside the pane
/// and a cover would land on nothing.
pub(super) fn check_mask(card: Id, rect: MaskRect) -> Result<(), StoreError> {
    if rect.inside_the_image() {
        Ok(())
    } else {
        Err(StoreError::MaskOutsideImage { card })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::CardKind;

    #[test]
    fn an_unknown_mode_is_refused() {
        assert!(matches!(
            mode_parse("frozen"),
            Err(StoreError::Text { column: "mode", .. })
        ));
        assert_eq!(mode_parse("trash").unwrap(), CardMode::Trash);
    }

    #[test]
    fn an_unknown_kind_is_refused() {
        assert!(matches!(
            CardKind::parse("video"),
            Err(StoreError::Text { column: "kind", .. })
        ));
    }
}
