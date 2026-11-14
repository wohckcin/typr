use super::{Entry, Session};
use crate::clock::LEARNING_STEP_MS;
use crate::screen::Queue;
use crate::store::Grade;

const NOON: i64 = 1_760_000_000_000;

fn entry(card: i64, sides: &[&str]) -> Entry {
    Entry {
        card: card as crate::store::Id,
        sides: sides.iter().map(|s| s.to_string()).collect(),
        overdue: 0,
    }
}

/// Answers the card on screen: reveal everything, grade, and acknowledge.
fn answer(session: &mut Session, grade: Grade, now_ms: i64) {
    while session.hidden() > 0 {
        assert!(session.reveal());
    }
    let (_, sent) = session.grade(grade).expect("the grade is offered");
    assert_eq!(sent, grade);
    session.sent(7);
    assert!(session.acknowledge(7, now_ms));
}

#[test]
fn a_new_card_is_scheduled_only_after_its_last_side() {
    let mut session = Session::new(Queue::New, vec![entry(1, &["front"]), entry(2, &["front"])]);
    assert_eq!(session.current().unwrap().card, 1);
    assert!(
        !session.gradeable(),
        "nothing is graded before the last side"
    );
    assert_eq!(session.grade(Grade::Remembered), None);

    assert!(session.reveal());
    assert!(session.gradeable());

    answer(&mut session, Grade::Remembered, NOON);
    assert_eq!(
        session.current().unwrap().card,
        2,
        "the next new card follows"
    );
    assert_eq!(session.grades, vec![Grade::Remembered]);
}

#[test]
fn a_three_sided_card_is_one_review() {
    let mut session = Session::new(Queue::New, vec![entry(1, &["one", "two", "three"])]);
    assert_eq!(session.next_side(), Some("one"));
    assert!(session.reveal());
    assert_eq!(session.next_side(), Some("two"));
    assert!(!session.gradeable(), "a side is still hidden");
    assert!(session.reveal());
    assert!(session.reveal());
    assert!(!session.reveal(), "there is no fourth side");
    assert!(session.gradeable());
    assert_eq!(session.grade(Grade::Remembered).map(|(c, _)| c), Some(1));

    session.sent(1);
    session.acknowledge(1, NOON);
    assert_eq!(session.grades.len(), 1, "one review, three sides");
    assert!(session.finished());
}

#[test]
fn a_forgotten_new_card_returns_last_in_the_queue() {
    let mut session = Session::new(
        Queue::New,
        vec![
            entry(1, &["a"]),
            entry(2, &["b"]),
            entry(3, &["c"]),
            entry(4, &["d"]),
        ],
    );

    answer(&mut session, Grade::Forgot, NOON);
    let mut order = Vec::new();
    while !session.finished() {
        order.push(session.current().unwrap().card);
        answer(&mut session, Grade::Remembered, NOON);
    }
    assert_eq!(
        order,
        vec![2, 3, 4, 1],
        "the forgotten card comes back last"
    );
    assert_eq!(session.remaining(), 0);
}

#[test]
fn going_back_shows_a_graded_card_read_only_and_forward_returns() {
    let mut session = Session::new(
        Queue::New,
        vec![entry(1, &["one", "two"]), entry(2, &["front"])],
    );
    answer(&mut session, Grade::Remembered, NOON);
    assert_eq!(session.current().unwrap().card, 2);

    assert!(session.backward());
    assert!(session.read_only);
    assert_eq!(session.current().unwrap().card, 1);
    assert_eq!(session.revealed(), 2, "its answer is shown");
    assert!(!session.gradeable(), "and it cannot be graded twice");

    assert!(session.forward(), "forward leaves the read-only view");
    assert!(!session.read_only);
    assert_eq!(session.current().unwrap().card, 2);
}

#[test]
fn going_forward_leaves_an_ungraded_card_queued() {
    let mut session = Session::new(Queue::New, vec![entry(1, &["a"]), entry(2, &["b"])]);
    assert!(session.forward());
    assert_eq!(session.current().unwrap().card, 2);
    assert_eq!(
        session.remaining(),
        2,
        "the skipped card is still to answer"
    );

    answer(&mut session, Grade::Remembered, NOON);
    assert_eq!(
        session.current().unwrap().card,
        1,
        "and it comes round again"
    );
}

#[test]
fn quitting_five_of_twelve_leaves_seven_to_answer() {
    let entries: Vec<Entry> = (1..=12).map(|card| entry(card, &["front"])).collect();
    let mut session = Session::new(Queue::Due, entries.clone());
    for _ in 0..5 {
        answer(&mut session, Grade::Remembered, NOON);
    }
    assert_eq!(session.grades.len(), 5);
    assert_eq!(session.remaining(), 7, "the rest are still due");

    // The next session is built from what is still unscheduled, which is exactly these seven.
    let resumed = Session::new(Queue::Due, entries[5..].to_vec());
    assert_eq!(resumed.remaining(), 7);
}

