use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use rusqlite::Connection;
use tempfile::TempDir;

use crate::clock::TestClock;

use super::db::Record;
use super::*;

fn noop_waker() -> rune::Waker {
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

#[test]
fn a_prompt_appears_with_the_second_side_and_survives_edits() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Mixed");

    let note = card(&mut db, deck, "just a note");
    assert!(
        db.prompt_keys(note).expect("keys").is_empty(),
        "a note has no prompt"
    );

    let two_sided = card(&mut db, deck, "front\n---\nback");
    assert_eq!(
        db.prompt_keys(two_sided).expect("keys"),
        vec!["fwd".to_string()]
    );

    // Turned into a note: the prompt and its schedule are kept, the card just stops queueing.
    db.apply(
        &Command::SaveCard {
            id: two_sided,
            markdown: "front".to_string(),
        },
        0,
    )
    .expect("save");
    let sides = db
        .snapshot()
        .expect("snapshot")
        .cards
        .iter()
        .find(|c| c.id == two_sided)
        .expect("card")
        .sides;
    assert_eq!(sides, 1);
    assert_eq!(
        db.prompt_keys(two_sided).expect("keys"),
        vec!["fwd".to_string()]
    );

    // And back again: still exactly one prompt, not two.
    db.apply(
        &Command::SaveCard {
            id: two_sided,
            markdown: "front\n---\nback".to_string(),
        },
        0,
    )
    .expect("save");
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
            &Command::Grade {
                card,
                key: "fwd".to_string(),
                grade: Grade::Forgot,
                duration_ms: None,
            },
            at,
        )
        .expect("grade");
        at += 10 * 60_000;
    }
    for _ in 0..2 {
        db.apply(
            &Command::Grade {
                card,
                key: "fwd".to_string(),
                grade: Grade::Remembered,
                duration_ms: None,
            },
            at,
        )
        .expect("grade");
        at += 24 * 60 * 60_000;
    }

    let history = db.history(card).expect("history");
    assert_eq!(history.len(), 5);
    assert_eq!(
        history.iter().map(|row| row.phase).collect::<Vec<_>>(),
        vec![
            Phase::Learn,
            Phase::Learn,
            Phase::Learn,
            Phase::Review,
            Phase::Review
        ]
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
        .apply(
            &Command::Grade {
                card: note,
                key: "fwd".to_string(),
                grade: Grade::Remembered,
                duration_ms: None,
            },
            0,
        )
        .expect_err("no prompt");
    assert!(matches!(error, StoreError::NoPrompt { .. }), "{error}");
}

#[test]
fn a_prompt_always_equals_the_fold_of_its_records() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Fold");

    // Five prompts, each pushed into a different phase by a different grade sequence.
    let sequences: [&[Grade]; 5] = [
        &[Grade::Remembered],
        &[Grade::Forgot],
        &[Grade::Remembered, Grade::Remembered],
        &[Grade::Remembered, Grade::Remembered, Grade::Remembered],
        &[Grade::Remembered, Grade::Remembered, Grade::Forgot],
    ];
    let mut cards = Vec::new();
    for (index, sequence) in sequences.iter().enumerate() {
        let card = card(&mut db, deck, &format!("front {index}\n---\nback {index}"));
        let mut at = 1_700_000_000_000;
        for grade in sequence.iter() {
            db.apply(
                &Command::Grade {
                    card,
                    key: "fwd".to_string(),
                    grade: *grade,
                    duration_ms: None,
                },
                at,
            )
            .expect("grade");
            at += 11 * 60_000;
        }
        cards.push(card);
    }

    let phases: Vec<Phase> = cards
        .iter()
        .map(|card| {
            db.prompt(*card, "fwd")
                .expect("prompt")
                .expect("exists")
                .phase
        })
        .collect();
    assert!(
        phases.contains(&Phase::Learn)
            && phases.contains(&Phase::Relearn)
            && phases.contains(&Phase::Review)
    );

    for card in &cards {
        let stored = db.prompt(*card, "fwd").expect("prompt");
        let folded = db.fold_records(*card, "fwd").expect("fold");
        assert_eq!(stored, folded, "card {card} disagrees with its history");
    }

    // Rebuilding from history changes nothing, which is the invariant in action.
    assert_eq!(db.rebuild_prompts().expect("rebuild"), cards.len());
    for card in &cards {
        assert_eq!(
            db.prompt(*card, "fwd").expect("prompt"),
            db.fold_records(*card, "fwd").expect("fold")
        );
    }
}

