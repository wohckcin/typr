//! A library file written before the crate split must keep working after it.
//!
//! The fixture next to this file was written by the single-crate build at commit `0acaded`, before
//! any module moved: one deck, one two-sided card, and one grade on its prompt. To regenerate it,
//! check that commit out in a worktree with its own target directory, create a deck, a card and a
//! grade through `typr::store::Db`, and copy the resulting file here. Never regenerate it from this
//! crate: a file this crate wrote proves only that it can read its own output.

use std::path::{Path, PathBuf};

use typr_core::day::study_day;
use typr_core::store::{CURRENT_VERSION, Command, Db, Grade, Outcome, Phase};

const NOON: i64 = 1_768_089_600_000 + 13 * 60 * 60 * 1000;
const HOUR: i64 = 60 * 60 * 1000;

fn fixture_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/pre-split-library.sqlite3")
}

/// Copy the fixture into a scratch directory: opening it in place would write a WAL beside a file
/// that belongs to git.
fn open_copy() -> (tempfile::TempDir, PathBuf, Db) {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("library.sqlite3");
    std::fs::copy(fixture_path(), &path).expect("copy the fixture");
    let db = Db::open(&path).expect("open the fixture");
    (dir, path, db)
}

/// The schema version an SQLite file records, read without going through the store.
fn user_version(path: &Path) -> i64 {
    let conn = rusqlite::Connection::open(path).expect("raw open");
    conn.pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("user_version")
}

#[test]
fn a_library_written_before_the_split_still_opens_and_grades() {
    let (_dir, path, db) = open_copy();
    assert_eq!(
        user_version(&path),
        CURRENT_VERSION,
        "the fixture is already at the version this crate writes"
    );
    let snapshot = db.snapshot().expect("snapshot");

    assert_eq!(snapshot.decks.len(), 1, "the deck survived");
    assert_eq!(snapshot.cards.len(), 1, "the card survived");

    let card = snapshot.cards[0].id;
    let records = db.history(card).expect("history");
    let before = records.len();
    assert!(before >= 1, "the old grade survived");

    let graded = db
        .apply(
            &Command::Grade {
                card,
                key: "fwd".to_string(),
                grade: Grade::Remembered,
                duration_ms: Some(1_200),
            },
            NOON + HOUR,
        )
        .expect("a new grade is accepted");

    match graded {
        Outcome::Graded { phase, due_at } => {
            assert_eq!(phase, Phase::Review);
            // The old grade graduated the card, so this second success is the one that puts it a
            // day out: the interval comes from the history, which is what a split must not lose.
            assert_eq!(
                due_at.map(|at| study_day(at, 0)),
                Some(study_day(NOON + HOUR, 0) + 1),
                "a day after today's study day"
            );
        }
        other => panic!("unexpected outcome: {other:?}"),
    }

    assert_eq!(db.history(card).expect("history").len(), before + 1);
    assert_eq!(
        user_version(&path),
        CURRENT_VERSION,
        "grading does not migrate the file"
    );
    assert_eq!(
        db.prompt_keys(card).expect("prompts"),
        vec!["fwd".to_string(), "rev".to_string()],
        "no prompt was invented or lost"
    );
    assert!(
        db.snapshot().expect("snapshot").cards[0].markdown == "front\n---\nback",
        "the card still carries its text"
    );
}
