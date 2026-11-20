use super::{Entry, Mode, Session};
use crate::clock::LEARNING_STEP_MS;
use crate::screen::Queue;
use crate::store::{Grade, MaskRect};

const NOON: i64 = 1_760_000_000_000;

fn entry(card: i64, sides: &[&str]) -> Entry {
    Entry::sides(
        card as crate::store::Id,
        sides.iter().map(|s| s.to_string()),
        0,
    )
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
    let mut session = Session::new(
        Queue::New,
        vec![entry(1, &["front", "back"]), entry(2, &["front", "back"])],
    );
    assert_eq!(session.current().unwrap().card, 1);
    assert_eq!(session.revealed(), 1, "the card opens on its first side");
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
    assert_eq!(session.revealed(), 1, "the card opens on its first side");
    assert_eq!(session.next_side(), Some("two"));
    assert!(!session.gradeable(), "a side is still hidden");
    assert!(session.reveal());
    assert_eq!(session.next_side(), Some("three"));
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
    assert!(session.grade(Grade::Forgot).is_some());
    session.refuse(0);
    assert!(session.gradeable());
}

fn overdue_entry(card: i64, overdue: i64) -> Entry {
    Entry::sides(card as crate::store::Id, ["front", "back"], overdue)
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
    assert_eq!(
        session.grade(Grade::Remembered),
        Some((1, Grade::Remembered))
    );
    session.sent(11);
    assert!(
        session.forward(),
        "the user moves on before the answer lands"
    );

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
    let cards = || -> Vec<Entry> { (1..=8).map(|card| entry(card, &["a", "b"])).collect() };
    let mut moved = 0;
    for seed in 1..40u64 {
        let mut session = Session::new(Queue::Due, cards());
        assert!(session.reveal(), "the second side is on screen");
        assert!(session.gradeable());
        let before = session.current().map(|entry| entry.card);
        session.reshuffle(seed);
        if session.current().map(|entry| entry.card) == before {
            assert_eq!(
                session.revealed(),
                2,
                "seed {seed} kept the card and its reveal"
            );
            continue;
        }
        moved += 1;
        assert_eq!(
            session.revealed(),
            1,
            "seed {seed} opens the new card on its first side"
        );
        assert!(
            !session.gradeable(),
            "seed {seed} offered a grade for a side never shown"
        );
    }
    assert!(moved > 0, "no seed reordered the front of the queue");
}

#[test]
fn a_card_that_arrives_with_a_schedule_waits_out_its_learning_step() {
    let mut session = Session::new(
        Queue::New,
        vec![Entry::sides(1, ["a".to_string(), "b".to_string()], 0)],
    );
    session.arrive(Entry::sides(2, ["a".to_string(), "b".to_string()], 0));

    // The card the session opened with is new, so a lapse sends it straight back.
    session.reveal();
    assert!(session.grade(Grade::Forgot).is_some());
    session.sent(1);
    assert!(session.acknowledge(1, 0));
    assert!(
        !session.waiting.iter().any(|(_, entry)| entry.card == 1),
        "a card that never had a schedule is not made to wait"
    );
    assert_eq!(session.current().map(|entry| entry.card), Some(2));

    // The card that arrived with a schedule behind it waits out its relearning step.
    session.reveal();
    assert!(session.grade(Grade::Forgot).is_some());
    session.sent(2);
    assert!(session.acknowledge(2, 0));
    assert!(
        session.waiting.iter().any(|(_, entry)| entry.card == 2),
        "the card that arrived scheduled keeps its relearning step"
    );
}