#[test]
fn prompt_keys_beyond_fwd_fold_into_their_own_prompts() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Keys");
    let card = card(&mut db, deck, "front\n---\nback");

    // Records a later version will write: a reverse direction, a cloze group and a mask group.
    let mut at = 1_700_000_000_000;
    for key in ["clair", "cloze:1", "mask:a1"] {
        db.append_record_for_test(Record {
            card,
            key,
            phase: Phase::Review,
            grade: Grade::Remembered,
            at,
            interval_ms: 86_400_000,
            due_at: Some(at + 86_400_000),
        })
        .expect("record");
        at += 1_000;
    }

    // Only the three keys that have records are rewritten; `fwd` has none and keeps its row.
    assert_eq!(db.rebuild_prompts().expect("rebuild"), 3);
    assert_eq!(
        db.prompt_keys(card).expect("keys"),
        vec![
            "clair".to_string(),
            "cloze:1".to_string(),
            "fwd".to_string(),
            "mask:a1".to_string()
        ]
    );
    for key in ["clair", "cloze:1", "mask:a1"] {
        assert_eq!(
            db.prompt(card, key).expect("prompt"),
            db.fold_records(card, key).expect("fold")
        );
    }
}

/// Noon-ish of a fixed day, so a test never straddles a rollover.
const NOON: i64 = 1_768_089_600_000 + 13 * 60 * 60 * 1000;
const HOUR: i64 = 60 * 60 * 1000;
const WEEK: i64 = 7 * 24 * HOUR;

/// Put a prompt's schedule where a record would have put it.
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

#[test]
fn the_most_overdue_study_day_comes_first() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Urgency");

    let week_ago = card(&mut db, deck, "old\n---\nancient");
    let an_hour_ago = card(&mut db, deck, "recent\n---\nnow");
    let two_hours_ago = card(&mut db, deck, "also recent\n---\nnow");
    schedule(&mut db, week_ago, NOON - WEEK);
    schedule(&mut db, an_hour_ago, NOON - HOUR);
    schedule(&mut db, two_hours_ago, NOON - 2 * HOUR);

    let due = db.due_prompts(NOON, 0, 11).expect("due");
    assert_eq!(due.len(), 3);
    assert_eq!(due[0].card, week_ago);
}

#[test]
fn prompts_due_on_the_same_study_day_are_ordered_by_the_shuffle() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Ties");
    let first = card(&mut db, deck, "a\n---\nb");
    let second = card(&mut db, deck, "c\n---\nd");
    schedule(&mut db, first, NOON - 2 * HOUR);
    schedule(&mut db, second, NOON - HOUR);

    let mut orders = std::collections::BTreeSet::new();
    for seed in 1..40 {
        let due = db.due_prompts(NOON, 0, seed).expect("due");
        orders.insert(due.iter().map(|prompt| prompt.card).collect::<Vec<_>>());
    }
    assert_eq!(
        orders.len(),
        2,
        "the timestamp must not decide the order: {orders:?}"
    );
}

#[test]
fn trashed_archived_and_note_prompts_are_never_due() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let archived_root = deck(&mut db, "Archived root");
    let child = match db.apply(
        &Command::CreateDeck {
            parent: Some(archived_root),
            name: "Child".to_string(),
        },
        0,
    ) {
        Ok(Outcome::Deck(id)) => id,
        other => panic!("{other:?}"),
    };
    let open = deck(&mut db, "Open");

    let kept = card(&mut db, open, "kept\n---\nkept");
    let trashed = card(&mut db, open, "trashed\n---\ntrashed");
    let under_archived = card(&mut db, child, "archived\n---\narchived");
    let note = card(&mut db, open, "just a note");
    for card in [kept, trashed, under_archived, note] {
        schedule(&mut db, card, NOON - HOUR);
    }
    db.apply(&Command::TrashCard { id: trashed }, NOON)
        .expect("trash");
    db.apply(
        &Command::ArchiveDeck {
            id: archived_root,
            archived: true,
        },
        NOON,
    )
    .expect("archive");

    let due = db.due_prompts(NOON, 0, 5).expect("due");
    assert_eq!(
        due.iter().map(|prompt| prompt.card).collect::<Vec<_>>(),
        vec![kept]
    );
}

