//! Whole-library copies and the formats a library moves through.

use std::io::Write;
use std::path::Path;

use typr_core::store::{Command, Db, Grade, Id, Outcome, StoreError};
use typr_core::transfer;
use zip::write::{SimpleFileOptions, ZipWriter};

const NOW: i64 = 1_768_089_600_000 + 13 * 60 * 60 * 1000;
const HOUR: i64 = 60 * 60 * 1000;

fn open(path: &Path) -> Db {
    Db::open(path).expect("a fresh library")
}

fn deck(db: &mut Db, parent: Option<Id>, name: &str) -> Id {
    match db.apply(
        &Command::CreateDeck {
            parent,
            name: name.to_string(),
        },
        NOW,
    ) {
        Ok(Outcome::Deck(id)) => id,
        other => panic!("creating a deck: {other:?}"),
    }
}

fn card(db: &mut Db, deck: Id, markdown: &str) -> Id {
    match db.apply(
        &Command::CreateCard {
            deck,
            markdown: markdown.to_string(),
        },
        NOW,
    ) {
        Ok(Outcome::Card(id)) => id,
        other => panic!("creating a card: {other:?}"),
    }
}

fn tags(db: &mut Db, card: Id, tags: &[&str]) {
    db.apply(
        &Command::SetTags {
            card,
            tags: tags.iter().map(|tag| (*tag).to_string()).collect(),
        },
        NOW,
    )
    .expect("tags");
}

/// A library with the parts an export must not lose: nested decks, a template with fields, a
/// hidden-text card and three review records.
fn fixture(db: &mut Db) -> (Id, Id, Id, Id) {
    let spanish = deck(db, None, "Spanish");
    let verbs = deck(db, Some(spanish), "Verbs");
    let two_sided = card(db, verbs, "<<Word>>\n---\n<<Meaning>>");
    db.apply(
        &Command::SetFields {
            card: two_sided,
            fields: vec![
                ("Word".to_string(), "hablar".to_string()),
                ("Meaning".to_string(), "to speak".to_string()),
            ],
        },
        NOW,
    )
    .expect("fields");
    db.apply(
        &Command::SetCardTemplate {
            card: two_sided,
            template: Some("<<Word>>\n---\n<<Meaning>>".to_string()),
        },
        NOW,
    )
    .expect("template");
    tags(db, two_sided, &["verb", "regular"]);

    let note = card(db, spanish, "The accent falls on the second syllable.");
    let cloze = card(db, spanish, "hablo {{1::español}} and {{2::inglés}}");
    tags(db, cloze, &["language"]);

    for (card, key, grade, at) in [
        (two_sided, "fwd", Grade::Remembered, NOW),
        (two_sided, "fwd", Grade::Remembered, NOW + HOUR),
        (two_sided, "rev", Grade::Forgot, NOW + 2 * HOUR),
        (cloze, "cloze:1", Grade::Remembered, NOW + 3 * HOUR),
    ] {
        db.apply(
            &Command::Grade {
                card,
                key: key.to_string(),
                grade,
                duration_ms: Some(1_500),
            },
            at,
        )
        .expect("grade");
    }

    (spanish, verbs, note, cloze)
}

#[test]
fn the_export_reopens_with_every_record() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let source = dir.path().join("typr.sqlite3");
    let mut db = open(&source);
    fixture(&mut db);
    let before = db.snapshot().expect("the library");
    assert_eq!(before.decks.len(), 2);
    assert_eq!(before.cards.len(), 3);

    let target = dir.path().join("library.typr");
    transfer::export_library(&source, &target).expect("the export");
    assert!(target.is_file(), "the export names one file");

    let reopened = open(&target);
    let after = reopened.snapshot().expect("the exported library");

    assert_eq!(before.decks, after.decks, "deck names and nesting survive");
    assert_eq!(before.cards, after.cards, "cards, fields and tags survive");
    assert_eq!(before.views, after.views);
    assert_eq!(before.settings, after.settings);

    for card in &before.cards {
        let id = card.id;
        assert_eq!(
            db.prompt_keys(id).expect("prompts"),
            reopened.prompt_keys(id).expect("prompts"),
            "card {id} keeps the prompts its shape asks for"
        );
        assert_eq!(
            db.history(id).expect("history"),
            reopened.history(id).expect("history"),
            "card {id} keeps every record and its time"
        );
    }
    assert_eq!(
        db.stats(0, i64::MAX).expect("stats"),
        reopened.stats(0, i64::MAX).expect("stats")
    );
}