#[test]
fn a_card_landing_at_an_empty_cursor_shows_its_first_side() {
    // Arriving while nothing is on screen.
    let mut session = Session::new(Queue::Due, Vec::new());
    session.arrive(Entry::sides(7, ["a".to_string(), "b".to_string()], 0));
    assert_eq!(
        session.revealed(),
        1,
        "the arriving card shows its first side"
    );

    // A relearning step landing while nothing is on screen.
    let mut waiting = Session::new(
        Queue::Due,
        vec![Entry::sides(3, ["a".to_string(), "b".to_string()], 0)],
    );
    waiting.reveal();
    assert!(waiting.grade(Grade::Forgot).is_some());
    waiting.sent(1);
    assert!(waiting.acknowledge(1, 0));
    assert!(waiting.current().is_none(), "the only card is waiting");
    assert_eq!(waiting.advance_clock(LEARNING_STEP_MS), 1);
    assert_eq!(
        waiting.revealed(),
        1,
        "the card that came back shows its first side"
    );

    // Forgetting the last new card while nothing is on screen.
    let mut last = Session::new(
        Queue::New,
        vec![Entry::sides(4, ["a".to_string(), "b".to_string()], 0)],
    );
    last.reveal();
    assert!(last.grade(Grade::Forgot).is_some());
    last.sent(1);
    assert!(last.acknowledge(1, 0));
    assert_eq!(
        last.revealed(),
        1,
        "the card that came straight back shows its first side"
    );
}

#[test]
fn a_hidden_text_face_unmasks_on_one_reveal() {
    let mut session = Session::new(
        Queue::Due,
        vec![Entry::masked(
            1,
            "cloze:0",
            "The [ … ] is small.",
            "The gland is small.",
            0,
        )],
    );

    assert_eq!(
        session.lines(),
        vec![("The [ … ] is small.", false)],
        "the masked sentence is the question, and it is on screen"
    );
    assert_eq!(session.hidden(), 1);
    assert!(
        !session.gradeable(),
        "a mask cannot be graded before it lifts"
    );

    assert!(session.reveal());
    assert_eq!(session.lines(), vec![("The gland is small.", true)]);
    assert!(session.gradeable(), "one reveal is the whole card");
    assert_eq!(session.hidden(), 0, "nothing is left masked");
}

#[test]
fn a_reverse_entry_shows_the_last_side_first() {
    let mut session = Session::new(
        Queue::Due,
        vec![Entry::reverse(1, ["What is mitosis?", "Cell division"], 0)],
    );

    assert_eq!(session.direction(), "reverse");
    assert_eq!(
        session.lines(),
        vec![("Cell division", true)],
        "the answer is the question, and the question is what has to be revealed"
    );
    assert_eq!(session.hidden(), 1, "one side is still behind the request");

    assert!(session.reveal());
    assert_eq!(
        session.lines(),
        vec![("Cell division", true), ("What is mitosis?", true)],
        "the sides are revealed in reverse order"
    );
    assert!(session.gradeable(), "grading waits for the second side");
}

#[test]
fn a_forward_entry_says_which_way_it_is_asked() {
    let session = Session::new(
        Queue::Due,
        vec![Entry::sides(1, ["What is mitosis?", "Cell division"], 0)],
    );

    assert_eq!(session.direction(), "forward");
    assert_eq!(
        Session::empty().direction(),
        "",
        "nothing on screen says nothing"
    );
}

#[test]
fn a_mask_group_is_one_face() {
    let masks = vec![
        MaskRect::new(0.1, 0.1, 0.2, 0.2),
        MaskRect::new(0.6, 0.1, 0.2, 0.2),
    ];
    let mut session = Session::new(
        Queue::Due,
        vec![Entry::mask_group(
            4,
            "mask:2",
            "plate.png",
            Some("The heart".to_string()),
            masks.clone(),
            0,
        )],
    );

    assert_eq!(
        session.hidden(),
        1,
        "two rectangles of one group are one answer, so one face is hidden"
    );
    assert_eq!(session.lines(), Vec::new(), "a picture has no text lines");
    let face = session.asking().expect("the face being asked");
    let (image, caption, covered) = face.picture().expect("a picture face");
    assert_eq!(image, "plate.png");
    assert_eq!(caption, Some("The heart"));
    assert_eq!(covered, masks.as_slice());
    assert!(!session.gradeable(), "the answer is covered");

    assert!(session.reveal());
    assert_eq!(session.hidden(), 0);
    assert!(
        session.gradeable(),
        "one reveal uncovers the whole group, so the card is gradeable"
    );
    assert_eq!(
        session.grade(Grade::Remembered),
        Some((4, Grade::Remembered))
    );
}

