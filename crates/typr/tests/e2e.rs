//! One session end to end through the public store API: create, learn, wait a day, review.

use std::path::Path;

use typr::store::{
    Command, Db, DuePrompt, Grade, Id, NewPrompt, Outcome, Phase, ReviewRow, StoreError,
};
use typr_core::clock::{Clock, TestClock};
use typr_core::day;

const NOON: i64 = 1_760_000_000_000;
const DAY: i64 = 24 * 60 * 60 * 1000;

fn open(dir: &Path) -> Db {
    Db::open(&dir.join("typr.sqlite3")).expect("a fresh library")
}

/// The one grade command a study session sends.
fn grade(db: &mut Db, card: Id, grade: Grade, now: i64) -> Result<Outcome, StoreError> {
    db.apply(
        &Command::Grade {
            card,
            key: "fwd".to_string(),
            grade,
            duration_ms: Some(2_000),
        },
        now,
    )
}

fn due(db: &mut Db, now: i64) -> Vec<DuePrompt> {
    db.due_prompts(now, 0, 7).expect("the due queue")
}

fn new(db: &mut Db, now: i64) -> Vec<NewPrompt> {
    db.new_prompts(now, 0, 7).expect("the new queue")
}

fn history(db: &Db, card: Id) -> Vec<ReviewRow> {
    db.history(card).expect("a card's records")
}

#[test]
fn a_card_is_learned_then_reviewed_the_next_day() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let mut db = open(dir.path());
    let clock: Box<dyn Clock + Send + Sync> = Box::new(TestClock::new(NOON));

    let Ok(Outcome::Deck(deck)) = db.apply(
        &Command::CreateDeck {
            parent: None,
            name: "Biology".to_string(),
        },
        clock.now_ms(),
    ) else {
        panic!("the deck is created");
    };
    let Ok(Outcome::Card(card)) = db.apply(
        &Command::CreateCard {
            deck,
            markdown: "What is **mitosis**?\n---\nCell division.".to_string(),
        },
        clock.now_ms(),
    ) else {
        panic!("the card is created");
    };

    // It starts in the new queue and nowhere else.
    assert_eq!(
        new(&mut db, NOON)
            .iter()
            .map(|p| p.card)
            .collect::<Vec<_>>(),
        vec![card]
    );
    assert!(
        due(&mut db, NOON).is_empty(),
        "a card never answered is not due"
    );

    // Learning: remembered, so the card graduates to a ten-minute step, then to a day.
    grade(&mut db, card, Grade::Remembered, NOON).expect("the graduating grade");
    assert!(
        due(&mut db, NOON).is_empty(),
        "ten minutes out is not due yet"
    );
    assert_eq!(
        due(&mut db, NOON + 11 * 60 * 1000).len(),
        1,
        "the learning step comes due"
    );
    assert!(
        new(&mut db, NOON + 11 * 60 * 1000).is_empty(),
        "a graduated card is no longer new"
    );

    grade(&mut db, card, Grade::Remembered, NOON + 11 * 60 * 1000).expect("the graduation grade");
    assert!(
        due(&mut db, NOON + 11 * 60 * 1000).is_empty(),
        "one day out is not due"
    );
    assert_eq!(
        due(&mut db, NOON + DAY + 11 * 60 * 1000).len(),
        1,
        "a day later it is due"
    );

    // A day later, a review: remembered, so the interval doubles-ish and the record keeps the phase.
    grade(
        &mut db,
        card,
        Grade::Remembered,
        NOON + DAY + 11 * 60 * 1000,
    )
    .expect("the review");
    let rows = history(&db, card);
    assert_eq!(
        rows.len(),
        3,
        "the graduating grade and two reviews, all kept"
    );
    assert_eq!(
        rows.iter().filter(|row| row.phase == Phase::Review).count(),
        3,
        "the grade that graduates a card counts as a review"
    );
    assert_eq!(db.review_count(card).expect("the count"), 3);
    assert!(rows.iter().all(|row| row.grade == Grade::Remembered));
    assert_eq!(
        rows.last().and_then(|row| row.duration_ms),
        Some(2_000),
        "the answer time is kept"
    );

    // Statistics cover the study days the grades fell on, from the store rather than the view.
    let today = day::study_day(NOON, 0);
    let stats = db.stats_for_day(today, 0).expect("a day's statistics");
    assert_eq!(stats.reviews, 2, "the graduating grade and the review");
    assert_eq!(stats.lapses, 0);
}

#[test]
fn a_forgotten_card_returns_in_ten_minutes_and_survives_a_reopen() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let mut db = open(dir.path());
    let Ok(Outcome::Deck(deck)) = db.apply(
        &Command::CreateDeck {
            parent: None,
            name: "Latin".to_string(),
        },
        NOON,
    ) else {
        panic!("the deck is created");
    };
    let Ok(Outcome::Card(card)) = db.apply(
        &Command::CreateCard {
            deck,
            markdown: "amo\n---\nI love".to_string(),
        },
        NOON,
    ) else {
        panic!("the card is created");
    };

    grade(&mut db, card, Grade::Remembered, NOON).expect("learned once");
    grade(&mut db, card, Grade::Remembered, NOON + 11 * 60 * 1000).expect("graduated");
    let review_at = NOON + DAY + 11 * 60 * 1000;
    grade(&mut db, card, Grade::Forgot, review_at).expect("forgotten");

    assert!(
        due(&mut db, review_at).is_empty(),
        "the relearning step is ten minutes out"
    );
    let relearn = due(&mut db, review_at + 11 * 60 * 1000);
    assert_eq!(relearn.len(), 1, "and then it is due");
    assert_eq!(relearn[0].due_at, review_at + 10 * 60 * 1000);
    assert_eq!(
        history(&db, card).last().map(|row| row.phase),
        Some(Phase::Relearn)
    );

    drop(db);
    let mut reopened = open(dir.path());
    assert_eq!(
        reopened.history(card).expect("the records survive").len(),
        3
    );
    assert_eq!(
        due(&mut reopened, review_at + 11 * 60 * 1000).len(),
        1,
        "and the date does too"
    );
    assert_eq!(reopened.review_count(card).expect("the count"), 3);
}
