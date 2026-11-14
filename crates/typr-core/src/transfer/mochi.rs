//! Reading a Mochi export.
//!
//! A `.mochi` file is a zip of JSON. `manifest.json` is read first and says which format version
//! the rest is in and how many rows it claims; an archive whose version this build does not know
//! is refused before a single deck is created, which is what makes "refused whole" possible.
//!
//! The rest of the archive is three arrays: `decks.json`, `cards.json`, and the pictures the cards
//! name, stored under `media/`. Decks point at their parent by id, cards point at their deck the
//! same way, and a card's reviews name the variant they graded — this is the shape the importer
//! reads, documented in the README so another tool can write one.

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use serde::Deserialize;
use serde::de::DeserializeOwned;
use zip::ZipArchive;

use crate::store::{Command, Db, Id, Outcome, StoreError};

use super::ImportReport;

/// The `format` field every Mochi manifest this build reads must name.
pub const FORMAT: &str = "mochi";

/// The format version this build writes and understands.
pub const VERSION: u64 = 1;

const MANIFEST: &str = "manifest.json";
/// What a manifest declares before the importer writes anything.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    pub version: u64,
    pub tables: Vec<String>,
    pub counts: Counts,
}

/// How many rows the archive claims, checked against what was written.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub decks: usize,
    pub cards: usize,
    pub reviews: usize,
    pub images: usize,
    pub masks: usize,
}

#[derive(Deserialize)]
struct RawManifest {
    format: String,
    version: u64,
    #[serde(default)]
    tables: Vec<String>,
    counts: RawCounts,
}

#[derive(Deserialize, Default)]
struct RawCounts {
    #[serde(default)]
    decks: usize,
    #[serde(default)]
    cards: usize,
    #[serde(default)]
    reviews: usize,
    #[serde(default)]
    images: usize,
    #[serde(default)]
    masks: usize,
}

/// Read an archive's manifest, refusing a format or version this build does not know.
///
/// The zip's central directory is read first, so a truncated file fails here — before the caller
/// has opened a transaction or copied a picture.
pub fn manifest(archive: &Path) -> Result<Manifest, StoreError> {
    let mut zip = open(archive)?;
    let raw: RawManifest = read_json(&mut zip, MANIFEST, archive)?;
    if raw.format != FORMAT {
        return Err(StoreError::Archive {
            path: archive.to_path_buf(),
            reason: format!("{MANIFEST} names format {:?}, not {:?}", raw.format, FORMAT),
        });
    }
    if raw.version != VERSION {
        return Err(StoreError::ArchiveVersion {
            path: archive.to_path_buf(),
            version: raw.version,
            supported: VERSION,
        });
    }
    for table in ["decks", "cards"] {
        if !raw.tables.iter().any(|name| name == table) {
            return Err(StoreError::Archive {
                path: archive.to_path_buf(),
                reason: format!("{MANIFEST} does not list the {table:?} table"),
            });
        }
    }
    Ok(Manifest {
        version: raw.version,
        tables: raw.tables,
        counts: Counts {
            decks: raw.counts.decks,
            cards: raw.counts.cards,
            reviews: raw.counts.reviews,
            images: raw.counts.images,
            masks: raw.counts.masks,
        },
    })
}

/// Open an archive, naming the file when it is not a zip at all.
fn open(archive: &Path) -> Result<ZipArchive<File>, StoreError> {
    let file = File::open(archive).map_err(|source| StoreError::Read {
        path: archive.to_path_buf(),
        source,
    })?;
    ZipArchive::new(file).map_err(|error| StoreError::Archive {
        path: archive.to_path_buf(),
        reason: error.to_string(),
    })
}

/// Read one JSON entry, naming the entry when it is missing or malformed.
fn read_json<T: DeserializeOwned>(
    zip: &mut ZipArchive<File>,
    name: &str,
    archive: &Path,
) -> Result<T, StoreError> {
    let mut entry = zip.by_name(name).map_err(|error| StoreError::Archive {
        path: archive.to_path_buf(),
        reason: format!("no {name}: {error}"),
    })?;
    let mut text = String::new();
    entry
        .read_to_string(&mut text)
        .map_err(|error| StoreError::Archive {
            path: archive.to_path_buf(),
            reason: format!("{name} could not be read: {error}"),
        })?;
    serde_json::from_str(&text).map_err(|error| StoreError::Archive {
        path: archive.to_path_buf(),
        reason: format!("{name} is not valid JSON: {error}"),
    })
}