#[test]
fn a_prompt_due_tomorrow_is_not_in_todays_queue() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Tomorrow");
    let card = card(&mut db, deck, "a\n---\nb");
    schedule(&mut db, card, NOON + 24 * HOUR);

    assert!(db.due_prompts(NOON, 0, 1).expect("due").is_empty());
    assert_eq!(
        db.due_prompts(NOON + 25 * HOUR, 0, 1).expect("due").len(),
        1
    );
}

#[test]
fn resetting_history_makes_a_card_new_again() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Reset");
    let card = card(&mut db, deck, "front\n---\nback");

    let mut at = NOON - WEEK;
    for _ in 0..3 {
        db.apply(
            &Command::Grade {
                card,
                key: "fwd".to_string(),
                grade: Grade::Remembered,
                duration_ms: None,
            },
            at,
        )
        .expect("grade");
        at += 24 * HOUR;
    }
    assert_eq!(db.review_count(card).expect("count"), 3);

    db.apply(
        &Command::ResetHistory {
            card,
            key: "fwd".to_string(),
        },
        NOON,
    )
    .expect("reset");

    assert!(
        db.history(card).expect("history").is_empty(),
        "learn records go too"
    );
    assert_eq!(db.review_count(card).expect("count"), 0);
    assert_eq!(db.fold_records(card, "fwd").expect("fold"), None);
    let prompt = db
        .prompt(card, "fwd")
        .expect("prompt")
        .expect("still exists");
    assert_eq!(
        (prompt.phase, prompt.interval_ms, prompt.due_at, prompt.reps),
        (Phase::Learn, 0, None, 0)
    );
    assert!(
        db.due_prompts(NOON, 0, 1).expect("due").is_empty(),
        "the card stops being due"
    );
}

#[test]
fn the_cap_counts_graduated_cards_across_decks_and_ignores_learning_cycles() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let first_deck = deck(&mut db, "First");
    let second_deck = deck(&mut db, "Second");
    let stubborn = card(&mut db, first_deck, "a\n---\nb");
    let other = card(&mut db, second_deck, "c\n---\nd");

    let mut at = NOON;
    for _ in 0..3 {
        db.apply(
            &Command::Grade {
                card: stubborn,
                key: "fwd".to_string(),
                grade: Grade::Forgot,
                duration_ms: None,
            },
            at,
        )
        .expect("grade");
        at += 60_000;
    }
    assert_eq!(
        db.learned_on(crate::day::study_day(NOON, 0), 0)
            .expect("learned"),
        0
    );

    db.apply(
        &Command::Grade {
            card: stubborn,
            key: "fwd".to_string(),
            grade: Grade::Remembered,
            duration_ms: None,
        },
        at,
    )
    .expect("grade");
    db.apply(
        &Command::Grade {
            card: other,
            key: "fwd".to_string(),
            grade: Grade::Remembered,
            duration_ms: None,
        },
        at + 60_000,
    )
    .expect("grade");

    assert_eq!(
        db.learned_on(crate::day::study_day(NOON, 0), 0)
            .expect("learned"),
        2
    );
}

#[test]
fn the_new_queue_stops_at_the_cap_and_resumes_the_next_day() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Backlog");
    for index in 0..3 {
        card(&mut db, deck, &format!("front {index}\n---\nback {index}"));
    }
    db.apply(
        &Command::SetSetting {
            key: "new_per_day".to_string(),
            value: "2".to_string(),
        },
        NOON,
    )
    .expect("setting");
    assert_eq!(db.settings().expect("settings").new_per_day, 2);

    assert_eq!(
        db.new_prompts(NOON, 0, 3).expect("new").len(),
        2,
        "the queue stops at the cap"
    );

    // Learning two of them spends today's allowance.
    let waiting = db.new_prompts(NOON, 0, 3).expect("new");
    for prompt in waiting {
        db.apply(
            &Command::Grade {
                card: prompt.card,
                key: prompt.key,
                grade: Grade::Remembered,
                duration_ms: None,
            },
            NOON + HOUR,
        )
        .expect("grade");
    }
    assert!(
        db.new_prompts(NOON + HOUR, 0, 3).expect("new").is_empty(),
        "the day is spent"
    );

    let tomorrow = NOON + 24 * HOUR;
    assert_eq!(
        db.new_prompts(tomorrow, 0, 3).expect("new").len(),
        1,
        "the last card waits for tomorrow"
    );
}

