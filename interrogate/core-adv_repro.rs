//! Adversarial repros. Each test asserts the behaviour the spec requires; a failure is the finding.

use typr_core::screen::Queue;
use typr_core::session::{Entry, Session};
use typr_core::store::{Command, Db, Grade, Id, Outcome};

const NOON: i64 = 1_760_000_000_000;

fn entry(card: Id) -> Entry {
    Entry {
        card,
        sides: vec![format!("front {card}"), format!("back {card}")],
        overdue: 0,
    }
}

fn session(cards: &[Id]) -> Session {
    Session::new(Queue::New, cards.iter().copied().map(entry).collect())
}

/// Reveal every side of the current card and take one acknowledged grade under `id`.
fn grade_current(s: &mut Session, grade: Grade, id: u64) {
    while s.reveal() {}
    s.grade(grade).expect("gradeable after the last side");
    s.sent(id);
    assert!(s.acknowledge(id, NOON), "ack lands");
}

// study-session / "Two distinct study queues" + review-scheduling / "Learning cycles never
// change the schedule": a learn-phase forgot leaves the prompt with no scheduling state, so after
// the session is left it must still be in the new-card queue.
#[test]
fn learn_forgot_card_stays_in_the_new_queue_after_quitting() {
    let dir = tempfile::tempdir().expect("tmp");
    let db = Db::open(&dir.path().join("typr.sqlite3")).expect("open");
    let Ok(Outcome::Deck(deck)) = db.apply(
        &Command::CreateDeck {
            parent: None,
            name: "D".into(),
        },
        NOON,
    ) else {
        panic!("deck")
    };
    let Ok(Outcome::Card(card)) = db.apply(
        &Command::CreateCard {
            deck,
            markdown: "q\n---\na".into(),
        },
        NOON,
    ) else {
        panic!("card")
    };
    db.apply(
        &Command::Grade {
            card,
            key: "fwd".into(),
            grade: Grade::Forgot,
            duration_ms: None,
        },
        NOON,
    )
    .expect("learn forgot");
    let later = NOON + 60_000;
    let new: Vec<Id> = db
        .new_prompts(later, 0, 7)
        .expect("new")
        .into_iter()
        .map(|p| p.card)
        .collect();
    let due: Vec<Id> = db
        .due_prompts(later, 0, 7)
        .expect("due")
        .into_iter()
        .map(|p| p.card)
        .collect();
    println!("after learn-forgot: new={new:?} due={due:?}");
    assert!(
        new.contains(&card) || due.contains(&card),
        "card {card} is in neither queue after a learn-phase forgot"
    );
}

// study-session / "The session is fully keyboard-operable": Left, R, Right must not let an
// already-graded card be graded again nor drop an ungraded one.
#[test]
fn reshuffle_while_revisiting_does_not_regrade_or_drop() {
    let mut broken = Vec::new();
    for seed in 1..64u64 {
        let mut s = session(&[1, 2, 3, 4, 5]);
        grade_current(&mut s, Grade::Remembered, 1);
        assert!(s.backward());
        s.reshuffle(seed);
        assert!(s.forward());
        let order = s.order();
        let mut sorted = order.clone();
        sorted.sort();
        if sorted != vec![2, 3, 4, 5] {
            broken.push((seed, order));
        }
    }
    println!("seeds where the remaining queue is wrong (seed, order): {broken:?}");
    assert!(broken.is_empty(), "{} seeds broke the queue", broken.len());
}

// study-session / "moving forward past a card without grading it SHALL leave that card in the
// queue": Right pressed between sending a grade and its ack.
#[test]
fn forward_before_ack_does_not_skip_the_next_card() {
    let mut s = session(&[1, 2, 3]);
    while s.reveal() {}
    s.grade(Grade::Remembered).expect("gradeable");
    s.sent(1);
    let moved = s.forward();
    assert!(s.acknowledge(1, NOON));
    let order = s.order();
    println!("forward-while-awaiting moved={moved} remaining={order:?}");
    let mut sorted = order.clone();
    sorted.sort();
    assert_eq!(sorted, vec![2, 3], "graded card 1 re-offered or card 2 dropped");
}

// study-session / "Sides are revealed in order": a reshuffle must not hand the next card over
// with sides already revealed.
#[test]
fn reshuffle_does_not_reveal_the_next_card() {
    let mut broken = Vec::new();
    for seed in 1..64u64 {
        let mut s = session(&[1, 2, 3]);
        while s.reveal() {}
        s.reshuffle(seed);
        if s.current().map(|e| e.card) != Some(1) && s.gradeable() {
            broken.push((seed, s.current().map(|e| e.card), s.revealed()));
        }
    }
    println!("seeds where a different card is gradeable unrevealed: {broken:?}");
    assert!(broken.is_empty());
}

// study-session / "SHALL show the first side of a prompt's card".
#[test]
fn a_new_session_shows_the_first_side() {
    let s = session(&[1]);
    println!("revealed at start = {}", s.revealed());
    assert!(s.revealed() >= 1, "no side is shown when the card comes up");
}
