use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use rusqlite::Connection;
use tempfile::TempDir;

use crate::clock::TestClock;

use super::*;

fn noop_waker() -> rune::Waker {
    Arc::new(|| {})
}

fn open_temp(dir: &TempDir) -> Db {
    Db::open(&dir.path().join("typr.sqlite3")).expect("open")
}

fn spawn_temp(dir: &TempDir) -> Handle {
    Handle::spawn(open_temp(dir), Box::new(TestClock::new(1_700_000_000_000)), noop_waker()).expect("spawn")
}

fn schema_version_at(path: &Path) -> i64 {
    Connection::open(path)
        .expect("open for version")
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .expect("user_version")
}

/// Take events until at least `count` have arrived, so a test never sleeps a fixed guess.
fn wait_for(store: &Handle, count: usize) -> Vec<Event> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut events = Vec::new();
    while Instant::now() < deadline {
        events.extend(store.poll());
        if events.len() >= count {
            return events;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    panic!("only {} of {count} events arrived", events.len())
}

#[test]
fn a_fresh_database_applies_every_migration_exactly_once() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("typr.sqlite3");

    assert_eq!(schema_version_at_free(&path), 0, "a file that does not exist yet is version 0");
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
    if path.exists() { schema_version_at(path) } else { 0 }
}

#[test]
fn each_command_produces_exactly_one_event() {
    let dir = TempDir::new().expect("temp dir");
    let store = spawn_temp(&dir);

    let id = store.create_deck(None, "Japanese");
    let events = wait_for(&store, 1);
    assert_eq!(events.len(), 1, "one command, one event");
    assert_eq!(events[0].id, id);
    assert!(matches!(events[0].result, Ok(Outcome::Deck(1))), "{:?}", events[0].result);
    assert!(store.poll().is_empty(), "poll returns nothing with no commands in flight");
    assert_eq!(store.snapshot().decks.len(), 1);
    assert_eq!(store.snapshot().revision, 1);

    store.close();
}

#[test]
fn a_failing_command_answers_with_an_error_and_leaves_the_snapshot_alone() {
    let dir = TempDir::new().expect("temp dir");
    let store = spawn_temp(&dir);

    store.create_deck(None, "Twice");
    let duplicate = store.create_deck(None, "Twice");
    let events = wait_for(&store, 2);

    let failed = events.iter().find(|event| event.id == duplicate).expect("second event");
    assert!(failed.result.is_err(), "a duplicate deck name is refused");
    assert_eq!(store.snapshot().decks.len(), 1);
    assert_eq!(store.snapshot().revision, 1, "a failed command publishes nothing");

    store.close();
}

#[test]
fn every_store_method_is_reachable_through_the_trait() {
    let dir = TempDir::new().expect("temp dir");
    let store = spawn_temp(&dir);
    let door: &dyn Store = &store;

    let ids = vec![
        door.create_deck(None, "Decks"),
        door.rename_deck(1, "Decks renamed"),
        door.archive_deck(1, false),
        door.create_card(1, "front\n---\nback"),
        door.save_card(1, "front\n---\nback, edited"),
        door.set_draft(1, Some("unsaved")),
        door.trash_card(1),
        door.restore_card(1),
        door.set_setting("new_per_day", "10"),
        door.history(1),
        door.stats(0, i64::MAX),
    ];

    let mut sorted = ids.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), ids.len(), "every command gets its own id");

    let events = wait_for(&store, ids.len());
    assert_eq!(events.len(), ids.len());
    assert!(events.iter().all(|event| event.result.is_ok()), "{events:?}");
    assert!(matches!(events.last().map(|e| &e.result), Some(Ok(Outcome::Stats(_)))));
    assert_eq!(store.snapshot().settings.new_per_day, 10);

    store.close();
}