#[test]
fn statistics_are_counted_per_study_day() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Stats");
    let card = card(&mut db, deck, "a\n---\nb");

    let today = crate::day::study_day(NOON, 0);
    let tomorrow_at = NOON + 24 * HOUR;
    db.apply(
        &Command::Grade {
            card,
            key: "fwd".to_string(),
            grade: Grade::Remembered,
            duration_ms: None,
        },
        NOON,
    )
    .expect("grade");
    db.apply(
        &Command::Grade {
            card,
            key: "fwd".to_string(),
            grade: Grade::Remembered,
            duration_ms: None,
        },
        NOON + 600_000,
    )
    .expect("grade");
    db.apply(
        &Command::Grade {
            card,
            key: "fwd".to_string(),
            grade: Grade::Forgot,
            duration_ms: None,
        },
        tomorrow_at,
    )
    .expect("grade");

    let today_stats = db.stats_for_day(today, 0).expect("stats");
    assert_eq!(today_stats.reviews, 2);
    assert_eq!(today_stats.lapses, 0);
    let tomorrow_stats = db.stats_for_day(today + 1, 0).expect("stats");
    assert_eq!(tomorrow_stats.reviews, 1);
    assert_eq!(tomorrow_stats.lapses, 1);
}

#[test]
fn a_deck_cannot_be_moved_inside_itself() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let parent = deck(&mut db, "Parent");
    let child = match db.apply(
        &Command::CreateDeck {
            parent: Some(parent),
            name: "Child".to_string(),
        },
        0,
    ) {
        Ok(Outcome::Deck(id)) => id,
        other => panic!("{other:?}"),
    };
    let grandchild = match db.apply(
        &Command::CreateDeck {
            parent: Some(child),
            name: "Grandchild".to_string(),
        },
        0,
    ) {
        Ok(Outcome::Deck(id)) => id,
        other => panic!("{other:?}"),
    };

    let error = db
        .apply(
            &Command::MoveDeck {
                id: parent,
                parent: Some(grandchild),
            },
            0,
        )
        .expect_err("cycle");
    assert!(
        matches!(error, StoreError::Cycle { deck } if deck == parent),
        "{error}"
    );
    let error = db
        .apply(
            &Command::MoveDeck {
                id: parent,
                parent: Some(parent),
            },
            0,
        )
        .expect_err("cycle");
    assert!(matches!(error, StoreError::Cycle { .. }), "{error}");

    // A legal move out of the tree, and one into a sibling branch, both work.
    db.apply(
        &Command::MoveDeck {
            id: grandchild,
            parent: None,
        },
        0,
    )
    .expect("move to root");
    db.apply(
        &Command::MoveDeck {
            id: child,
            parent: None,
        },
        0,
    )
    .expect("move to root");
    let roots = db
        .snapshot()
        .expect("snapshot")
        .decks
        .iter()
        .filter(|deck| deck.parent.is_none())
        .count();
    assert_eq!(roots, 3);
}

#[test]
fn deck_names_are_unique_under_one_parent_and_free_elsewhere() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let first = deck(&mut db, "Japanese");
    let second = deck(&mut db, "Spanish");

    let error = db
        .apply(
            &Command::CreateDeck {
                parent: None,
                name: "JAPANESE".to_string(),
            },
            0,
        )
        .expect_err("duplicate");
    assert!(error.to_string().contains("UNIQUE"), "{error}");

    db.apply(
        &Command::CreateDeck {
            parent: Some(first),
            name: "Verbs".to_string(),
        },
        0,
    )
    .expect("nested");
    db.apply(
        &Command::CreateDeck {
            parent: Some(second),
            name: "Verbs".to_string(),
        },
        0,
    )
    .expect("nested elsewhere");
}

