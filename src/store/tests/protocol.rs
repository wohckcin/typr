//! What the worker promises: one answer per command, in order, and a published
//! library that never lies about what was written.

use super::*;

#[test]
fn each_command_produces_exactly_one_event() {
    let dir = TempDir::new().expect("temp dir");
    let store = spawn_temp(&dir);

    let id = store.create_deck(None, "Japanese");
    let events = wait_for(&store, 1);
    assert_eq!(events.len(), 1, "one command, one event");
    assert_eq!(events[0].id, id);
    assert!(
        matches!(events[0].result, Ok(Outcome::Deck(1))),
        "{:?}",
        events[0].result
    );
    assert!(
        store.poll().is_empty(),
        "poll returns nothing with no commands in flight"
    );
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

    let failed = events
        .iter()
        .find(|event| event.id == duplicate)
        .expect("second event");
    assert!(failed.result.is_err(), "a duplicate deck name is refused");
    assert_eq!(store.snapshot().decks.len(), 1);
    assert_eq!(
        store.snapshot().revision,
        1,
        "a failed command publishes nothing"
    );

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
    assert!(
        events.iter().all(|event| event.result.is_ok()),
        "{events:?}"
    );
    assert!(matches!(
        events.last().map(|e| &e.result),
        Some(Ok(Outcome::Stats(_)))
    ));
    assert_eq!(store.snapshot().settings.new_per_day, 10);

    store.close();
}

#[test]
fn a_command_sent_and_then_quit_is_on_disk() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("typr.sqlite3");

    {
        let store = Handle::spawn(
            Db::open(&path).expect("open"),
            Box::new(TestClock::new(0)),
            noop_waker(),
        )
        .expect("spawn");
        store.create_deck(None, "Biology");
        // Sent, not acknowledged: quitting must still write all of it.
        store.create_card(1, "mitochondria\n---\nthe powerhouse");
        store.grade(1, "fwd", Grade::Remembered, Some(4_000));
        store.close();
    }

    let db = Db::open(&path).expect("reopen");
    let snapshot = db.snapshot().expect("snapshot");
    assert_eq!(snapshot.decks.len(), 1);
    assert_eq!(snapshot.cards.len(), 1);
    let history = db.history(1).expect("history");
    assert_eq!(
        history.len(),
        1,
        "the grade sent just before the quit is on disk"
    );
    assert_eq!(history[0].grade, Grade::Remembered);
    assert_eq!(history[0].duration_ms, Some(4_000));
}

#[test]
fn dropping_the_handle_writes_what_was_sent() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("typr.sqlite3");

    {
        let store = Handle::spawn(
            Db::open(&path).expect("open"),
            Box::new(TestClock::new(0)),
            noop_waker(),
        )
        .expect("spawn");
        store.create_deck(None, "Biology");
        store.create_card(1, "mitochondria\n---\nthe powerhouse");
        // No close(): the drop is the quit, and Rune's AppSpec has no quit hook to call one.
    }

    let db = Db::open(&path).expect("reopen");
    assert_eq!(
        db.snapshot().expect("snapshot").cards.len(),
        1,
        "the card sent before the drop is on disk"
    );
}

#[test]
fn a_read_only_command_leaves_the_revision_alone() {
    let dir = TempDir::new().expect("temp dir");
    let store = spawn_temp(&dir);
    store.create_deck(None, "Biology");
    wait_for(&store, 1);

    let before = store.snapshot().revision;
    store.stats(0, 1);
    wait_for(&store, 1);
    assert_eq!(
        before,
        store.snapshot().revision,
        "a query answers with rows of its own, so the library was not republished"
    );

    store.create_deck(None, "Chemistry");
    wait_for(&store, 1);
    assert!(
        store.snapshot().revision > before,
        "a write does republish it"
    );
}

#[test]
fn reading_the_snapshot_twice_shares_one_allocation() {
    let dir = TempDir::new().expect("temp dir");
    let store = spawn_temp(&dir);
    let first = store.snapshot();
    let second = store.snapshot();
    assert!(
        std::sync::Arc::ptr_eq(&first, &second),
        "a read hands out the published library instead of copying it"
    );

    store.create_deck(None, "Chemistry");
    wait_for(&store, 1);
    assert!(
        !std::sync::Arc::ptr_eq(&first, &store.snapshot()),
        "a write publishes a new one, so a held copy stays the library it was"
    );
}

#[test]
fn a_failing_command_leaves_no_partial_write() {
    let dir = TempDir::new().expect("temp dir");
    let db = open_temp(&dir);

    let error = db
        .apply(
            &Command::SetTags {
                card: 404,
                tags: vec!["orphan".to_string()],
            },
            0,
        )
        .expect_err("no card 404");

    assert!(matches!(error, StoreError::Sqlite(_)), "{error:?}");
    assert!(
        db.tag_names().expect("the tag table").is_empty(),
        "the tag written before the foreign key refused the card is gone"
    );
}