#[test]
fn exporting_leaves_the_library_untouched() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let source = dir.path().join("typr.sqlite3");
    let mut db = open(&source);
    fixture(&mut db);

    let before = std::fs::metadata(&source)
        .expect("the library file")
        .modified()
        .expect("a modification time");
    let facts = db.schema_facts();

    transfer::export_library(&source, &dir.path().join("copy.typr")).expect("the export");

    let after = std::fs::metadata(&source)
        .expect("the library file")
        .modified()
        .expect("a modification time");
    assert_eq!(before, after, "exporting does not write to the library");
    assert_eq!(facts, db.schema_facts(), "exporting does not migrate it");
}

#[test]
fn an_existing_target_is_refused() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let source = dir.path().join("typr.sqlite3");
    let mut db = open(&source);
    fixture(&mut db);

    let target = dir.path().join("copy.typr");
    std::fs::write(&target, b"an older export").expect("an existing file");
    let error = transfer::export_library(&source, &target).expect_err("the refusal");
    assert!(matches!(error, StoreError::Exists { path } if path == target));
    assert_eq!(
        std::fs::read(&target).expect("the existing file"),
        b"an older export",
        "the refused export left the old file alone"
    );
}

/// Write a minimal archive with the three files every Mochi export carries.
fn write_archive(path: &Path, manifest: &str, decks: &str, cards: &str, media: &[(&str, &[u8])]) {
    let file = std::fs::File::create(path).expect("an archive");
    let mut zip = ZipWriter::new(file);
    for (name, body) in [
        ("manifest.json", manifest.as_bytes()),
        ("decks.json", decks.as_bytes()),
        ("cards.json", cards.as_bytes()),
    ] {
        zip.start_file(name, SimpleFileOptions::default())
            .expect("a zip entry");
        zip.write_all(body).expect("a written entry");
    }
    for (name, body) in media {
        zip.start_file(*name, SimpleFileOptions::default())
            .expect("a zip entry");
        zip.write_all(body).expect("a written entry");
    }
    zip.finish().expect("a finished archive");
}

#[test]
fn a_newer_export_version_is_refused() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let archive = dir.path().join("future.mochi");
    write_archive(
        &archive,
        r#"{"format":"mochi","version":2,"tables":["decks","cards"],"counts":{}}"#,
        "[]",
        "[]",
        &[],
    );

    let error = transfer::mochi::manifest(&archive).expect_err("the refusal");
    assert!(
        matches!(
            error,
            StoreError::ArchiveVersion {
                version: 2,
                supported: 1,
                ..
            }
        ),
        "the refusal names both versions: {error}"
    );
}

#[test]
fn a_truncated_archive_is_refused_before_any_write() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let source = dir.path().join("typr.sqlite3");
    let mut db = open(&source);
    fixture(&mut db);
    let before = db.snapshot().expect("the library").cards.len();

    let archive = dir.path().join("cut.mochi");
    write_archive(
        &archive,
        r#"{"format":"mochi","version":1,"tables":["decks","cards"],"counts":{"decks":1,"cards":1}}"#,
        "[]",
        "[]",
        &[],
    );
    let bytes = std::fs::read(&archive).expect("the archive");
    std::fs::write(&archive, &bytes[..bytes.len() / 2]).expect("a truncated archive");

    assert!(
        transfer::mochi::manifest(&archive).is_err(),
        "a cut archive has no usable central directory"
    );
    assert_eq!(
        db.snapshot().expect("the library").cards.len(),
        before,
        "reading the manifest wrote nothing"
    );
}