#[test]
fn a_moved_card_keeps_its_prompt_and_due_date() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let source = deck(&mut db, "Source");
    let destination = deck(&mut db, "Destination");
    let card = card(&mut db, source, "front\n---\nback");
    schedule(&mut db, card, NOON - 3 * HOUR);

    let before = db.prompt(card, "fwd").expect("prompt").expect("exists");
    db.apply(
        &Command::MoveCard {
            id: card,
            deck: destination,
        },
        NOON,
    )
    .expect("move");

    assert_eq!(
        db.prompt(card, "fwd").expect("prompt").expect("exists"),
        before
    );
    let moved = db
        .snapshot()
        .expect("snapshot")
        .cards
        .into_iter()
        .find(|row| row.id == card)
        .expect("card");
    assert_eq!(moved.deck, destination);
    assert_eq!(
        db.due_prompts(NOON, 0, 1).expect("due").len(),
        1,
        "still due, now under the new deck"
    );
}

#[test]
fn tags_match_without_case_and_keep_the_first_casing() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Tags");
    let first = card(&mut db, deck, "front\n---\nback");
    let second = card(&mut db, deck, "front\n---\nback");
    let heading = card(&mut db, deck, "# Heading\n\ntext");

    db.apply(
        &Command::SetTags {
            card: first,
            tags: vec!["Grammar".into(), "grammar".into(), "verbs".into()],
        },
        0,
    )
    .expect("tags");
    db.apply(
        &Command::SetTags {
            card: second,
            tags: vec!["GRAMMAR".into()],
        },
        0,
    )
    .expect("tags");

    let summary = |id| {
        db.snapshot()
            .expect("snapshot")
            .cards
            .into_iter()
            .find(|card| card.id == id)
            .expect("card")
    };
    assert_eq!(
        summary(first).tags,
        vec!["Grammar".to_string(), "verbs".to_string()]
    );
    assert_eq!(
        summary(second).tags,
        vec!["Grammar".to_string()],
        "the first casing is library-wide"
    );
    assert_eq!(
        db.cards_with_tag("grammar").expect("filter"),
        vec![first, second]
    );
    assert_eq!(
        db.tag_names().expect("names"),
        vec!["Grammar".to_string(), "verbs".to_string()]
    );
    assert!(summary(heading).tags.is_empty(), "a heading is not a tag");
}

#[test]
fn search_folds_case_and_diacritics_within_the_deck_subtree() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let spanish = deck(&mut db, "Spanish");
    let songs = match db.apply(
        &Command::CreateDeck {
            parent: Some(spanish),
            name: "Songs".to_string(),
        },
        0,
    ) {
        Ok(Outcome::Deck(id)) => id,
        other => panic!("{other:?}"),
    };
    let other = deck(&mut db, "Other");

    let accented = card(&mut db, songs, "Canción\n---\nuna canción");
    let elsewhere = card(&mut db, other, "canción de cuna\n---\notra");
    let unrelated = card(&mut db, other, "Grammar\n---\nthe rules");

    assert_eq!(
        db.search("cancion", None).expect("search"),
        vec![accented, elsewhere]
    );
    assert_eq!(
        db.search("CANCION", Some(spanish)).expect("search"),
        vec![accented],
        "subtree only"
    );
    assert_eq!(db.search("grammar", None).expect("search"), vec![unrelated]);
    assert!(
        db.search("   ", None).expect("search").is_empty(),
        "blank matches nothing"
    );
}

#[test]
fn archiving_a_card_takes_it_out_of_the_queue_and_restoring_keeps_its_date() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Archive me");
    let card = card(&mut db, deck, "front\n---\nback");
    schedule(&mut db, card, NOON - HOUR);

    db.apply(
        &Command::ArchiveCard {
            id: card,
            archived: true,
        },
        NOON,
    )
    .expect("archive");
    assert!(db.due_prompts(NOON, 0, 1).expect("due").is_empty());
    let prompt = db
        .prompt(card, "fwd")
        .expect("prompt")
        .expect("still stored");
    assert_eq!(
        prompt.due_at,
        Some(NOON - HOUR),
        "archiving is not a schedule change"
    );

    db.apply(
        &Command::ArchiveCard {
            id: card,
            archived: false,
        },
        NOON,
    )
    .expect("restore");
    assert_eq!(db.due_prompts(NOON, 0, 1).expect("due").len(), 1);
    assert_eq!(
        db.prompt(card, "fwd")
            .expect("prompt")
            .expect("stored")
            .due_at,
        Some(NOON - HOUR)
    );
}

