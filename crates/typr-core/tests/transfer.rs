//! Whole-library copies and the formats a library moves through.

use std::io::Write;
use std::path::Path;

use typr_core::store::{CardKind, Command, Db, Grade, Id, Outcome, StoreError};
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

/// Write an archive whose manifest counts are given by the caller, so a test can also claim
/// something the archive does not hold.
fn write_mochi(
    dir: &Path,
    name: &str,
    decks: &serde_json::Value,
    cards: &serde_json::Value,
    counts: serde_json::Value,
) -> std::path::PathBuf {
    write_mochi_media(dir, name, decks, cards, counts, &[])
}

fn write_mochi_media(
    dir: &Path,
    name: &str,
    decks: &serde_json::Value,
    cards: &serde_json::Value,
    counts: serde_json::Value,
    media: &[(&str, &[u8])],
) -> std::path::PathBuf {
    let archive = dir.join(name);
    let manifest = serde_json::json!({
        "format": "mochi",
        "version": 1,
        "tables": ["decks", "cards"],
        "counts": counts,
    });
    write_archive(
        &archive,
        &manifest.to_string(),
        &decks.to_string(),
        &cards.to_string(),
        media,
    );
    archive
}

#[test]
fn three_nested_decks_and_forty_cards_import_whole() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let decks = serde_json::json!([
        {"id": 1, "name": "Spanish", "parent": null},
        {"id": 2, "name": "Verbs", "parent": 1},
        {"id": 3, "name": "Irregular", "parent": 2},
    ]);
    let cards: Vec<_> = (0..40)
        .map(|i| {
            serde_json::json!({
                "id": 100 + i,
                "deck": if i % 2 == 0 { 2 } else { 3 },
                "text": format!("front {i}\n---\nback {i}"),
            })
        })
        .collect();
    let archive = write_mochi(
        dir.path(),
        "nested.mochi",
        &decks,
        &serde_json::json!(cards),
        serde_json::json!({"decks": 3, "cards": 40}),
    );

    let db_path = dir.path().join("typr.sqlite3");
    let db = open(&db_path);
    let outcome = db
        .apply(
            &Command::ImportArchive {
                archive,
                attachments: dir.path().join("media"),
            },
            NOW,
        )
        .expect("the import");
    assert_eq!(
        outcome,
        Outcome::Imported(typr_core::transfer::ImportReport {
            decks: 3,
            cards: 40,
            ..Default::default()
        })
    );

    let snapshot = db.snapshot().expect("the library");
    assert_eq!(snapshot.decks.len(), 3);
    assert_eq!(snapshot.cards.len(), 40);
    let named = |name: &str| {
        snapshot
            .decks
            .iter()
            .find(|deck| deck.name == name)
            .unwrap_or_else(|| panic!("deck {name} imported"))
    };
    let spanish = named("Spanish");
    let verbs = named("Verbs");
    let irregular = named("Irregular");
    assert_eq!(spanish.parent, None);
    assert_eq!(verbs.parent, Some(spanish.id));
    assert_eq!(irregular.parent, Some(verbs.id), "nesting is parent-first");
    assert!(
        snapshot
            .cards
            .iter()
            .all(|card| card.markdown.contains("front "))
    );
}

#[test]
fn an_imported_card_renders_through_its_template() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let template = "<<Word>>\n---\n<<Meaning>>";
    let decks = serde_json::json!([{"id": 1, "name": "Spanish", "parent": null}]);
    let cards = serde_json::json!([{
        "id": 7,
        "deck": 1,
        "text": "gato\n---\ncat",
        "template": template,
        "fields": [
            {"name": "Word", "value": "gato"},
            {"name": "Meaning", "value": "cat"},
        ],
        "tags": ["animal"],
    }]);
    let archive = write_mochi(
        dir.path(),
        "template.mochi",
        &decks,
        &cards,
        serde_json::json!({"decks": 1, "cards": 1}),
    );

    let db_path = dir.path().join("typr.sqlite3");
    let db = open(&db_path);
    db.apply(
        &Command::ImportArchive {
            archive,
            attachments: dir.path().join("media"),
        },
        NOW,
    )
    .expect("the import");

    let snapshot = db.snapshot().expect("the library");
    let card = &snapshot.cards[0];
    assert_eq!(
        card.markdown, "gato\n---\ncat",
        "the template rendered the text"
    );
    assert_eq!(card.sides, 2);
    assert_eq!(card.template.as_deref(), Some(template));
    assert_eq!(
        card.fields,
        [
            ("Meaning".to_string(), "cat".to_string()),
            ("Word".to_string(), "gato".to_string())
        ],
        "fields are read back in name order"
    );
    assert_eq!(card.tags, ["animal"]);
    assert_eq!(db.prompt_keys(card.id).expect("prompts"), ["fwd", "rev"]);
}

