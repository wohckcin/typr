//! Markdown, one file per deck.
//!
//! A file starts with `# <deck name>` and holds that deck's text cards separated by a blank line
//! pair. A card's own `---` and `{{…}}` are written verbatim, so importing the file reproduces the
//! same prompts rather than approximating them.
//!
//! The separator is the one thing Markdown cannot represent: a card that itself holds a blank line
//! would be cut in two on the way back. `.typr` and CSV carry those cards unchanged.

use std::path::{Path, PathBuf};

use crate::store::{Command, Db, Id, Outcome, StoreError};

use super::ImportReport;

/// Write one file per deck under `target`, and answer how many cards were written.
pub fn export(db: &Db, deck: Id, target: &Path) -> Result<usize, StoreError> {
    let snapshot = db.snapshot()?;
    if !snapshot.decks.iter().any(|row| row.id == deck) {
        return Err(StoreError::Missing {
            what: "deck",
            id: deck,
        });
    }
    std::fs::create_dir_all(target).map_err(|source| StoreError::Write {
        path: target.to_path_buf(),
        source,
    })?;
    let under = snapshot.subtree(Some(deck));
    let mut written = 0;
    for row in snapshot.decks.iter().filter(|row| under.contains(&row.id)) {
        let cards: Vec<&str> = snapshot
            .cards
            .iter()
            .filter(|card| {
                card.deck == row.id
                    && card.trashed_at.is_none()
                    && card.kind != crate::store::CardKind::Diagram
            })
            .map(|card| card.markdown.as_str())
            .collect();
        let mut body = format!("# {}\n\n", row.name);
        body.push_str(&cards.join("\n\n"));
        if !cards.is_empty() {
            body.push('\n');
        }
        let file = free_name(target, &file_name(&row.name));
        std::fs::write(&file, body).map_err(|source| StoreError::Write {
            path: file.clone(),
            source,
        })?;
        written += cards.len();
    }
    Ok(written)
}

/// Read `source` into `deck`.
///
/// The deck heading is provenance: an import lands in the deck its caller chose, so a file can be
/// moved between decks without editing it.
pub fn import(db: &Db, deck: Id, source: &Path, now_ms: i64) -> Result<ImportReport, StoreError> {
    let text = std::fs::read_to_string(source).map_err(|error| StoreError::Read {
        path: source.to_path_buf(),
        source: error,
    })?;
    let text = text
        .strip_prefix('\u{feff}')
        .unwrap_or(&text)
        .replace("\r\n", "\n");
    let body = match text.find('\n') {
        Some(end) if text[..end].trim_start().starts_with("# ") => &text[end + 1..],
        None if text.trim_start().starts_with("# ") => "",
        _ => text.as_str(),
    };

    let cards: Vec<&str> = body
        .split("\n\n")
        .map(|block| block.trim_matches(['\n', '\r']))
        .filter(|block| !block.trim().is_empty())
        .collect();
    if cards.is_empty() {
        return Err(StoreError::Malformed {
            path: source.to_path_buf(),
            reason: "the file holds no card".to_string(),
        });
    }

    let mut report = ImportReport::default();
    for markdown in cards {
        let outcome = db.staged(
            &Command::CreateCard {
                deck,
                markdown: markdown.to_string(),
            },
            now_ms,
        )?;
        if !matches!(outcome, Outcome::Card(_)) {
            return Err(StoreError::Malformed {
                path: source.to_path_buf(),
                reason: "a card command did not return a card".to_string(),
            });
        }
        report.cards += 1;
    }
    Ok(report)
}

/// A deck name that is safe as a file name on every desktop the app runs on.
fn file_name(deck: &str) -> String {
    let cleaned: String = deck
        .chars()
        .map(|character| {
            if character.is_control() || matches!(character, '/' | '\\' | ':') {
                '-'
            } else {
                character
            }
        })
        .collect();
    let trimmed = cleaned.trim().trim_matches('.');
    if trimmed.is_empty() {
        "deck.md".to_string()
    } else {
        format!("{trimmed}.md")
    }
}

/// The same rule the attachments folder uses: never overwrite a file an earlier export wrote.
fn free_name(dir: &Path, name: &str) -> PathBuf {
    let (stem, extension) = match name.rfind('.') {
        Some(dot) if dot > 0 => (&name[..dot], &name[dot..]),
        _ => (name, ""),
    };
    let mut candidate = dir.join(name);
    let mut copy = 1;
    while candidate.exists() {
        copy += 1;
        candidate = dir.join(format!("{stem}-{copy}{extension}"));
    }
    candidate
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_deck_name_cannot_open_a_folder() {
        assert_eq!(file_name("A/B: C"), "A-B- C.md");
        assert_eq!(file_name("  ..  "), "deck.md");
    }

    #[test]
    fn an_existing_file_is_not_overwritten() {
        let dir = tempfile::tempdir().expect("a temporary folder");
        std::fs::write(dir.path().join("Spanish.md"), "first").expect("a first export");
        let next = free_name(dir.path(), "Spanish.md");
        assert_eq!(
            next.file_name().and_then(|name| name.to_str()),
            Some("Spanish-2.md")
        );
    }
}