#[test]
fn a_card_under_an_archived_deck_leaves_the_queues_while_a_sibling_deck_stays() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let parent = deck(&mut db, "Parent");
    let hidden = match db.apply(
        &Command::CreateDeck {
            parent: Some(parent),
            name: "Hidden".to_string(),
        },
        0,
    ) {
        Ok(Outcome::Deck(id)) => id,
        other => panic!("{other:?}"),
    };
    let visible = match db.apply(
        &Command::CreateDeck {
            parent: Some(parent),
            name: "Visible".to_string(),
        },
        0,
    ) {
        Ok(Outcome::Deck(id)) => id,
        other => panic!("{other:?}"),
    };
    let hidden_card = card(&mut db, hidden, "hidden\n---\nside");
    let visible_card = card(&mut db, visible, "visible\n---\nside");
    db.apply(
        &Command::ArchiveDeck {
            id: hidden,
            archived: true,
        },
        NOON,
    )
    .expect("archive");

    let new: Vec<Id> = db
        .new_prompts(NOON, 0, 1)
        .expect("new")
        .into_iter()
        .map(|p| p.card)
        .collect();
    assert!(
        !new.contains(&hidden_card),
        "an archived ancestor hides the card"
    );
    assert!(new.contains(&visible_card));

    schedule(&mut db, hidden_card, NOON - HOUR);
    schedule(&mut db, visible_card, NOON - HOUR);
    let due: Vec<Id> = db
        .due_prompts(NOON, 0, 1)
        .expect("due")
        .into_iter()
        .map(|p| p.card)
        .collect();
    assert_eq!(due, vec![visible_card]);
}

#[test]
fn a_restored_card_keeps_its_history_and_an_emptied_one_loses_everything() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Trash");
    let kept = card(&mut db, deck, "kept\n---\nside");
    let doomed = card(&mut db, deck, "doomed\n---\nside");
    schedule(&mut db, doomed, NOON - HOUR);
    db.apply(
        &Command::Grade {
            card: doomed,
            key: "fwd".to_string(),
            grade: Grade::Remembered,
            duration_ms: None,
        },
        NOON,
    )
    .expect("grade");
    assert_eq!(db.record_total().expect("total"), 2);

    db.apply(&Command::TrashCard { id: doomed }, NOON)
        .expect("trash");
    assert!(
        db.search("doomed", None).expect("search").is_empty(),
        "trash is not searchable"
    );
    db.apply(&Command::RestoreCard { id: doomed }, NOON)
        .expect("restore");
    assert_eq!(
        db.review_count(doomed).expect("count"),
        2,
        "restoring keeps history"
    );

    db.apply(&Command::TrashCard { id: doomed }, NOON)
        .expect("trash again");
    db.apply(&Command::EmptyTrash, NOON).expect("empty");

    assert_eq!(
        db.record_total().expect("total"),
        0,
        "the log went with the card"
    );
    let cards = db.snapshot().expect("snapshot").cards;
    assert_eq!(
        cards.iter().map(|row| row.id).collect::<Vec<_>>(),
        vec![kept]
    );
    assert!(db.search("doomed", None).expect("search").is_empty());
    assert!(db.prompt(doomed, "fwd").expect("prompt").is_none());
}

#[test]
fn an_unsaved_draft_survives_reopening_the_database() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("library.sqlite3");
    {
        let mut db = Db::open(&path).unwrap();
        db.apply(
            &Command::SetDraft {
                card: 0,
                text: Some("half a card".to_string()),
            },
            NOON,
        )
        .unwrap();
    }

    let mut db = Db::open(&path).unwrap();
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
fn a_percent_or_underscore_in_the_query_matches_itself() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Search");
    card(&mut db, deck, "Off by 100%\n---\nback");
    card(&mut db, deck, "Off by 100 percent\n---\nback");

    assert_eq!(
        db.search("100%", None).expect("search").len(),
        1,
        "only the literal percent matches"
    );
    assert!(
        db.search("_", None).expect("search").is_empty(),
        "an underscore is not a single-character wildcard"
    );
}
