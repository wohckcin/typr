//! Opening a file: migrations, refusals and drafts that outlive a restart.

use super::*;

#[test]
fn a_fresh_database_applies_every_migration_exactly_once() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("typr.sqlite3");

    assert_eq!(
        schema_version_at_free(&path),
        0,
        "a file that does not exist yet is version 0"
    );
    {
        Db::open(&path).expect("first open");
    }
    assert_eq!(schema_version_at(&path), CURRENT_VERSION);

    // A second open must not re-apply anything: re-running `CREATE TABLE` would fail.
    {
        Db::open(&path).expect("second open");
    }
    assert_eq!(schema_version_at(&path), CURRENT_VERSION);
}

/// A path with no file behind it reads as schema version 0.
fn schema_version_at_free(path: &Path) -> i64 {
    if path.exists() {
        schema_version_at(path)
    } else {
        0
    }
}

#[test]
fn a_file_that_is_not_a_database_is_reported_and_left_alone() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("typr.sqlite3");
    let contents = b"this file belongs to something else";
    std::fs::write(&path, contents).expect("write");

    let error = Db::open(&path).expect_err("not a database");
    assert!(matches!(error, StoreError::Sqlite(_)), "{error}");
    assert_eq!(std::fs::read(&path).expect("read back"), contents);
}

#[test]
fn a_database_from_a_newer_build_is_refused_byte_for_byte() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("typr.sqlite3");
    Db::open(&path).expect("create");
    {
        let conn = Connection::open(&path).expect("open for bump");
        conn.pragma_update(None, "user_version", CURRENT_VERSION + 1)
            .expect("bump");
    }
    let before = std::fs::read(&path).expect("read before");

    let error = Db::open(&path).expect_err("too new");
    assert!(
        matches!(error, StoreError::TooNew { found, supported } if found == CURRENT_VERSION + 1 && supported == CURRENT_VERSION),
        "{error}"
    );
    assert_eq!(std::fs::read(&path).expect("read after"), before);
}

#[test]
fn a_negative_schema_version_is_refused() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("library.sqlite3");
    {
        let conn = rusqlite::Connection::open(&path).expect("a raw connection");
        conn.pragma_update(None, "user_version", -1i64)
            .expect("a nonsense version");
    }

    match Db::open(&path) {
        Err(StoreError::Corrupt { version }) => assert_eq!(version, -1),
        other => panic!("{other:?}"),
    }
}

#[test]
fn an_unsaved_draft_survives_reopening_the_database() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("library.sqlite3");
    {
        let db = Db::open(&path).unwrap();
        db.apply(
            &Command::SetDraft {
                card: 0,
                text: Some("half a card".to_string()),
            },
            NOON,
        )
        .unwrap();
    }

    let db = Db::open(&path).unwrap();
    assert_eq!(
        db.draft(0).unwrap().as_deref(),
        Some("half a card"),
        "the draft is still there"
    );

    db.apply(
        &Command::SetDraft {
            card: 0,
            text: None,
        },
        NOON,
    )
    .unwrap();
    assert_eq!(
        db.draft(0).unwrap(),
        None,
        "and it is cleared once the card exists"
    );

    let deck = match db
        .apply(
            &Command::CreateDeck {
                parent: None,
                name: "Deck".into(),
            },
            NOON,
        )
        .unwrap()
    {
        Outcome::Deck(id) => id,
        other => panic!("expected a deck, got {other:?}"),
    };
    db.apply(
        &Command::SetDraft {
            card: 0,
            text: Some("draft of a new card".into()),
        },
        NOON,
    )
    .unwrap();
    let card = match db
        .apply(
            &Command::CreateCard {
                deck,
                markdown: "draft of a new card".into(),
            },
            NOON,
        )
        .unwrap()
    {
        Outcome::Card(id) => id,
        other => panic!("expected a card, got {other:?}"),
    };
    assert_eq!(
        db.draft(0).unwrap(),
        None,
        "creating the card consumes the draft"
    );
    assert_eq!(db.draft(card).unwrap(), None);
}
