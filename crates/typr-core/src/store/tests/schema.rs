//! Opening a file: migrations, refusals and drafts that outlive a restart.

use super::*;
use crate::store::SchemaFacts;

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
    let StoreError::Unreadable {
        path: named,
        source,
    } = &error
    else {
        panic!("a file that is not a library says which file: {error}");
    };
    assert_eq!(*named, path, "the error names the file it could not read");
    assert!(
        source.to_string().contains("not a database"),
        "the reason travels with it: {source}"
    );
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

#[test]
fn a_library_from_before_the_view_gains_it_on_open() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("typr.sqlite3");
    let db = Db::open(&path).expect("open");
    db.apply(
        &Command::CreateDeck {
            parent: None,
            name: "Keep".to_string(),
        },
        0,
    )
    .expect("deck");
    drop(db);

    // Put the file back to the state a v2 build wrote: no view, version two.
    let conn = Connection::open(&path).expect("open raw");
    conn.execute_batch("DROP VIEW studyable_prompt")
        .expect("drop the view");
    conn.pragma_update(None, "user_version", 2)
        .expect("rewind the version");
    drop(conn);

    let db = Db::open(&path).expect("reopen");
    assert_eq!(schema_version_at(&path), CURRENT_VERSION);
    assert_eq!(
        db.new_prompts(0, 0, 1, None)
            .expect("the view answers")
            .len(),
        0,
        "a deck with no cards has nothing studyable"
    );
}

#[test]
fn opening_reports_what_the_schema_did() {
    let dir = TempDir::new().expect("temp dir");
    let db = open_temp(&dir);
    assert_eq!(
        db.schema_facts(),
        SchemaFacts {
            found: 0,
            written: CURRENT_VERSION,
            applied: usize::try_from(CURRENT_VERSION).expect("a version that fits"),
        }
    );
    drop(db);

    let reopened = open_temp(&dir);
    assert_eq!(
        reopened.schema_facts(),
        SchemaFacts {
            found: CURRENT_VERSION,
            written: CURRENT_VERSION,
            applied: 0,
        },
        "a second open migrates nothing and says so"
    );
}

#[test]
fn a_library_from_one_version_back_reports_the_step_it_ran() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("typr.sqlite3");
    drop(Db::open(&path).expect("create"));
    let raw = Connection::open(&path).expect("raw");
    raw.execute_batch("DROP VIEW studyable_prompt;")
        .expect("rewind the view too");
    raw.pragma_update(None, "user_version", CURRENT_VERSION - 1)
        .expect("rewind");

    let db = Db::open(&path).expect("reopen");
    assert_eq!(
        db.schema_facts(),
        SchemaFacts {
            found: CURRENT_VERSION - 1,
            written: CURRENT_VERSION,
            applied: 1,
        }
    );
}