#[test]
fn a_command_sent_and_then_quit_is_on_disk() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("typr.sqlite3");

    {
        let store = Handle::spawn(Db::open(&path).expect("open"), Box::new(TestClock::new(0)), noop_waker())
            .expect("spawn");
        store.create_deck(None, "Biology");
        // Sent, not acknowledged: quitting must still write it.
        store.create_card(1, "mitochondria\n---\nthe powerhouse");
        store.close();
    }

    let db = Db::open(&path).expect("reopen");
    let snapshot = db.snapshot().expect("snapshot");
    assert_eq!(snapshot.decks.len(), 1);
    assert_eq!(snapshot.cards.len(), 1);
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
        conn.pragma_update(None, "user_version", CURRENT_VERSION + 1).expect("bump");
    }
    let before = std::fs::read(&path).expect("read before");

    let error = Db::open(&path).expect_err("too new");
    assert!(
        matches!(error, StoreError::TooNew { found, supported } if found == CURRENT_VERSION + 1 && supported == CURRENT_VERSION),
        "{error}"
    );
    assert_eq!(std::fs::read(&path).expect("read after"), before);
}

fn deck(db: &mut Db, name: &str) -> Id {
    match db.apply(&Command::CreateDeck { parent: None, name: name.to_string() }, 0) {
        Ok(Outcome::Deck(id)) => id,
        other => panic!("creating a deck: {other:?}"),
    }
}

fn card(db: &mut Db, deck: Id, markdown: &str) -> Id {
    match db.apply(&Command::CreateCard { deck, markdown: markdown.to_string() }, 0) {
        Ok(Outcome::Card(id)) => id,
        other => panic!("creating a card: {other:?}"),
    }
}

#[test]
fn a_prompt_appears_with_the_second_side_and_survives_edits() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Mixed");

    let note = card(&mut db, deck, "just a note");
    assert!(db.prompt_keys(note).expect("keys").is_empty(), "a note has no prompt");

    let two_sided = card(&mut db, deck, "front\n---\nback");
    assert_eq!(db.prompt_keys(two_sided).expect("keys"), vec!["fwd".to_string()]);

    // Turned into a note: the prompt and its schedule are kept, the card just stops queueing.
    db.apply(&Command::SaveCard { id: two_sided, markdown: "front".to_string() }, 0).expect("save");
    let sides = db.snapshot().expect("snapshot").cards.iter().find(|c| c.id == two_sided).expect("card").sides;
    assert_eq!(sides, 1);
    assert_eq!(db.prompt_keys(two_sided).expect("keys"), vec!["fwd".to_string()]);

    // And back again: still exactly one prompt, not two.
    db.apply(&Command::SaveCard { id: two_sided, markdown: "front\n---\nback".to_string() }, 0).expect("save");
    assert_eq!(db.prompt_keys(two_sided).expect("keys").len(), 1);
}

#[test]
fn three_learning_cycles_then_two_reviews_report_a_count_of_two() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Counting");
    let card = card(&mut db, deck, "front\n---\nback");

    let mut at = 1_700_000_000_000;
    for _ in 0..3 {
        db.apply(
            &Command::Grade { card, key: "fwd".to_string(), grade: Grade::Forgot, duration_ms: None },
            at,
        )
        .expect("grade");
        at += 10 * 60_000;
    }
    for _ in 0..2 {
        db.apply(
            &Command::Grade { card, key: "fwd".to_string(), grade: Grade::Remembered, duration_ms: None },
            at,
        )
        .expect("grade");
        at += 24 * 60 * 60_000;
    }

    let history = db.history(card).expect("history");
    assert_eq!(history.len(), 5);
    assert_eq!(
        history.iter().map(|row| row.phase).collect::<Vec<_>>(),
        vec![Phase::Learn, Phase::Learn, Phase::Learn, Phase::Review, Phase::Review]
    );
    assert_eq!(db.review_count(card).expect("count"), 2);
}

#[test]
fn grading_a_note_is_refused() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Notes");
    let note = card(&mut db, deck, "nothing to recall");

    let error = db
        .apply(&Command::Grade { card: note, key: "fwd".to_string(), grade: Grade::Remembered, duration_ms: None }, 0)
        .expect_err("no prompt");
    assert!(matches!(error, StoreError::NoPrompt { .. }), "{error}");
}
