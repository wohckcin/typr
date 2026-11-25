//! Where the library, its attachments and the app's folders live.
//!
//! The backend takes a path and writes what it is given; deciding which directory that is belongs to
//! the app, which is the only layer that knows the platform's conventions. Native Shell answers the
//! app's data folder, and `TYPR_DATA_DIR` overrides it, which is how a test or an isolated run keeps
//! out of the real one.

use std::fmt;
use std::path::{Path, PathBuf};

use nsh_core::prelude::data_dir;
use typr_core::clock::{Clock, SystemClock};
use typr_core::store::{Command, Db, StoreError};

/// Why the desktop app could not open its library at startup.
#[derive(Debug)]
#[non_exhaustive]
pub enum OpenError {
    /// No home or platform application-data folder was available.
    NoDataDir,
    /// Creating the app's data or attachments folder failed.
    Io(std::io::Error),
    /// Opening, migrating or seeding the SQLite library failed.
    Store(StoreError),
}

impl fmt::Display for OpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpenError::NoDataDir => f.write_str("no home folder to keep the library in"),
            OpenError::Io(_) => f.write_str("could not prepare the library folder"),
            OpenError::Store(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for OpenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            OpenError::NoDataDir => None,
            OpenError::Io(source) => Some(source),
            OpenError::Store(source) => source.source(),
        }
    }
}

impl From<std::io::Error> for OpenError {
    fn from(source: std::io::Error) -> Self {
        OpenError::Io(source)
    }
}

impl From<StoreError> for OpenError {
    fn from(source: StoreError) -> Self {
        OpenError::Store(source)
    }
}

/// The library's file inside the app's data folder.
pub fn database_path(root: &Path) -> PathBuf {
    root.join("typr.sqlite3")
}

/// Where a card's attached images are copied.
pub fn attachments_root(root: &Path) -> PathBuf {
    root.join("attachments")
}

/// Where the app writes its log.
pub fn logs_root(root: &Path) -> PathBuf {
    root.join("logs")
}

/// The app's data folder: `TYPR_DATA_DIR` when set, else the platform's for this app.
pub fn data_root() -> Result<PathBuf, OpenError> {
    if let Some(root) = std::env::var_os("TYPR_DATA_DIR") {
        return Ok(PathBuf::from(root));
    }
    data_dir().ok_or(OpenError::NoDataDir)
}

/// Open the library from the process environment, creating its folders if needed.
pub fn open_from_environment() -> Result<Db, OpenError> {
    let root = data_root()?;
    std::fs::create_dir_all(&root)?;
    std::fs::create_dir_all(attachments_root(&root))?;
    let db = Db::open(&database_path(&root))?;
    // A library with no deck has nowhere to put its first card, so the empty grid's one action
    // would refuse. A new library gets the deck that action needs; a library that has decks is
    // left alone, archived ones included.
    if db.snapshot()?.decks.is_empty() {
        for name in ["Flashcards", "Notes"] {
            let first = Command::CreateDeck {
                parent: None,
                name: name.to_owned(),
            };
            db.apply(&first, SystemClock.now_ms())?;
            log::info!("library was empty, so it starts with the deck {first:?} made");
        }
    }
    // What a report needs to tell a migrated library from a refused or unopened one.
    let facts = db.schema_facts();
    log::info!(
        "library {} schema {} -> {}, {} migrations",
        db.path().display(),
        facts.found,
        facts.written,
        facts.applied
    );
    Ok(db)
}