/// One deck as the archive names it: its own id, a name, and the id of its parent.
#[derive(Deserialize)]
struct ArchiveDeck {
    id: i64,
    name: String,
    #[serde(default)]
    parent: Option<i64>,
}

/// One card. Everything but `id`, `deck` and `text` may be absent, so a hand-written archive can
/// leave out what it does not use.
#[derive(Deserialize)]
struct ArchiveCard {
    id: i64,
    deck: i64,
    #[serde(default)]
    text: String,
    /// A `<<Field>>` template, stored on the card so the import does not change the deck every
    /// card inherits it from.
    #[serde(default)]
    template: Option<String>,
    #[serde(default)]
    fields: Vec<ArchiveField>,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Deserialize)]
struct ArchiveField {
    name: String,
    value: String,
}

/// Write every deck and card in `archive` through `db`'s command bodies, inside the transaction
/// `Db::apply` already holds, and report how many of each were written.
///
/// Decks are placed parent-first, so nesting survives even though the archive lists a child before
/// its parent. A record that names something the archive does not hold fails the whole import:
/// the transaction rolls back and no deck, card or picture is left behind.
pub(crate) fn import(
    db: &Db,
    archive: &Path,
    _attachments: &Path,
    now_ms: i64,
) -> Result<ImportReport, StoreError> {
    let declared = manifest(archive)?;
    let mut zip = open(archive)?;
    let decks: Vec<ArchiveDeck> = read_json(&mut zip, "decks.json", archive)?;
    let cards: Vec<ArchiveCard> = read_json(&mut zip, "cards.json", archive)?;
    drop(zip);

    let mut report = ImportReport::default();
    let mut deck_ids: HashMap<i64, Id> = HashMap::new();
    let mut pending = decks;
    while !pending.is_empty() {
        let mut waiting = Vec::new();
        let mut placed = 0;
        for deck in pending.drain(..) {
            let parent = match deck.parent {
                None => None,
                Some(parent) => match deck_ids.get(&parent) {
                    Some(id) => Some(*id),
                    None => {
                        waiting.push(deck);
                        continue;
                    }
                },
            };
            let Outcome::Deck(id) = db.staged(
                &Command::CreateDeck {
                    parent,
                    name: deck.name,
                },
                now_ms,
            )?
            else {
                return Err(StoreError::Archive {
                    path: archive.to_path_buf(),
                    reason: "a deck command did not return a deck".to_string(),
                });
            };
            deck_ids.insert(deck.id, id);
            placed += 1;
        }
        if placed == 0 {
            return Err(StoreError::Archive {
                path: archive.to_path_buf(),
                reason: format!(
                    "deck {} names a parent the archive does not hold, or the deck nest loops",
                    waiting[0].id
                ),
            });
        }
        report.decks += placed;
        pending = waiting;
    }

    for card in cards {
        let Some(&deck) = deck_ids.get(&card.deck) else {
            return Err(StoreError::Archive {
                path: archive.to_path_buf(),
                reason: format!("card {} names deck {}", card.id, card.deck),
            });
        };
        let Outcome::Card(id) = db.staged(
            &Command::CreateCard {
                deck,
                markdown: card.text,
            },
            now_ms,
        )?
        else {
            return Err(StoreError::Archive {
                path: archive.to_path_buf(),
                reason: "a card command did not return a card".to_string(),
            });
        };
        if !card.fields.is_empty() {
            db.staged(
                &Command::SetFields {
                    card: id,
                    fields: card
                        .fields
                        .into_iter()
                        .map(|field| (field.name, field.value))
                        .collect(),
                },
                now_ms,
            )?;
        }
        if let Some(template) = card.template {
            db.staged(
                &Command::SetCardTemplate {
                    card: id,
                    template: Some(template),
                },
                now_ms,
            )?;
        }
        if !card.tags.is_empty() {
            db.staged(
                &Command::SetTags {
                    card: id,
                    tags: card.tags,
                },
                now_ms,
            )?;
        }
        report.cards += 1;
    }

    let written = Counts {
        decks: report.decks,
        cards: report.cards,
        reviews: report.reviews,
        images: report.images,
        masks: report.masks,
    };
    if written != declared.counts {
        return Err(StoreError::Archive {
            path: archive.to_path_buf(),
            reason: format!(
                "the manifest declares {declared:?}, but the archive holds {written:?}"
            ),
        });
    }
    Ok(report)
}
