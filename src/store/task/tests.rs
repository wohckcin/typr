//! The worker's own failure paths: a poisoned lock and a library that cannot be read back.

use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::clock::SystemClock;
use crate::store::Store;

use super::{Handle, UNPROMPTED, lock};

#[test]
fn a_poisoned_lock_is_still_usable() {
    let mutex = Arc::new(Mutex::new(7));
    let poison = Arc::clone(&mutex);
    let poisoned = std::thread::spawn(move || {
        let _guard = poison.lock().expect("first lock");
        panic!("leave the lock poisoned");
    })
    .join();

    assert!(poisoned.is_err(), "the thread really did panic");
    assert_eq!(
        *lock(&mutex),
        7,
        "the frame keeps reading what the worker left"
    );
}

#[test]
fn a_library_that_cannot_be_read_back_is_reported_not_swallowed() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let path = dir.path().join("typr.sqlite3");
    let db = crate::store::Db::open(&path).expect("open");
    let store = Handle::spawn(db, Box::new(SystemClock), Arc::new(|| {})).expect("spawn");

    // A second connection drops a table the snapshot reads, so the next publish fails.
    crate::store::Db::open(&path)
        .expect("reopen")
        .drop_card_table_for_test()
        .expect("break it");

    store.create_deck(None, "Chemistry").expect("sent");
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut events = Vec::new();
    while events.len() < 2 && Instant::now() < deadline {
        events.extend(store.poll());
        std::thread::sleep(Duration::from_millis(2));
    }

    let reported = events
        .iter()
        .find(|event| event.id == UNPROMPTED)
        .expect("the read-back failure came back as an event");
    assert!(
        reported.result.is_err(),
        "the failure is an error, not a silent success"
    );
    assert!(
        events
            .iter()
            .any(|event| event.id != UNPROMPTED && event.result.is_ok()),
        "the command itself still succeeded"
    );
}