#[test]
fn duplicate_deck_names_are_refused_rather_than_merged() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let decks = serde_json::json!([
        {"id": 1, "name": "Spanish", "parent": null},
        {"id": 2, "name": "Spanish", "parent": null},
    ]);
    let archive = write_mochi(
        dir.path(),
        "duplicate.mochi",
        &decks,
        &serde_json::json!([]),
        serde_json::json!({"decks": 2, "cards": 0}),
    );

    let db_path = dir.path().join("typr.sqlite3");
    let db = open(&db_path);
    let error = db
        .apply(
            &Command::ImportArchive {
                archive,
                attachments: dir.path().join("media"),
            },
            NOW,
        )
        .expect_err("the refusal");
    assert!(
        matches!(&error, StoreError::DuplicateName { what, name } if *what == "deck under that parent" && name == "Spanish"),
        "the duplicate is named: {error}"
    );
    let snapshot = db.snapshot().expect("the library");
    assert!(
        snapshot.decks.is_empty(),
        "the first deck rolled back with the second"
    );
    assert!(snapshot.cards.is_empty());
}

const MINUTE: i64 = 60 * 1000;
const DAY: i64 = 24 * HOUR;

fn import_one_card(
    dir: &Path,
    card: serde_json::Value,
    reviews: usize,
) -> (Db, std::path::PathBuf, Id) {
    let decks = serde_json::json!([{"id": 1, "name": "Spanish", "parent": null}]);
    let archive = write_mochi(
        dir,
        "history.mochi",
        &decks,
        &serde_json::json!([card]),
        serde_json::json!({"decks": 1, "cards": 1, "reviews": reviews}),
    );
    let db = open(&dir.join("typr.sqlite3"));
    db.apply(
        &Command::ImportArchive {
            archive: archive.clone(),
            attachments: dir.join("media"),
        },
        NOW,
    )
    .expect("the import");
    let id = db.snapshot().expect("the library").cards[0].id;
    (db, archive, id)
}

#[test]
fn imported_history_keeps_its_grades_and_times() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let start = NOW - 30 * DAY;
    let (db, _archive, id) = import_one_card(
        dir.path(),
        serde_json::json!({
            "id": 1,
            "deck": 1,
            "text": "big\n---\nlarge",
            "reviews": [
                {"variant": "forward", "grade": "remembered", "at": start, "duration_ms": 900},
                {"variant": "forward", "grade": "remembered", "at": start + DAY, "duration_ms": 1100},
                {"variant": "forward", "grade": "forgot", "at": start + 2 * DAY},
            ],
        }),
        3,
    );

    let rows = db.history(id).expect("history");
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].key, "fwd");
    assert_eq!(rows[0].grade, Grade::Remembered);
    assert_eq!(rows[0].at, start);
    assert_eq!(rows[0].duration_ms, Some(900));
    assert_eq!(rows[1].at, start + DAY);
    assert_eq!(rows[2].grade, Grade::Forgot);
    assert_eq!(rows[2].at, start + 2 * DAY);
    assert_eq!(rows[2].duration_ms, None, "an absent duration stays absent");

    assert_eq!(
        db.prompt_keys(id).expect("prompts").len(),
        2,
        "two sides ask forward and reverse"
    );
    let card = &db.snapshot().expect("the library").cards[0];
    assert_eq!(card.reviews, 3);
    assert_eq!(
        card.due_at,
        Some(start + 2 * DAY + 10 * MINUTE),
        "the due date is the last record's, not the import's"
    );
}

#[test]
fn a_card_reviewed_both_ways_imports_both_prompts() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let start = NOW - 7 * DAY;
    let (db, _archive, id) = import_one_card(
        dir.path(),
        serde_json::json!({
            "id": 1,
            "deck": 1,
            "text": "big\n---\nlarge",
            "reviews": [
                {"variant": "forward", "grade": "remembered", "at": start},
                {"variant": "reverse", "grade": "forgot", "at": start + HOUR},
            ],
        }),
        2,
    );

    assert_eq!(db.prompt_keys(id).expect("prompts"), ["fwd", "rev"]);
    let rows = db.history(id).expect("history");
    assert_eq!(rows.len(), 2);
    assert!(
        rows.iter()
            .any(|row| row.key == "fwd" && row.grade == Grade::Remembered)
    );
    assert!(
        rows.iter()
            .any(|row| row.key == "rev" && row.grade == Grade::Forgot)
    );
}

