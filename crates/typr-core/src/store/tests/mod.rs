use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use rusqlite::Connection;
use tempfile::TempDir;

use crate::clock::TestClock;

use super::db::Record;
use super::*;

mod cards;
mod protocol;
mod review;
mod schema;

/// Noon on a fixed day, so a study day and its bounds are unambiguous in tests.
const NOON: i64 = 1_768_089_600_000 + 13 * 60 * 60 * 1000;
const HOUR: i64 = 60 * 60 * 1000;

fn noop_waker() -> super::Waker {
    Arc::new(|| {})
}

fn open_temp(dir: &TempDir) -> Db {
    Db::open(&dir.path().join("typr.sqlite3")).expect("open")
}

fn spawn_temp(dir: &TempDir) -> Handle {
    Handle::spawn(
        open_temp(dir),
        Box::new(TestClock::new(1_700_000_000_000)),
        noop_waker(),
    )
    .expect("spawn")
}

/// Ids the snapshot's own filter keeps, which is what the library screen shows.
fn matching(db: &Db, query: &str, deck: Option<Id>, mode: CardMode) -> Vec<Id> {
    let snapshot = db.snapshot().expect("snapshot");
    snapshot
        .cards_matching(query, None, deck, mode)
        .into_iter()
        .map(|card| card.id)
        .collect()
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

fn deck(db: &mut Db, name: &str) -> Id {
    match db.apply(
        &Command::CreateDeck {
            parent: None,
            name: name.to_string(),
        },
        0,
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
        0,
    ) {
        Ok(Outcome::Card(id)) => id,
        other => panic!("creating a card: {other:?}"),
    }
}

fn schedule(db: &mut Db, card: Id, due_at: i64) {
    db.append_record_for_test(Record {
        card,
        key: "fwd",
        phase: Phase::Review,
        grade: Grade::Remembered,
        at: due_at - 24 * HOUR,
        interval_ms: 24 * HOUR,
        due_at: Some(due_at),
    })
    .expect("record");
    db.rebuild_prompts().expect("rebuild");
}

/// Put a file back to an older schema: the version number alone is not a simulation, because a build
/// of that age had none of the objects later migrations added.
pub(crate) fn rewind_for_test(conn: &Connection, version: i64) {
    if version < 4 {
        // The later view names the later column, so it has to go first, and a version-3 file then
        // carries version 3's view again.
        conn.execute_batch("DROP VIEW IF EXISTS studyable_prompt")
            .expect("drop the view");
        conn.execute_batch("ALTER TABLE card DROP COLUMN has_cloze")
            .expect("drop the hidden-text column");
        if version == 3 {
            conn.execute_batch(crate::store::schema::V3_SQL)
                .expect("put the version-3 view back");
        }
    }
    conn.pragma_update(None, "user_version", version)
        .expect("rewind the version");
}
