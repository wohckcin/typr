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

use crate::command::{Command, Outcome};
use crate::error::StoreError;
pub use crate::model::ImportReport;
use crate::model::{Grade, Id, Snapshot};

/// Minimal storage operations needed by the transfer importers and exporters so `transfer` stays
/// decoupled from `store::Db`.
pub trait TransferTarget {
    fn snapshot(&self) -> Result<Snapshot, StoreError>;
    fn staged(&self, command: &Command, now_ms: i64) -> Result<Outcome, StoreError>;
    fn record_review(
        &self,
        card: Id,
        key: &str,
        grade: Grade,
        duration_ms: Option<i64>,
        at_ms: i64,
    ) -> Result<Outcome, StoreError>;
}

/// Distinguishes the temporary names of exports racing in one process.
static NEXT_TEMP: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

mod csv;
mod markdown;
pub mod mochi;

pub use csv::{export as export_csv, import as import_csv};
pub use markdown::{export as export_markdown, import as import_markdown};

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
    let invalid = || StoreError::Write {
        path: target.to_path_buf(),
        source: std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "the target path is not valid UTF-8",
        ),
    };
    let parent = target
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let name = target.file_name().ok_or_else(invalid)?;
    // `VACUUM INTO` needs a name that does not exist yet, so each export writes its own hidden
    // neighbour and publishes it with a hard link. The link is the check and the write in one
    // step: two exports racing to the same new path both vacuum, but only one can take the name.
    let temp = parent.join(format!(
        ".{}.{}.{}.tmp",
        name.to_string_lossy(),
        std::process::id(),
        NEXT_TEMP.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
    ));
    let text = temp.to_str().ok_or_else(invalid)?;
    let source = Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|error| StoreError::Unreadable {
        path: db_path.to_path_buf(),
        source: error,
    })?;
    if let Err(error) = source.execute("VACUUM INTO ?1", [text]) {
        let _ = std::fs::remove_file(&temp);
        return Err(StoreError::Export {
            path: target.to_path_buf(),
            source: error,
        });
    }
    // FAT and exFAT have no hard links, so any other failure falls back to a copy that is still
    // refused when the name is taken.
    let linked = std::fs::hard_link(&temp, target).or_else(|error| {
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            Err(error)
        } else {
            copy_new(&temp, target)
        }
    });
    let _ = std::fs::remove_file(&temp);
    linked.map_err(|error| {
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            StoreError::Exists {
                path: target.to_path_buf(),
            }
        } else {
            StoreError::Write {
                path: target.to_path_buf(),
                source: error,
            }
        }
    })
}

/// Copy `from` to a `target` that must not exist yet. `create_new` makes the check and the create
/// one step, so a racing export still cannot take the same name.
fn copy_new(from: &Path, target: &Path) -> std::io::Result<()> {
    let mut out = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)?;
    let copied =
        std::fs::File::open(from).and_then(|mut source| std::io::copy(&mut source, &mut out));
    if copied.is_err() {
        let _ = std::fs::remove_file(target);
    }
    copied.map(drop)
}

/// Copy `source` into `root` under a name that is not taken, and answer the reference to write into
/// the card.
///
/// The file keeps its name; a name already in use gets `-2`, `-3` before the extension, so a
/// second import of the same picture never overwrites the copy a card already points at.
pub fn attach(root: &Path, source: &Path) -> std::io::Result<String> {
    let name = source
        .file_name()
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "the picture has no file name",
            )
        })?
        .to_string_lossy()
        .into_owned();
    let bytes = std::fs::read(source)?;
    write_attachment(root, &name, &bytes)
}

/// [`attach`] for bytes that never existed as a file, which is how a picture inside a `.mochi`
/// archive is copied out. Only the last component of `name` is used, so a reference cannot point
/// the copy outside `root`.
pub fn attach_bytes(root: &Path, name: &str, bytes: &[u8]) -> std::io::Result<String> {
    let file_name = Path::new(name).file_name().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "the picture has no file name",
        )
    })?;
    write_attachment(root, &file_name.to_string_lossy(), bytes)
}

fn write_attachment(root: &Path, name: &str, bytes: &[u8]) -> std::io::Result<String> {
    std::fs::create_dir_all(root)?;
    // A markdown destination cannot hold `<` or `>`, so the copy is named without them.
    let name = name.replace(['<', '>'], "_");
    let name = name.as_str();
    let (stem, extension) = match name.rfind('.') {
        Some(dot) if dot > 0 => (&name[..dot], &name[dot..]),
        _ => (name, ""),
    };
    let mut candidate = name.to_string();
    let mut copy = 1;
    while root.join(&candidate).exists() {
        copy += 1;
        candidate = format!("{stem}-{copy}{extension}");
    }
    std::fs::write(root.join(&candidate), bytes)?;
    Ok(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_copy_fallback_never_overwrites_a_target() {
        let dir = tempfile::tempdir().expect("a temporary folder");
        let temp = dir.path().join("temp");
        std::fs::write(&temp, "new").expect("a temp export");
        let free = dir.path().join("free");
        copy_new(&temp, &free).expect("a free name is taken");
        assert_eq!(std::fs::read_to_string(&free).expect("the copy"), "new");

        let taken = dir.path().join("taken");
        std::fs::write(&taken, "old").expect("an earlier export");
        let error = copy_new(&temp, &taken).expect_err("a taken name is refused");
        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
        assert_eq!(std::fs::read_to_string(&taken).expect("untouched"), "old");
    }
}
