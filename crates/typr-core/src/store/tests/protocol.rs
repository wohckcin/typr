//! What the worker promises: one answer per command, in order, and a published
//! library that never lies about what was written.

use super::*;
use crate::store::task::UNPROMPTED;

#[test]
fn each_command_produces_exactly_one_event() {
    let dir = TempDir::new().expect("temp dir");
    let store = spawn_temp(&dir);

    let id = store.create_deck(None, "Japanese").expect("sent");
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

    store.create_deck(None, "Twice").expect("sent");
    let duplicate = store.create_deck(None, "Twice").expect("sent");
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

    let ids: Vec<u64> = ids.into_iter().map(|id| id.expect("sent")).collect();
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
        store.create_deck(None, "Biology").expect("sent");
        // Sent, not acknowledged: quitting must still write all of it.
        store
            .create_card(1, "mitochondria\n---\nthe powerhouse")
            .expect("sent");
        store
            .grade(1, "fwd", Grade::Remembered, Some(4_000))
            .expect("sent");
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
        store.create_deck(None, "Biology").expect("sent");
        store
            .create_card(1, "mitochondria\n---\nthe powerhouse")
            .expect("sent");
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
    store.create_deck(None, "Biology").expect("sent");
    wait_for(&store, 1);

    let before = store.snapshot().revision;
    store.stats(0, 1).expect("sent");
    wait_for(&store, 1);
    assert_eq!(
        before,
        store.snapshot().revision,
        "a query answers with rows of its own, so the library was not republished"
    );

    store.create_deck(None, "Chemistry").expect("sent");
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

    store.create_deck(None, "Chemistry").expect("sent");
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

#[test]
fn a_command_sent_after_quitting_is_refused() {
    let dir = TempDir::new().expect("temp dir");
    let store = spawn_temp(&dir);
    store.close();

    let error = store.create_deck(None, "Late").expect_err("closed");

    assert!(matches!(error, StoreError::Closed), "{error}");
    assert_eq!(store.snapshot().decks.len(), 0, "nothing was written");
}

#[test]
fn an_error_keeps_the_cause_that_produced_it() {
    let io = StoreError::from(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "denied",
    ));
    let cause = std::error::Error::source(&io).expect("a cause");

    assert_eq!(cause.to_string(), "denied");
    assert!(
        std::error::Error::source(&StoreError::from(rusqlite::Error::InvalidQuery)).is_some(),
        "the SQLite error is reachable through the chain"
    );
}

/// A clock that panics on the `trigger`-th reading, so a test can make the worker fail at a
/// chosen command without inventing a command that panics.
struct ExplodesOn {
    seen: std::sync::atomic::AtomicUsize,
    trigger: usize,
    payload: Option<u32>,
}

impl ExplodesOn {
    fn new(trigger: usize) -> Self {
        Self {
            seen: std::sync::atomic::AtomicUsize::new(0),
            trigger,
            payload: None,
        }
    }

    fn with_payload(trigger: usize, payload: u32) -> Self {
        Self {
            payload: Some(payload),
            ..Self::new(trigger)
        }
    }
}

impl crate::clock::Clock for ExplodesOn {
    fn now_ms(&self) -> i64 {
        let seen = self.seen.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        if seen >= self.trigger {
            match self.payload {
                Some(payload) => std::panic::panic_any(payload),
                None => panic!("clock exploded"),
            }
        }
        NOON
    }
}

/// A clock that holds the first reading until the test lets go, so three commands are provably
/// queued when the first one fails.
struct Held {
    released: std::sync::Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>,
}

impl crate::clock::Clock for Held {
    fn now_ms(&self) -> i64 {
        let (flag, cvar) = &*self.released;
        let mut released = flag.lock().expect("gate");
        while !*released {
            released = cvar.wait(released).expect("gate");
        }
        panic!("clock exploded after the queue filled");
    }
}

#[test]
fn a_stoppage_names_itself_and_has_no_cause() {
    let error = StoreError::WorkerPanicked {
        message: "clock exploded".to_string(),
    };
    assert_eq!(error.to_string(), "the library stopped: clock exploded");
    assert!(std::error::Error::source(&error).is_none());
}

#[test]
fn a_worker_panic_answers_the_request_it_was_applying() {
    let dir = TempDir::new().expect("temp dir");
    let store =
        Handle::spawn(open_temp(&dir), Box::new(ExplodesOn::new(1)), noop_waker()).expect("spawn");

    let id = store.create_deck(None, "Japanese").expect("sent");
    let events = wait_for(&store, 2);
    assert_eq!(events.len(), 2, "one answer and one report: {events:?}");
    assert_eq!(events[0].id, id);
    match &events[0].result {
        Err(StoreError::WorkerPanicked { message }) => assert!(message.contains("clock exploded")),
        other => panic!("expected a panic answer, got {other:?}"),
    }
    assert_eq!(events[1].id, UNPROMPTED, "the report answers no request");
    assert!(
        matches!(&events[1].result, Err(StoreError::WorkerPanicked { message }) if message.contains("clock exploded")),
        "{:?}",
        events[1].result
    );
}

