//! Storage target trait and attachment writer shared across transfer formats.

use std::path::Path;

use crate::command::{Command, Outcome};
use crate::error::StoreError;
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