#[test]
fn a_cram_session_starts_with_every_entry_queued() {
    let session = Session::cram(vec![
        entry(1, &["front"]),
        entry(2, &["front"]),
        entry(3, &["front"]),
    ]);

    assert_eq!(session.mode(), Mode::Cram);
    assert_eq!(
        session.order(),
        vec![1, 2, 3],
        "no due date and no allowance stands between the filter and the queue"
    );
    assert_eq!(session.remaining(), 3);
    assert_eq!(session.current().map(|entry| entry.card), Some(1));
    assert!(!session.finished(), "nothing has been answered yet");
    assert!(session.grades.is_empty());
}

#[test]
fn one_card_three_passes() {
    let mut session = Session::cram(vec![entry(1, &["front", "back"])]);

    for (pass, grade) in [Grade::Forgot, Grade::Forgot, Grade::Remembered]
        .into_iter()
        .enumerate()
    {
        while session.hidden() > 0 {
            assert!(session.reveal());
        }
        assert_eq!(
            session.grade(grade),
            Some((1, grade)),
            "pass {} asks the same card again",
            pass + 1
        );
        assert!(
            session.answer_here(),
            "a cram grade is taken here, not by the store"
        );
        assert_eq!(session.remaining(), usize::from(pass < 2));
        assert_eq!(
            session.finished(),
            pass == 2,
            "forgot comes round again; only the remembered pass ends the run"
        );
    }

    assert_eq!(
        session.grades,
        vec![Grade::Forgot, Grade::Forgot, Grade::Remembered]
    );
}

#[test]
fn a_cram_session_writes_no_schedule() {
    // A cram answer is settled inside the session: no store id is awaited and nothing leaves the run
    // except the in-memory grade and queue move.
    let mut session = Session::cram(vec![Entry::sides(1, ["front", "back"], 3)]);

    while session.hidden() > 0 {
        assert!(session.reveal());
    }
    assert_eq!(
        session.grade(Grade::Remembered),
        Some((1, Grade::Remembered))
    );
    assert!(
        session.answer_here(),
        "the grade is taken here, not sent to the store"
    );
    assert!(!session.gradeable(), "nothing waits for a store answer");
    assert_eq!(session.remaining(), 0);
    assert!(session.finished());
    assert_eq!(session.grades, vec![Grade::Remembered]);
    assert_eq!(session.mode(), Mode::Cram);
}

#[test]
fn a_cram_session_repeats_what_was_forgotten() {
    let mut session = Session::cram(vec![entry(1, &["front"]), entry(2, &["front"])]);

    // Forget card 1: it returns to the end of this run and card 2 comes next.
    while session.hidden() > 0 {
        assert!(session.reveal());
    }
    session.grade(Grade::Forgot);
    assert!(session.answer_here());
    assert_eq!(session.current().map(|e| e.card), Some(2));
    assert_eq!(
        session.order(),
        vec![2, 1],
        "forgotten card is queued again"
    );
    assert!(!session.finished());

    // Remember card 2, then the forgotten card 1 returns and must be remembered to finish.
    while session.hidden() > 0 {
        assert!(session.reveal());
    }
    session.grade(Grade::Remembered);
    assert!(session.answer_here());
    assert_eq!(session.current().map(|e| e.card), Some(1));

    while session.hidden() > 0 {
        assert!(session.reveal());
    }
    session.grade(Grade::Remembered);
    assert!(session.answer_here());
    assert!(
        session.finished(),
        "the run ends only after every card was remembered"
    );
    assert_eq!(
        session.grades,
        vec![Grade::Forgot, Grade::Remembered, Grade::Remembered]
    );
}

#[test]
fn two_prompts_of_one_card_are_each_asked() {
    let mut session = Session::new(
        Queue::New,
        vec![
            Entry::masked(1, "cloze:1", "[ … ] b", "a b", 0),
            Entry::masked(1, "cloze:2", "a [ … ]", "a b", 0),
        ],
    );
    answer(&mut session, Grade::Remembered, NOON);

    assert!(!session.finished(), "the second prompt is still to answer");
    assert_eq!(
        session.current().map(|entry| entry.prompt.as_str()),
        Some("cloze:2")
    );
    answer(&mut session, Grade::Remembered, NOON);
    assert!(session.finished());
}