#[test]
fn a_panic_answers_every_command_that_was_already_queued() {
    let dir = TempDir::new().expect("temp dir");
    let gate = std::sync::Arc::new((std::sync::Mutex::new(false), std::sync::Condvar::new()));
    let store = Handle::spawn(
        open_temp(&dir),
        Box::new(Held {
            released: std::sync::Arc::clone(&gate),
        }),
        noop_waker(),
    )
    .expect("spawn");

    let ids = [
        store.create_deck(None, "One").expect("sent"),
        store.create_deck(None, "Two").expect("sent"),
        store.create_deck(None, "Three").expect("sent"),
    ];
    {
        let (flag, cvar) = &*gate;
        *flag.lock().expect("gate") = true;
        cvar.notify_all();
    }

    let events = wait_for(&store, 4);
    assert_eq!(events.len(), 4, "three answers and one report: {events:?}");
    let mut answered: Vec<(u64, String)> = events
        .iter()
        .map(|event| {
            (
                event.id,
                match &event.result {
                    Ok(_) => "ok".to_string(),
                    Err(error) => error.to_string(),
                },
            )
        })
        .collect();
    answered.sort();
    assert_eq!(
        answered,
        vec![
            (
                UNPROMPTED,
                "the library stopped: clock exploded after the queue filled".to_string()
            ),
            (
                ids[0],
                "the library stopped: clock exploded after the queue filled".to_string()
            ),
            (ids[1], "the library is closed".to_string()),
            (ids[2], "the library is closed".to_string()),
        ]
    );
}

#[test]
fn a_panic_payload_without_text_is_still_reported() {
    let dir = TempDir::new().expect("temp dir");
    let store = Handle::spawn(
        open_temp(&dir),
        Box::new(ExplodesOn::with_payload(1, 7)),
        noop_waker(),
    )
    .expect("spawn");

    let _ = store.create_deck(None, "Japanese").expect("sent");
    let events = wait_for(&store, 2);
    let report = events
        .iter()
        .find(|event| event.id == UNPROMPTED)
        .expect("a report");
    match &report.result {
        Err(StoreError::WorkerPanicked { message }) => {
            assert!(message.contains("no message"), "{message}");
        }
        other => panic!("expected a panic answer, got {other:?}"),
    }
}

#[test]
fn a_send_after_a_panic_is_refused() {
    let dir = TempDir::new().expect("temp dir");
    let store =
        Handle::spawn(open_temp(&dir), Box::new(ExplodesOn::new(1)), noop_waker()).expect("spawn");

    let _ = store.create_deck(None, "Japanese").expect("sent");
    let _ = wait_for(&store, 2);
    let refused = store.create_deck(None, "Later");
    assert!(matches!(refused, Err(StoreError::Closed)), "{refused:?}");
}

#[test]
fn the_worker_status_outlives_the_handle() {
    let dir = TempDir::new().expect("temp dir");
    let store =
        Handle::spawn(open_temp(&dir), Box::new(ExplodesOn::new(1)), noop_waker()).expect("spawn");
    let status = store.status();

    let _ = store.create_deck(None, "Japanese").expect("sent");
    let _ = wait_for(&store, 2);
    drop(store);

    let ended = status.ended().expect("the worker ended abnormally");
    assert!(
        matches!(&*ended, StoreError::WorkerPanicked { message } if message.contains("clock exploded")),
        "{ended}"
    );
}

#[test]
fn a_worker_that_shut_down_cleanly_leaves_no_status() {
    let dir = TempDir::new().expect("temp dir");
    let store = spawn_temp(&dir);
    let status = store.status();

    let id = store.create_deck(None, "Japanese").expect("sent");
    let _ = wait_for(&store, 1);
    assert!(status.ended().is_none(), "still running after {id}");
    drop(store);
    assert!(status.ended().is_none(), "a clean quit records nothing");
}

/// A clock that holds the first reading until released, then panics on every later one.
struct HoldsThenDrops {
    readings: std::sync::atomic::AtomicUsize,
    released: std::sync::Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>,
}

impl crate::clock::Clock for HoldsThenDrops {
    fn now_ms(&self) -> i64 {
        let reading = self
            .readings
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if reading == 0 {
            let (flag, cvar) = &*self.released;
            let mut open = flag.lock().expect("gate");
            while !*open {
                open = cvar.wait(open).expect("gate");
            }
        }
        assert!(reading == 0, "the second reading fell over");
        0
    }
}

#[test]
fn a_panic_on_a_later_message_is_answered_against_that_message() {
    let dir = TempDir::new().expect("temp dir");
    let gate = std::sync::Arc::new((std::sync::Mutex::new(false), std::sync::Condvar::new()));
    let store = Handle::spawn(
        open_temp(&dir),
        Box::new(HoldsThenDrops {
            readings: std::sync::atomic::AtomicUsize::new(0),
            released: std::sync::Arc::clone(&gate),
        }),
        noop_waker(),
    )
    .expect("spawn");

    let ids = [
        store.create_deck(None, "One").expect("sent"),
        store.create_deck(None, "Two").expect("sent"),
        store.create_deck(None, "Three").expect("sent"),
    ];
    {
        let (flag, cvar) = &*gate;
        *flag.lock().expect("gate") = true;
        cvar.notify_all();
    }

    let events = wait_for(&store, 4);
    let mut answered: Vec<(u64, String)> = events
        .iter()
        .map(|event| {
            (
                event.id,
                match &event.result {
                    Ok(_) => "ok".to_string(),
                    Err(error) => error.to_string(),
                },
            )
        })
        .collect();
    answered.sort();
    assert_eq!(
        answered,
        vec![
            (
                UNPROMPTED,
                "the library stopped: the second reading fell over".to_string()
            ),
            (ids[0], "ok".to_string()),
            (
                ids[1],
                "the library stopped: the second reading fell over".to_string()
            ),
            (ids[2], "the library is closed".to_string()),
        ],
        "the message that panicked is the one answered as stopped, not the first of the batch"
    );
}
