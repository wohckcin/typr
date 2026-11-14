//! Moving a library in and out.
//!
//! A `.typr` file is the library itself, copied by SQLite rather than rebuilt field by field, so
//! decks, cards, prompts, fields, masks and every review record arrive exactly as they were. The
//! other formats are for other tools: a Mochi archive to import, and CSV or Markdown to read and
//! edit outside the app.
//!
//! Nothing here opens the store's own connection. `export_library` opens a second read-only one
//! because `VACUUM INTO` cannot run inside a transaction; imports run through [`Db::apply`] on the
//! worker's connection, inside the transaction a command already holds.

use std::path::Path;

use rusqlite::{Connection, OpenFlags};

use crate::store::StoreError;

pub mod mochi;

/// What an import wrote, in the units the archive itself counts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ImportReport {
    pub decks: usize,
    pub cards: usize,
    pub reviews: usize,
    pub images: usize,
    pub masks: usize,
}

/// Copy the library at `db_path` into `target` without opening it for writing.
///
/// `VACUUM INTO` reads the source in one statement and writes a consistent database with no WAL
/// sidecar, so a library mid-session still exports as something that opens. The target must not
/// exist: overwriting the file a user picked would lose the previous export without asking.
pub fn export_library(db_path: &Path, target: &Path) -> Result<(), StoreError> {
    if target.exists() {
        return Err(StoreError::Exists {
            path: target.to_path_buf(),
        });
    }
    let source = Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|error| StoreError::Unreadable {
        path: db_path.to_path_buf(),
        source: error,
    })?;
    let text = target.to_str().ok_or_else(|| StoreError::Write {
        path: target.to_path_buf(),
        source: std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "the target path is not valid UTF-8",
        ),
    })?;
    source
        .execute("VACUUM INTO ?1", [text])
        .map_err(|error| StoreError::Export {
            path: target.to_path_buf(),
            source: error,
        })?;
    Ok(())
}
