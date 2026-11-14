//! Adversarial repros written during review at c77c60b. Each test asserts what the spec requires;
//! a failure is the finding. NOT YET RUN: the reviewing session had no shell access.

use typr_core::clock::LEARNING_STEP_MS;
use typr_core::screen::Queue;
use typr_core::session::{Entry, Session};
use typr_core::store::{Command, Db, Grade, Id, Outcome};

const NOON: i64 = 1_760_000_000_000;
const DAY: i64 = 24 * 60 * 60 * 1000;

fn entry(card: Id) -> Entry {
    Entry {
        card,
        sides: vec!["front".to_string(), "back".to_string()],
        overdue: 0,
    }
}

/// Reveal, grade, send under `id`, acknowledge. Returns the card graded.
fn answer(session: &mut Session, grade: Grade, id: u64) -> Id {
    while session.hidden() > 0 {
        assert!(session.reveal());
    }
    let (card, _) = session.grade(grade).expect("gradeable");
    session.sent(id);
    assert!(session.acknowledge(id, NOON));
    card
}

/// Grade everything left with `remembered`, returning the cards in the order they were graded.
fn finish(session: &mut Session, mut id: u64) -> Vec<Id> {
    let mut graded = Vec::new();
    while session.current().is_some() {
        graded.push(answer(session, Grade::Remembered, id));
        id += 1;
    }
    graded
}

/// study-session: "moving forward past a card without grading it SHALL leave that card in the
/// queue"; "exactly one grade exists for that card".
#[test]
fn forward_while_a_grade_is_in_flight_skips_a_card_and_regrades_another() {
    let mut s = Session::new(Queue::Due, vec![entry(1), entry(2), entry(3)]);
    s.reveal();
    let (card, _) = s.grade(Grade::Remembered).expect("gradeable");
    assert_eq!(card, 1);
    s.sent(10);
    assert!(s.forward(), "Right is accepted while the grade is in flight");
    assert!(s.acknowledge(10, NOON));

    let rest = finish(&mut s, 11);
    assert_eq!(rest, vec![2, 3], "card 2 offered once, card 1 not graded twice");
}

/// study-session: "Moving backward SHALL show an earlier card read-only and SHALL NOT accept a
/// second grade"; a reshuffle "SHALL NOT change any prompt's scheduling state".
#[test]
fn reshuffle_while_revisiting_regrades_an_answered_card() {
    let mut broken = Vec::new();
    for seed in 1..=64u64 {
        let mut s = Session::new(Queue::New, (1..=6).map(entry).collect());
        assert_eq!(answer(&mut s, Grade::Remembered, 1), 1);
        assert!(s.backward());
        s.reshuffle(seed);
        assert!(s.forward());
        let rest = finish(&mut s, 2);
        let mut sorted = rest.clone();
        sorted.sort();
        if sorted != vec![2, 3, 4, 5, 6] {
            broken.push((seed, rest));
        }
    }
    assert!(
        broken.is_empty(),
        "(seed, cards graded after revisit+reshuffle): {broken:?}"
    );
}

/// study-session: "A relearning step returns before the session ends" — for the card that was
/// forgotten, not the one on screen when the answer landed.
#[test]
fn backward_while_a_forgot_is_in_flight_requeues_the_wrong_card() {
    let mut s = Session::new(Queue::Due, vec![entry(1), entry(2), entry(3)]);
    assert_eq!(answer(&mut s, Grade::Remembered, 1), 1);
    while s.hidden() > 0 {
        s.reveal();
    }
    let (card, _) = s.grade(Grade::Forgot).expect("gradeable");
    assert_eq!(card, 2);
    s.sent(2);
    assert!(s.backward(), "Left is accepted while the grade is in flight");
    assert!(s.acknowledge(2, NOON));
    s.forward();
    finish(&mut s, 3);
    s.advance_clock(NOON + LEARNING_STEP_MS);
    assert_eq!(
        s.current().map(|e| e.card),
        Some(2),
        "the forgotten card returns, not the remembered one"
    );
}

fn library() -> (tempfile::TempDir, Db, Id) {
    let dir = tempfile::tempdir().expect("temp dir");
    let db = Db::open(&dir.path().join("typr.sqlite3")).expect("open");
    let Ok(Outcome::Deck(deck)) = db.apply(
        &Command::CreateDeck {
            parent: None,
            name: "D".to_string(),
        },
        NOON,
    ) else {
        panic!("deck");
    };
    let Ok(Outcome::Card(card)) = db.apply(
        &Command::CreateCard {
            deck,
            markdown: "Q\n---\nA".to_string(),
        },
        NOON,
    ) else {
        panic!("card");
    };
    (dir, db, card)
}

/// review-scheduling: "A learn-phase forgot SHALL leave scheduling state unchanged";
/// study-session: the new queue holds "prompts of studyable cards that have no scheduling state";
/// "a session abandoned halfway resumes with ... the remaining ones still queued".
#[test]
fn a_new_card_forgotten_then_abandoned_is_in_neither_queue() {
    let (_dir, db, card) = library();
    db.apply(
        &Command::Grade {
            card,
            key: "fwd".to_string(),
            grade: Grade::Forgot,
            duration_ms: None,
        },
        NOON,
    )
    .expect("graded");
    // The session is abandoned here. Next launch, a day later:
    let later = NOON + DAY;
    let in_new = db
        .new_prompts(later, 0, 1)
        .expect("new")
        .iter()
        .any(|p| p.card == card);
    let in_due = db
        .due_prompts(later + 365 * DAY, 0, 1)
        .expect("due")
        .iter()
        .any(|p| p.card == card);
    assert!(
        in_new || in_due,
        "card {card} is in no queue: new={in_new} due={in_due}"
    );
}
