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