#[test]
fn an_imported_due_date_matches_its_last_record() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let start = NOW - 3 * DAY;
    let (db, _archive, id) = import_one_card(
        dir.path(),
        serde_json::json!({
            "id": 1,
            "deck": 1,
            "text": "uno {{1::dos}} tres",
            "reviews": [
                {"variant": "cloze", "group": 1, "grade": "remembered", "at": start},
            ],
        }),
        1,
    );

    let rows = db.history(id).expect("history");
    let last = rows.last().expect("one record");
    assert_eq!(last.key, "cloze:1");
    let card = &db.snapshot().expect("the library").cards[0];
    assert_eq!(card.due_at, last.due_at);
    assert_eq!(card.due_at, Some(start + 10 * MINUTE));
}

#[test]
fn an_imported_image_is_present_after_its_source_is_deleted() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let picture: &[u8] = b"\x89PNG\r\n\x1a\na tiny picture";
    let decks = serde_json::json!([{"id": 1, "name": "Anatomy", "parent": null}]);
    let cards = serde_json::json!([{
        "id": 1,
        "deck": 1,
        "text": "",
        "image": "media/leak.png",
        "caption": "Where is the leak?",
    }]);
    let archive = write_mochi_media(
        dir.path(),
        "picture.mochi",
        &decks,
        &cards,
        serde_json::json!({"decks": 1, "cards": 1, "images": 1}),
        &[("media/leak.png", picture)],
    );
    let attachments = dir.path().join("attachments");

    let db = open(&dir.path().join("typr.sqlite3"));
    db.apply(
        &Command::ImportArchive {
            archive: archive.clone(),
            attachments: attachments.clone(),
        },
        NOW,
    )
    .expect("the import");
    std::fs::remove_file(&archive).expect("the archive is the only source");

    let card = &db.snapshot().expect("the library").cards[0];
    assert_eq!(card.kind, CardKind::Diagram);
    assert_eq!(card.image.as_deref(), Some("leak.png"));
    assert_eq!(card.caption.as_deref(), Some("Where is the leak?"));
    assert_eq!(
        std::fs::read(attachments.join("leak.png")).expect("the copied picture"),
        picture,
        "the picture outlives the archive it came from"
    );
}

#[test]
fn imported_masks_keep_their_group_and_position() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let decks = serde_json::json!([{"id": 1, "name": "Anatomy", "parent": null}]);
    let cards = serde_json::json!([{
        "id": 1,
        "deck": 1,
        "text": "",
        "image": "media/body.png",
        "caption": "",
        "masks": [
            {"group": 1, "x": 0.25, "y": 0.125, "w": 0.5, "h": 0.25},
            {"group": 0, "x": 0.0, "y": 0.0, "w": 0.125, "h": 0.125},
        ],
    }]);
    let archive = write_mochi_media(
        dir.path(),
        "masks.mochi",
        &decks,
        &cards,
        serde_json::json!({"decks": 1, "cards": 1, "images": 1, "masks": 2}),
        &[("media/body.png", b"png")],
    );

    let db = open(&dir.path().join("typr.sqlite3"));
    db.apply(
        &Command::ImportArchive {
            archive,
            attachments: dir.path().join("attachments"),
        },
        NOW,
    )
    .expect("the import");

    let card = &db.snapshot().expect("the library").cards[0];
    let expected = [
        typr_core::store::MaskRow {
            group: 0,
            rect: typr_core::store::MaskRect::new(0.0, 0.0, 0.125, 0.125),
        },
        typr_core::store::MaskRow {
            group: 1,
            rect: typr_core::store::MaskRect::new(0.25, 0.125, 0.5, 0.25),
        },
    ];
    assert_eq!(card.masks, expected, "fractions and groups survive exactly");
    assert_eq!(db.mask_groups(card.id).expect("groups"), [0, 1]);
    assert_eq!(
        db.prompt_keys(card.id).expect("prompts"),
        ["mask:0", "mask:1"]
    );
}

#[test]
fn a_copied_picture_never_overwrites_the_first_copy() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let source = dir.path().join("leak.png");
    let pictures = dir.path().join("attachments");
    std::fs::write(&source, b"first").expect("a source picture");

    let first = typr_core::transfer::attach(&pictures, &source).expect("the first copy");
    std::fs::write(&source, b"second").expect("the same name, new bytes");
    let second = typr_core::transfer::attach(&pictures, &source).expect("the second copy");

    assert_eq!(first, "leak.png");
    assert_eq!(second, "leak-2.png");
    assert_eq!(
        std::fs::read(pictures.join(&first)).expect("the first copy"),
        b"first"
    );
    assert_eq!(
        std::fs::read(pictures.join(&second)).expect("the second copy"),
        b"second"
    );
}

