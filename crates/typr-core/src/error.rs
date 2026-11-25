//! Errors reported by the backend library when opening, validating, transferring, or writing.

use crate::model::Id;

/// What can go wrong opening or writing the store.
///
/// Every variant says what the caller can do about it and keeps its cause reachable through
/// [`std::error::Error::source`], so the UI can show one line while a log or a debugger can still
/// see the SQLite or `io` error underneath.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StoreError {
    /// The database refused a statement. The cause is reachable through `source()`, not repeated
    /// here, so a chain-aware description reads once.
    #[error("the database refused the statement")]
    Sqlite(#[from] rusqlite::Error),
    /// A file that could not be read as this app's library, named so the reason can be acted on.
    #[error("the library at {path} is unreadable: {source}")]
    Unreadable {
        path: std::path::PathBuf,
        #[source]
        source: rusqlite::Error,
    },
    /// A database written by a newer build: refused rather than downgraded.
    #[error(
        "this library was written by a newer version of typr (schema {found}, this build knows {supported})"
    )]
    TooNew { found: i64, supported: i64 },
    /// A grade for a prompt that does not exist, e.g. for a card with one side.
    #[error("card {card} has no prompt {key}")]
    NoPrompt { card: Id, key: String },
    /// A deck name another deck under the same parent already uses.
    #[error("{what} {id} does not exist")]
    Missing { what: &'static str, id: Id },
    #[error("a {what} called {name:?} already exists")]
    DuplicateName { what: &'static str, name: String },
    /// A name that is empty once trimmed.
    #[error("a {what} name cannot be blank")]
    BlankName { what: &'static str },
    /// A text card saved with nothing in it.
    #[error("card {card} has no text to save")]
    EmptyCard { card: Id },
    /// A move that would put a deck inside itself.
    #[error("deck {deck} cannot be moved inside itself")]
    Cycle { deck: Id },
    /// A schema version no build could have written: the file is not one of ours.
    #[error("this file records schema version {version}, which is not a typr library")]
    Corrupt { version: i64 },
    /// A value in the database that no build of typr writes.
    #[error("the library holds {value} in {column}, which typr never wrote")]
    Text { column: &'static str, value: String },
    /// A setting the store cannot honour.
    #[error("{value} is not a valid value for {key}")]
    Setting { key: String, value: String },
    /// A file operation on the library failed. The cause is reachable through `source()`.
    #[error("a file operation on the library failed")]
    Io(#[from] std::io::Error),
    /// A card that cannot change kind because it still holds what the other kind would hide.
    #[error("card {card} still holds {holds}, so it cannot change kind until that is cleared")]
    NotEmpty { card: Id, holds: &'static str },
    /// A mask rectangle that lies outside the normalized image bounds.
    #[error("a mask on card {card} does not sit inside its image")]
    MaskOutsideImage { card: Id },
    /// An export target that is already a file.
    #[error("the file {path} already exists, so it was not overwritten")]
    Exists { path: std::path::PathBuf },
    /// A file the transfer had to write, named so the reader can act on it.
    #[error("the file {path} could not be written: {source}")]
    Write {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// `VACUUM INTO` failed: the reason is SQLite's, the path is the target's.
    #[error("the export to {path} failed: {source}")]
    Export {
        path: std::path::PathBuf,
        #[source]
        source: rusqlite::Error,
    },
    /// A file the transfer had to read, named so the reader can act on it.
    #[error("the file {path} could not be read: {source}")]
    Read {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// A transfer file that cannot be understood, with the part that was wrong.
    #[error("the file {path} is not a Mochi export this build can read: {reason}")]
    Archive {
        path: std::path::PathBuf,
        reason: String,
    },
    #[error("the file {path} is not in the format this build reads: {reason}")]
    Malformed {
        path: std::path::PathBuf,
        reason: String,
    },
    /// A Mochi export whose format version this build does not know: refused before writing.
    #[error(
        "the Mochi export {path} records format version {version}, but this build reads version {supported}"
    )]
    ArchiveVersion {
        path: std::path::PathBuf,
        version: u64,
        supported: u64,
    },
    #[error("the library is closed")]
    Closed,
    /// The background library work panicked. The connection is suspect, so nothing restarts it.
    #[error("the library stopped: {message}")]
    WorkerPanicked { message: String },
}
