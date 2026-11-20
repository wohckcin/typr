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
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde::de::DeserializeOwned;
use zip::ZipArchive;

use crate::store::{Command, Db, Grade, Id, MaskRect, Outcome, StoreError};

use super::{ImportReport, attach_bytes};

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

/// The most one archive entry may inflate to. A zip header's size is the archive's own claim, so
/// the read is bounded here instead. 64 MiB is far above any real `cards.json` or picture, and
/// small enough that a zip bomb cannot exhaust memory.
const ENTRY_LIMIT: u64 = 64 * 1024 * 1024;

fn read_entry(entry: impl Read, name: &str, archive: &Path) -> Result<Vec<u8>, StoreError> {
    let mut bytes = Vec::new();
    entry
        .take(ENTRY_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| StoreError::Archive {
            path: archive.to_path_buf(),
            reason: format!("{name} could not be read: {error}"),
        })?;
    if bytes.len() as u64 > ENTRY_LIMIT {
        return Err(StoreError::Archive {
            path: archive.to_path_buf(),
            reason: format!("{name} is larger than the {} MiB limit", ENTRY_LIMIT >> 20),
        });
    }
    Ok(bytes)
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
    let bytes = read_entry(&mut entry, name, archive)?;
    serde_json::from_slice(&bytes).map_err(|error| StoreError::Archive {
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
    #[serde(default)]
    reviews: Vec<ArchiveReview>,
    /// A picture inside the archive (`media/...`), which makes this a diagram card.
    #[serde(default)]
    image: Option<String>,
    #[serde(default)]
    caption: String,
    #[serde(default)]
    masks: Vec<ArchiveMask>,
}

/// A mask rectangle in image fractions, so it lands where Mochi drew it whatever the picture's
/// pixel size.
#[derive(Deserialize)]
struct ArchiveMask {
    group: u32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

/// One graded review, named the way Mochi names the card's variants. A cloze deletion and a mask
/// group carry the group they belong to; forward and reverse do not.
#[derive(Deserialize)]
struct ArchiveReview {
    variant: ArchiveVariant,
    grade: ArchiveGrade,
    at: i64,
    #[serde(default)]
    duration_ms: Option<i64>,
    #[serde(default)]
    group: Option<u32>,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ArchiveVariant {
    Forward,
    Reverse,
    Cloze,
    Mask,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ArchiveGrade {
    Forgot,
    Remembered,
}

impl ArchiveGrade {
    const fn into_grade(self) -> Grade {
        match self {
            ArchiveGrade::Forgot => Grade::Forgot,
            ArchiveGrade::Remembered => Grade::Remembered,
        }
    }
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
/// the transaction rolls back and no deck, card or picture is left behind. `commit` runs last, so a
/// refused commit removes the pictures too.
pub(crate) fn import(
    db: &Db,
    archive: &Path,
    attachments: &Path,
    now_ms: i64,
    commit: impl FnOnce() -> Result<(), StoreError>,
) -> Result<ImportReport, StoreError> {
    let mut copied = Vec::new();
    let result = import_into(db, archive, attachments, now_ms, &mut copied)
        .and_then(|report| commit().map(|()| report));
    if result.is_err() {
        // The transaction is about to roll back its rows, so the pictures copied out of the
        // archive must not outlive the cards that would have named them.
        for path in copied {
            let _ = std::fs::remove_file(path);
        }
    }
    result
}

fn import_into(
    db: &Db,
    archive: &Path,
    attachments: &Path,
    now_ms: i64,
    copied: &mut Vec<PathBuf>,
) -> Result<ImportReport, StoreError> {
    let declared = manifest(archive)?;
    let mut zip = open(archive)?;
    let decks: Vec<ArchiveDeck> = read_json(&mut zip, "decks.json", archive)?;
    let cards: Vec<ArchiveCard> = read_json(&mut zip, "cards.json", archive)?;

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
            if deck_ids.insert(deck.id, id).is_some() {
                return Err(StoreError::Archive {
                    path: archive.to_path_buf(),
                    reason: format!("deck {} appears more than once", deck.id),
                });
            }
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
        let diagram = card
            .image
            .as_deref()
            .is_some_and(|image| !image.trim().is_empty());
        let outcome = if diagram {
            let image = card.image.as_deref().unwrap_or_default();
            let bytes = picture(&mut zip, image, archive)?;
            let name =
                attach_bytes(attachments, image, &bytes).map_err(|source| StoreError::Write {
                    path: attachments.join(image),
                    source,
                })?;
            copied.push(attachments.join(&name));
            report.images += 1;
            db.staged(
                &Command::CreateDiagram {
                    deck,
                    image: name,
                    caption: card.caption,
                },
                now_ms,
            )?
        } else {
            db.staged(
                &Command::CreateCard {
                    deck,
                    markdown: card.text,
                },
                now_ms,
            )?
        };
        let Outcome::Card(id) = outcome else {
            return Err(StoreError::Archive {
                path: archive.to_path_buf(),
                reason: "a card command did not return a card".to_string(),
            });
        };
        for mask in card.masks {
            db.staged(
                &Command::AddMask {
                    card: id,
                    group: mask.group,
                    rect: MaskRect::new(mask.x, mask.y, mask.w, mask.h),
                },
                now_ms,
            )?;
            report.masks += 1;
        }
        // A diagram has no text to render, so its template and fields (which a text card would
        // carry) have nothing to apply to; our own export leaves them out for diagrams.
        if !diagram && !card.fields.is_empty() {
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
        if !diagram && let Some(template) = card.template {
            db.staged(
                &Command::SetCardTemplate {
                    card: id,
                    template: Some(template),
                    // The archive names decks but not cards, so the imported card takes the name
                    // its text and this template imply.
                    name: None,
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
        // Replaying oldest first is what makes each record's phase and interval the ones a live
        // grade would have written: the scheduler reads the state the previous record left.
        let mut reviews = card.reviews;
        reviews.sort_by_key(|review| review.at);
        for review in reviews {
            let key = match review.variant {
                ArchiveVariant::Forward => "fwd".to_string(),
                ArchiveVariant::Reverse => "rev".to_string(),
                ArchiveVariant::Cloze => format!(
                    "cloze:{}",
                    review.group.ok_or_else(|| {
                        bad_review(archive, card.id, "a cloze review needs its group")
                    })?
                ),
                ArchiveVariant::Mask => format!(
                    "mask:{}",
                    review.group.ok_or_else(|| {
                        bad_review(archive, card.id, "a mask review needs its group")
                    })?
                ),
            };
            db.record_review(
                id,
                &key,
                review.grade.into_grade(),
                review.duration_ms,
                review.at,
            )?;
            report.reviews += 1;
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

fn picture(zip: &mut ZipArchive<File>, entry: &str, archive: &Path) -> Result<Vec<u8>, StoreError> {
    let mut file = zip.by_name(entry).map_err(|_| StoreError::Archive {
        path: archive.to_path_buf(),
        reason: format!("the archive holds no picture {entry:?}"),
    })?;
    read_entry(&mut file, &format!("picture {entry:?}"), archive)
}

fn bad_review(archive: &Path, card: i64, reason: &str) -> StoreError {
    StoreError::Archive {
        path: archive.to_path_buf(),
        reason: format!("card {card}: {reason}"),
    }
}