#[test]
fn a_manifest_count_mismatch_imports_nothing() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let decks = serde_json::json!([{"id": 1, "name": "Spanish"}]);
    let cards = serde_json::json!([{"id": 1, "deck": 1, "text": "hola"}]);
    let archive = write_mochi(
        dir.path(),
        "short.mochi",
        &decks,
        &cards,
        serde_json::json!({"decks": 1, "cards": 2}),
    );

    let db = open(&dir.path().join("typr.sqlite3"));
    let error = db
        .apply(
            &Command::ImportArchive {
                archive,
                attachments: dir.path().join("attachments"),
            },
            NOW,
        )
        .expect_err("the manifest promised a card the archive does not hold");

    let StoreError::Archive { reason, .. } = error else {
        panic!("expected the archive to be refused, got {error:?}");
    };
    assert!(
        reason.contains("declares") && reason.contains("cards: 2"),
        "the reason names the mismatch: {reason}"
    );
    let snapshot = db.snapshot().expect("the library");
    assert!(snapshot.cards.is_empty(), "no card survived the rollback");
    assert!(snapshot.decks.is_empty(), "no deck survived the rollback");
}

fn new_deck(db: &Db, name: &str) -> Id {
    let Outcome::Deck(id) = db
        .apply(
            &Command::CreateDeck {
                parent: None,
                name: name.to_string(),
            },
            NOW,
        )
        .expect("a deck")
    else {
        panic!("a deck command returned something else");
    };
    id
}

fn one_card(db: &Db, deck: Id, markdown: &str) -> Id {
    let Outcome::Card(id) = db
        .apply(
            &Command::CreateCard {
                deck,
                markdown: markdown.to_string(),
            },
            NOW,
        )
        .expect("a card")
    else {
        panic!("a card command returned something else");
    };
    id
}

#[test]
fn a_csv_round_trip_keeps_sides_tags_and_fields() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let db = open(&dir.path().join("typr.sqlite3"));
    let deck = new_deck(&db, "Biology");
    let original = one_card(&db, deck, "front\n---\nback");
    db.apply(
        &Command::SetTags {
            card: original,
            tags: vec!["verb".to_string(), "irregular".to_string()],
        },
        NOW,
    )
    .expect("tags");
    db.apply(
        &Command::SetFields {
            card: original,
            fields: vec![
                ("Word".to_string(), "front".to_string()),
                ("Meaning".to_string(), "back".to_string()),
            ],
        },
        NOW,
    )
    .expect("fields");

    let csv = dir.path().join("cards.csv");
    db.apply(
        &Command::ExportCsv {
            deck,
            target: csv.clone(),
        },
        NOW,
    )
    .expect("the export");
    let text = std::fs::read_to_string(&csv).expect("the exported file");
    assert_eq!(
        text.lines().next(),
        Some("deck, sides, tags, fields, markdown"),
        "the documented header leads the file"
    );
    assert!(
        text.contains("\"front\n---\nback\""),
        "the markdown cell keeps its newlines and quotes: {text}"
    );

    db.apply(&Command::ImportCsv { deck, source: csv }, NOW)
        .expect("the import");

    let snapshot = db.snapshot().expect("the library");
    let imported = snapshot
        .cards
        .iter()
        .find(|card| card.id != original)
        .expect("the imported card");
    assert_eq!(imported.markdown, "front\n---\nback");
    assert_eq!(imported.sides, 2);
    assert_eq!(imported.tags, ["irregular", "verb"], "tags come back whole");
    assert_eq!(
        imported.fields,
        [
            ("Meaning".to_string(), "back".to_string()),
            ("Word".to_string(), "front".to_string())
        ]
    );
}

#[test]
fn a_csv_without_the_header_is_refused() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let db = open(&dir.path().join("typr.sqlite3"));
    let deck = new_deck(&db, "Biology");
    let path = dir.path().join("not-cards.csv");
    std::fs::write(&path, "front,back\none,two\n").expect("a file without the header");

    let error = db
        .apply(&Command::ImportCsv { deck, source: path }, NOW)
        .expect_err("the header does not name the columns");
    let StoreError::Malformed { reason, .. } = error else {
        panic!("expected the file to be called malformed, got {error:?}");
    };
    assert!(
        reason.contains("missing") && reason.contains("deck") && reason.contains("markdown"),
        "the reason names the missing columns: {reason}"
    );
    assert!(
        db.snapshot().expect("the library").cards.is_empty(),
        "nothing was imported"
    );
}
