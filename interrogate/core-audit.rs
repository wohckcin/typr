//! Audit probes: behaviours the specs require, run against the store directly.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use typr_core::clock::Clock;
use typr_core::store::{CardMode, Command, Db, Handle, Outcome, Store, Waker};

fn open() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().expect("temp dir");
    let db = Db::open(&dir.path().join("library.sqlite3")).expect("open");
    (dir, db)
}

fn deck(db: &Db, name: &str) -> i64 {
    match db
        .apply(
            &Command::CreateDeck {
                parent: None,
                name: name.to_string(),
            },
            0,
        )
        .expect("deck")
    {
        Outcome::Deck(id) => id,
        other => panic!("expected a deck, got {other:?}"),
    }
}

fn card(db: &Db, deck: i64, markdown: &str) -> i64 {
    match db
        .apply(
            &Command::CreateCard {
                deck,
                markdown: markdown.to_string(),
            },
            0,
        )
        .expect("card")
    {
        Outcome::Card(id) => id,
        other => panic!("expected a card, got {other:?}"),
    }
}

#[test]
fn search_finds_a_card_by_its_tag() {
    let (_dir, db) = open();
    let deck = deck(&db, "Spanish");
    let card = card(&db, deck, "What is the subjunctive?\n---\nA mood, not a tense.");
    db.apply(
        &Command::SetTags {
            card,
            tags: vec!["Grammar".to_string()],
        },
        0,
    )
    .expect("tags");

    let snapshot = db.snapshot().expect("snapshot");
    let stored = &snapshot.cards[0];
    println!(
        "card {} tags {:?} search_text {:?}",
        stored.id, stored.tags, stored.search_text
    );

    let by_tag = snapshot.cards_matching("grammar", None, None, CardMode::Active);
    let by_prefix = snapshot.cards_matching("gram", None, None, CardMode::Active);
    let by_content = snapshot.cards_matching("subjunctive", None, None, CardMode::Active);
    println!(
        "search 'grammar' -> {:?}, 'gram' -> {:?}, 'subjunctive' -> {:?}",
        by_tag.iter().map(|c| c.id).collect::<Vec<_>>(),
        by_prefix.iter().map(|c| c.id).collect::<Vec<_>>(),
        by_content.iter().map(|c| c.id).collect::<Vec<_>>(),
    );

    assert_eq!(by_content.len(), 1, "content search works");
    assert_eq!(
        by_tag.len(),
        1,
        "the card-library spec requires a text search over card content AND tags"
    );
}

/// A clock that is slow the first time it is read and panics the second, so two commands are
/// already queued, and the second one is the one that fails.
struct SlowThenStops(AtomicUsize);

impl Clock for SlowThenStops {
    fn now_ms(&self) -> i64 {
        match self.0.fetch_add(1, Ordering::SeqCst) {
            0 => {
                std::thread::sleep(std::time::Duration::from_millis(150));
                0
            }
            _ => panic!("the clock fell over"),
        }
    }
}

#[test]
fn the_command_that_panicked_gets_its_answer_and_the_first_gets_one() {
    let dir = tempfile::tempdir().expect("temp dir");
    let db = Db::open(&dir.path().join("library.sqlite3")).expect("open");
    let waker: Waker = Arc::new(|| {});
    let handle = Handle::spawn(db, Box::new(SlowThenStops(AtomicUsize::new(0))), waker).expect("spawn");

    // Two requests, both accepted before the first one is answered.
    let first = handle.history(1).expect("accepted");
    let second = handle.stats(0, 1).expect("accepted");

    let mut answers: Vec<(u64, String)> = Vec::new();
    for _ in 0..200 {
        for event in handle.poll() {
            answers.push((event.id, format!("{:?}", event.result)));
        }
        if answers.iter().any(|(id, _)| *id == 0) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    println!("first={first} second={second}");
    for (id, result) in &answers {
        println!("answer id={id}: {result}");
    }
    let for_first = answers.iter().filter(|(id, _)| *id == first).count();
    let for_second = answers.iter().filter(|(id, _)| *id == second).count();
    println!("answers: first={for_first} second={for_second}");
    assert_eq!(for_first, 1, "the first request gets exactly one answer");
    assert_eq!(
        for_second, 1,
        "the request that was being applied when the worker failed gets an answer"
    );
}

/// A card forgotten while learning has no schedule; it must still be in the new queue.
#[test]
fn a_new_card_forgotten_once_is_still_new() {
    let (_dir, db) = open();
    let deck = deck(&db, "Biology");
    let card = card(&db, deck, "front\n---\nback");

    let new_before = match db
        .apply(
            &Command::NewPrompts {
                now_ms: 1_000,
                offset_ms: 0,
                seed: 7,
            },
            0,
        )
        .expect("new")
    {
        Outcome::New(prompts) => prompts,
        other => panic!("expected the new queue, got {other:?}"),
    };
    println!(
        "new queue before: {:?}",
        new_before.iter().map(|p| p.card).collect::<Vec<_>>()
    );

    // A failure while learning records the grade and leaves the schedule alone.
    let outcome = db
        .apply(
            &Command::Grade {
                card,
                key: "fwd".to_string(),
                grade: typr_core::store::Grade::Forgot,
                duration_ms: None,
            },
            1_000,
        )
        .expect("grade");
    println!("grade answered {outcome:?}");
    println!(
        "prompt after: {:?}",
        db.prompt(card, "fwd").expect("prompt")
    );

    let new_after = match db
        .apply(
            &Command::NewPrompts {
                now_ms: 2_000,
                offset_ms: 0,
                seed: 7,
            },
            0,
        )
        .expect("new")
    {
        Outcome::New(prompts) => prompts,
        other => panic!("expected the new queue, got {other:?}"),
    };
    let due_after = match db
        .apply(
            &Command::DuePrompts {
                now_ms: 2_000,
                offset_ms: 0,
                seed: 7,
            },
            0,
        )
        .expect("due")
    {
        Outcome::Due(prompts) => prompts,
        other => panic!("expected the due queue, got {other:?}"),
    };
    println!(
        "new queue after: {:?}, due queue after: {:?}",
        new_after.iter().map(|p| p.card).collect::<Vec<_>>(),
        due_after.iter().map(|p| p.card).collect::<Vec<_>>()
    );
    assert_eq!(
        new_after.iter().map(|p| p.card).collect::<Vec<_>>(),
        vec![card],
        "a learn-phase failure leaves no scheduling state, so the card is still new"
    );
}

#[test]
fn an_unreadable_library_is_reported_with_its_path() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("library.sqlite3");
    std::fs::write(&path, b"this file is not a database of ours").expect("write");
    let before = std::fs::read(&path).expect("read");

    let error = Db::open(&path).expect_err("a foreign file is refused");
    let mut line = error.to_string();
    let mut cause = std::error::Error::source(&error);
    while let Some(inner) = cause {
        line.push_str(": ");
        line.push_str(&inner.to_string());
        cause = inner.source();
    }
    println!("path {path:?}");
    println!("failure line {line:?}");
    let after = std::fs::read(&path).expect("read");
    println!("file left alone: {}", after == before);
    assert!(
        line.contains(path.to_str().expect("utf8")),
        "the failure the user sees has to name the file"
    );
    assert_eq!(
        std::fs::read(&path).expect("read"),
        before,
        "and the file is left alone"
    );
}
