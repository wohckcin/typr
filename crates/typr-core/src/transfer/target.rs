//! Storage target trait and attachment writer shared across transfer formats.

use std::io::Write;
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
pub fn attach(root: &Path, source: &Path) -> Result<String, StoreError> {
    let read = |source_error| StoreError::Read {
        path: source.to_path_buf(),
        source: source_error,
    };
    let name = source
        .file_name()
        .ok_or_else(|| {
            read(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "the picture has no file name",
            ))
        })?
        .to_string_lossy()
        .into_owned();
    let bytes = std::fs::read(source).map_err(read)?;
    write_attachment(root, &name, &bytes).map_err(|error| StoreError::Write {
        path: root.join(&name),
        source: error,
    })
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
    loop {
        // `create_new` makes the check and the create one step, so a racing import cannot take the
        // same name.
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(root.join(&candidate))
        {
            Ok(mut file) => {
                if let Err(error) = file.write_all(bytes) {
                    let _ = std::fs::remove_file(root.join(&candidate));
                    return Err(error);
                }
                return Ok(candidate);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                copy += 1;
                candidate = format!("{stem}-{copy}{extension}");
            }
            Err(error) => return Err(error),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concurrent_attachments_never_share_a_name() {
        let dir = tempfile::tempdir().expect("a temporary folder");
        let root = dir.path().to_path_buf();
        let names: Vec<String> = std::thread::scope(|s| {
            let handles: Vec<_> = (0..16u8)
                .map(|i| {
                    let root = &root;
                    s.spawn(move || attach_bytes(root, "a.png", &[i]).expect("an attachment"))
                })
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().expect("a thread"))
                .collect()
        });
        let unique: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(unique.len(), 16);
        for name in &names {
            assert_eq!(std::fs::read(root.join(name)).expect("a copy").len(), 1);
        }
    }

    #[test]
    fn a_missing_source_is_named_in_the_error() {
        let dir = tempfile::tempdir().expect("a temporary folder");
        let error = attach(dir.path(), Path::new("/nope/cat.png")).expect_err("no such file");
        assert!(error.to_string().contains("/nope/cat.png"), "{error}");
    }
}