#[test]
fn a_forgotten_due_card_returns_when_its_relearning_step_lands() {
    let mut session = Session::new(Queue::Due, vec![entry(1, &["front"]), entry(2, &["back"])]);
    answer(&mut session, Grade::Forgot, NOON);
    assert_eq!(session.current().unwrap().card, 2);
    assert_eq!(
        session.remaining(),
        2,
        "the forgotten card is waiting, not queued"
    );

    assert_eq!(
        session.advance_clock(NOON + LEARNING_STEP_MS - 1),
        0,
        "not yet"
    );
    assert_eq!(
        session.advance_clock(NOON + LEARNING_STEP_MS),
        1,
        "ten minutes later it is back"
    );
    assert_eq!(session.current().unwrap().card, 2);
    answer(&mut session, Grade::Remembered, NOON);
    assert_eq!(
        session.current().unwrap().card,
        1,
        "the relearning step returns within the session"
    );
}

#[test]
fn a_prompt_arriving_mid_session_joins_the_end() {
    let mut session = Session::new(Queue::Due, vec![entry(1, &["front"])]);
    session.arrive(entry(2, &["front"]));
    answer(&mut session, Grade::Remembered, NOON);
    assert_eq!(session.current().unwrap().card, 2);
}

#[test]
fn a_grade_is_ignored_until_the_store_answers() {
    let mut session = Session::new(Queue::New, vec![entry(1, &["front"]), entry(2, &["front"])]);
    session.reveal();
    assert!(session.grade(Grade::Remembered).is_some());
    session.sent(3);
    assert_eq!(session.grade(Grade::Forgot), None, "one grade at a time");
    assert!(
        !session.acknowledge(4, NOON),
        "a different id is not this grade"
    );
    assert!(session.acknowledge(3, NOON));
    assert_eq!(session.grades, vec![Grade::Remembered]);
}

#[test]
fn a_refused_grade_frees_the_card_again() {
    let mut session = Session::new(Queue::Due, vec![entry(1, &["front"])]);
    assert!(session.reveal());

    let (_, grade) = session.grade(Grade::Remembered).expect("offered");
    assert_eq!(grade, Grade::Remembered);
    session.sent(4);
    assert!(!session.gradeable(), "the card waits on the store");

    session.refuse(4);
    assert!(session.gradeable(), "a refused grade frees the card");
    assert_eq!(session.remaining(), 1, "and the card is still unanswered");
    assert!(session.grade(Grade::Remembered).is_some());
}

#[test]
fn a_grade_that_was_never_sent_can_be_taken_again() {
    let mut session = Session::new(Queue::New, vec![entry(1, &["front"])]);
    assert!(session.reveal());
    assert!(session.grade(Grade::Forgot).is_some());
    session.refuse(0);
    assert!(session.gradeable());
}

fn overdue_entry(card: i64, overdue: i64) -> Entry {
    Entry {
        card: card as crate::store::Id,
        sides: vec!["front".to_string(), "back".to_string()],
        overdue,
    }
}

#[test]
fn a_reshuffle_reorders_only_within_one_due_day() {
    let mut session = Session::new(
        Queue::Due,
        vec![
            overdue_entry(1, 0),
            overdue_entry(2, 0),
            overdue_entry(3, 0),
            overdue_entry(4, 2),
        ],
    );
    let mut orders = Vec::new();
    for seed in 1..40 {
        session.reshuffle(seed);
        let order = session.order();
        assert_eq!(
            order.len(),
            4,
            "a reshuffle never loses a card from the queue"
        );
        assert_eq!(
            *order.last().expect("a card"),
            4,
            "the card that is not due stays behind the ones that are"
        );
        let mut front = order[..3].to_vec();
        front.sort_unstable();
        assert_eq!(
            front,
            vec![1, 2, 3],
            "the due cards keep their membership, only their order changes"
        );
        orders.push(order);
    }
    let distinct = {
        let mut unique = orders.clone();
        unique.sort();
        unique.dedup();
        unique.len()
    };
    assert!(
        distinct > 1,
        "different seeds give different orders, or the shuffle does nothing"
    );
}

#[test]
fn a_grade_lands_on_the_card_it_was_given_for() {
    let mut session = Session::new(
        Queue::Due,
        vec![entry(1, &["a"]), entry(2, &["b"]), entry(3, &["c"])],
    );
    assert!(session.reveal());
    assert_eq!(
        session.grade(Grade::Remembered),
        Some((1, Grade::Remembered))
    );
    session.sent(11);
    assert!(session.forward(), "the user moves on before the answer lands");

    assert!(session.acknowledge(11, NOON));

    assert_eq!(
        session.order(),
        vec![2, 3],
        "the graded card leaves the queue and nothing else moves"
    );
    assert_eq!(session.current().map(|entry| entry.card), Some(2));
}

#[test]
fn a_reshuffle_never_hands_over_a_revealed_card() {
    let cards = || -> Vec<Entry> { (1..=8).map(|card| entry(card, &["a"])).collect() };
    let mut moved = 0;
    for seed in 1..40u64 {
        let mut session = Session::new(Queue::Due, cards());
        assert!(session.reveal());
        let before = session.current().map(|entry| entry.card);
        session.reshuffle(seed);
        if session.current().map(|entry| entry.card) == before {
            assert_eq!(session.revealed(), 1, "seed {seed} kept the card and its reveal");
            continue;
        }
        moved += 1;
        assert_eq!(session.revealed(), 0, "seed {seed} carried the reveal along");
        assert!(!session.gradeable(), "seed {seed} offered a grade for a side never shown");
    }
    assert!(moved > 0, "no seed reordered the front of the queue");
}
