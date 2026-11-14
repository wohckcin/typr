use super::*;

#[test]
fn a_grade_that_lands_after_leaving_does_not_open_a_summary() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = library_with_one_card(&dir);
    let mut state = State {
        store: Some(store),
        session: session_on(1),
        screen: Screen::Session,
        ..State::default()
    };
    let grade = state.session.grade(Grade::Remembered).expect("graded").1;
    let id = state.submit_grade(1, grade).expect("sent");
    state.session.sent(id);

    state.leave_session();
    for _ in 0..100 {
        state.drain();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    assert!(
        matches!(state.screen, Screen::Library),
        "the answer arrived after the user left, and the summary is not shown"
    );
    assert!(state.failure.is_none(), "the grade itself was accepted");
}

#[test]
fn a_late_history_answer_for_another_card_is_ignored() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (store, reviewed, fresh) = library_with_a_reviewed_and_a_fresh_card(&dir);
    let mut state = State {
        store: Some(store),
        ..State::default()
    };

    // The user asks for one card's records, then another's, before either answer lands.
    let first = state
        .store
        .as_ref()
        .expect("store")
        .history(reviewed)
        .expect("sent");
    let second = state
        .store
        .as_ref()
        .expect("store")
        .history(fresh)
        .expect("sent");
    state.editor.card = Some(fresh);
    state.history_request = Some((second, fresh));

    for _ in 0..200 {
        state.drain();
        if state.history_request.is_none() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    assert_eq!(
        state.editor.history.card,
        Some(fresh),
        "the panel shows the card the user is on, not the one it asked about first"
    );
    assert_eq!(
        state.editor.history.count, 0,
        "the fresh card has no records"
    );
    assert!(first < second, "the older answer was really answered first");
}

#[test]
fn a_refused_grade_says_why_and_leaves_the_card_up() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = library_with_one_card(&dir);
    let mut state = State {
        store: Some(store),
        session: session_on(9),
        screen: Screen::Session,
        ..State::default()
    };
    let grade = state.session.grade(Grade::Remembered).expect("graded").1;
    let id = state.submit_grade(9, grade).expect("sent");
    state.session.sent(id);

    for _ in 0..100 {
        state.drain();
        if state.failure.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    let failure = state.failure.as_deref().expect("the refusal is reported");
    assert!(failure.contains("has no prompt"), "got {failure}");
    assert_eq!(
        state.session.current().map(|entry| entry.card),
        Some(9),
        "the card stays up instead of wedging the session"
    );
}

#[test]
fn a_frame_asks_to_be_woken_for_a_relearning_step() {
    let mut state = forgotten_due_card();
    let mut wake = None;
    let changed = frame_step(&mut state, 1_000, &mut wake);
    assert!(!changed, "nothing has landed yet");
    assert_eq!(
        wake,
        Some(LEARNING_STEP_MS),
        "the frame loop is told when to look again, instead of drawing forever"
    );

    let mut wake = None;
    assert!(frame_step(&mut state, 1_000 + LEARNING_STEP_MS, &mut wake));
    assert_eq!(wake, None, "nothing left to wait for");
}

#[test]
fn a_forgotten_due_card_returns_when_its_step_lands() {
    let mut state = forgotten_due_card();
    assert!(state.session.current().is_none(), "the card left the queue");

    let (landed, wake) = advance_session(&mut state, 1_000);
    assert_eq!(landed, 0, "not yet");
    assert_eq!(wake, Some(LEARNING_STEP_MS), "the frame loop is told when");

    let (landed, wake) = advance_session(&mut state, 1_000 + LEARNING_STEP_MS);
    assert_eq!(landed, 1, "the card is back");
    assert_eq!(wake, None, "nothing left to wait for");
    assert_eq!(state.session.current().map(|entry| entry.card), Some(7));
    assert!(!state.session.finished(), "the session is not over");
}

#[test]
fn a_prompt_due_yesterday_is_one_day_overdue() {
    const HOUR: i64 = 60 * 60 * 1000;
    const NOON: i64 = 1_760_000_000_000;
    // Noon today, due at noon yesterday: one whole study day late.
    assert_eq!(crate::overdue_days(NOON, NOON - 24 * HOUR, 0), 1);
    // Due an hour ago, same study day: nothing to be late for yet.
    assert_eq!(crate::overdue_days(NOON, NOON - HOUR, 0), 0);
    // Due tomorrow: never negative.
    assert_eq!(crate::overdue_days(NOON, NOON + 24 * HOUR, 0), 0);
    // Three study days late.
    assert_eq!(crate::overdue_days(NOON, NOON - 3 * 24 * HOUR, 0), 3);
}

#[test]
fn the_session_screen_offers_a_shuffle() {
    let mut state = State {
        session: Session::new(
            Queue::Due,
            vec![
                Entry::sides(1, ["a".to_string(), "b".to_string()], 0),
                Entry::sides(2, ["c".to_string(), "d".to_string()], 0),
                Entry::sides(3, ["e".to_string(), "f".to_string()], 3),
            ],
        ),
        ..State::default()
    };
    state.screen = Screen::Session;
    let before = state.session.order();
    study::Host::reshuffle(&mut state);
    let after = state.session.order();
    let mut was = before.clone();
    let mut now = after.clone();
    was.sort_unstable();
    now.sort_unstable();
    assert_eq!(now, was, "a shuffle keeps every card in the queue");
    assert_eq!(
        after.last().copied(),
        Some(3),
        "and keeps the most overdue card behind the rest"
    );

    let mut runtime = rune::Runtime::new(
        state,
        crate::view,
        (900.0, 600.0),
        rune::Env::new(true, 1.0),
    );
    runtime.frame(0.0);
    assert!(
        runtime.find("shuffle").is_some(),
        "the screen offers the shuffle"
    );
}

#[test]
fn a_grade_that_could_not_be_sent_is_not_waited_on() {
    let mut state = State {
        session: Session::new(
            Queue::Due,
            vec![Entry::sides(
                1,
                ["front".to_string(), "back".to_string()],
                0,
            )],
        ),
        screen: Screen::Session,
        ..State::default()
    };
    assert!(state.store.is_none(), "the library is not open");

    // Answer the card: it opens on its first side, one reveal shows the second, then grade.
    let space = KeyEvent {
        key: Key::Text(" ".into()),
        mods: Mods::default(),
    };
    assert!(study::key(&mut state, &space), "the reveal key is taken");
    assert!(state.session.gradeable(), "the answers are offered");
    let grade = KeyEvent {
        key: Key::Text("j".into()),
        mods: Mods::default(),
    };
    assert!(study::key(&mut state, &grade), "the grade key is taken");

    assert!(
        state.session.gradeable(),
        "a grade that never left the app leaves the card answerable"
    );
    assert!(
        !state.session.finished(),
        "and the session does not silently end"
    );
    assert_eq!(
        state.failure.as_deref(),
        Some("the library is closed"),
        "while saying why nothing was recorded"
    );

    let mut runtime = rune::Runtime::new(
        state,
        crate::view,
        (900.0, 600.0),
        rune::Env::new(true, 1.0),
    );
    runtime.frame(0.0);
    assert!(
        runtime.find("failure").is_some(),
        "the status line is drawn for a failure"
    );
    let drawn: Vec<String> = runtime
        .elements()
        .into_iter()
        .filter_map(|element| element.text)
        .collect();
    assert!(
        drawn
            .iter()
            .any(|line| line.contains("the library is closed")),
        "and the reason is on screen, got {drawn:?}"
    );
}

#[test]
fn a_prompt_that_falls_due_during_a_session_joins_it() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = library_with_one_card_due_and_one_almost(&dir);
    let mut state = State {
        store: Some(store),
        ..State::default()
    };
    refresh_library(&mut state);
    state.start_session(Queue::Due);
    for _ in 0..50 {
        let mut wake = None;
        frame_step(&mut state, 0, &mut wake);
        if state.screen == Screen::Session {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert_eq!(state.screen, Screen::Session, "the session opened");
    assert!(state.session.holds(1), "the first due card is queued");
    assert!(state.session.holds(2), "the second due card is queued");
    assert!(
        !state.session.holds(3),
        "the card that is not due yet stays out"
    );

    // Wait for the second card's due moment to pass, then answer the first.
    std::thread::sleep(std::time::Duration::from_millis(6_300));
    let mut answered = false;
    let mut settled = 0;
    while settled < 100 {
        let mut wake = None;
        frame_step(&mut state, 0, &mut wake);
        if !answered && state.session.hidden() > 0 {
            // The card opens on its first side; the rest are shown before it can be graded.
            state.session.reveal();
        }
        if !answered && state.session.gradeable() {
            // Grade through the keyboard path, the only one that tells the session it is waiting.
            let remembered = KeyEvent {
                key: Key::Text("j".into()),
                mods: Mods::default(),
            };
            answered = study::key(&mut state, &remembered);
        }
        settled += 1;
        if state.session.holds(3) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    assert!(answered, "the first card was answered");
    assert!(
        state.session.holds(3),
        "the card that fell due mid-session joined the running queue"
    );
    assert!(!state.session.finished(), "and the session is still going");
}

#[test]
fn the_summary_counts_what_is_due_now_not_what_was_due_at_the_start() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = library_with_one_card_due_and_one_almost(&dir);
    let mut state = State {
        store: Some(store),
        ..State::default()
    };

    refresh_library(&mut state);
    let deck = state.library.snapshot().decks[0].id;
    state.library.select_deck(Some(deck));
    state.start_session(Queue::Due);
    wait_for(&mut state, "the session to open", |s| {
        matches!(s.screen, Screen::Session)
    });
    assert_eq!(state.due_now, 2, "two cards are due when the session opens");

    // Answer the card on screen through the key path, so the answer is acknowledged the way the
    // app acknowledges it. The arrival query that follows is what recounts the queue.
    state.session.reveal();
    assert!(state.session.current().is_some(), "a card is on screen");
    assert!(crate::study::key(
        &mut state,
        &KeyEvent {
            key: Key::Text("f".to_string()),
            mods: Mods::default(),
        }
    ));
    assert!(
        !state.session.gradeable(),
        "the grade left a card awaiting its answer"
    );
    wait_for(&mut state, "the count to be taken again", |s| {
        s.due_now != 2
    });

    assert_eq!(
        state.due_now, 1,
        "the answered card left the queue, so the summary counts one"
    );
}

#[test]
fn an_arrivals_answer_that_lands_after_leaving_is_not_taken() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = library_with_a_card(&dir);
    let mut state = State {
        store: Some(store),
        session: session_on(1),
        screen: Screen::Session,
        arrivals: Some(9),
        ..State::default()
    };
    assert!(
        state.takes_arrivals(9),
        "an answer for the session on screen is taken"
    );
    assert!(
        !state.takes_arrivals(8),
        "an answer to some other question is not"
    );

    crate::study::Host::leave_session(&mut state);
    assert!(
        state.arrivals.is_none(),
        "leaving forgets the question it asked"
    );
    assert!(
        !state.takes_arrivals(9),
        "and an answer that arrives afterwards is ignored, so the app still offers a session"
    );
    // Even a question the app is still waiting for must not land off the session screen.
    state.arrivals = Some(9);
    assert!(
        !state.takes_arrivals(9),
        "an answer is only taken while a session is on screen"
    );
    assert!(state.session.finished(), "with nothing left to answer");
}

#[test]
fn a_hidden_text_prompt_shows_a_masked_sentence() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = library_with_markdown(&dir, "The {{gland}} is small.");
    let mut state = State {
        store: Some(store),
        seeded: true,
        ..State::default()
    };
    settle(&mut state, 0);
    refresh_library(&mut state);

    // `rune::Runtime::new` installs no `poll`, so the app's own frame step has to be driven here.
    state.start_session(Queue::New);
    let mut wake = None;
    for _ in 0..200 {
        crate::frame_step(
            &mut state,
            typr_core::clock::SystemClock.now_ms(),
            &mut wake,
        );
        if matches!(state.screen, Screen::Session) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(
        matches!(state.screen, Screen::Session),
        "the session opened on the hidden-text card"
    );

    let mut runtime = rune::Runtime::new(
        state,
        crate::view,
        (900.0, 640.0),
        rune::Env::new(true, 1.0),
    );
    runtime.frame(0.1);
    let drawn = |runtime: &rune::Runtime<State>| -> String {
        runtime
            .elements()
            .into_iter()
            .filter_map(|element| element.text)
            .collect::<Vec<String>>()
            .join("\n")
    };
    let masked = drawn(&runtime);
    assert!(masked.contains("[ … ]"), "the answer is masked: {masked}");
    assert!(
        !masked.contains("gland"),
        "the answer is nowhere on the screen: {masked}"
    );

    let mut state = std::mem::take(&mut runtime.state);
    drop(runtime);
    study::key(
        &mut state,
        &rune::KeyEvent {
            key: rune::Key::Text(" ".to_string()),
            mods: rune::Mods::default(),
        },
    );
    let mut runtime = rune::Runtime::new(
        state,
        crate::view,
        (900.0, 640.0),
        rune::Env::new(true, 1.0),
    );
    runtime.frame(0.1);

    let unmasked = drawn(&runtime);
    assert!(
        unmasked.contains("gland"),
        "one press shows the answer: {unmasked}"
    );
    assert!(
        runtime.find("remembered").is_some(),
        "and the grade is offered"
    );
    assert!(runtime.find("forgot").is_some());
}

#[test]
fn a_prompt_left_over_from_a_two_sided_card_does_not_open_a_session() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = library_with_markdown(&dir, "front\n---\nback");
    let mut state = State {
        store: Some(store),
        seeded: true,
        ..State::default()
    };
    settle(&mut state, 0);
    let card = state.library.cards()[0].id;

    // Saving it down to one side keeps the `fwd` prompt, because a prompt is never deleted; the
    // card is a note now, so the queue answer must not put it in front of anyone.
    let saved = state
        .store
        .as_ref()
        .expect("store")
        .save_card(card, "```\n{{x}}\n```\nonly a note")
        .expect("saved");
    settle(&mut state, saved);
    refresh_library(&mut state);

    let entries = state.session_entries(vec![(card, "fwd".to_string(), None)]);
    assert!(
        entries.is_empty(),
        "a one-sided card is a note even with a prompt left over: {} entries",
        entries.len()
    );
}

#[test]
fn a_prompt_the_card_no_longer_claims_does_not_open_a_session() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = library_with_markdown(&dir, "{{1::alpha}} {{2::beta}}");
    let mut state = State {
        store: Some(store),
        seeded: true,
        ..State::default()
    };
    settle(&mut state, 0);
    let card = state.library.cards()[0].id;
    let saved = state
        .store
        .as_ref()
        .expect("store")
        .save_card(card, "{{1::alpha}} only")
        .expect("saved");
    settle(&mut state, saved);
    refresh_library(&mut state);

    assert!(
        state
            .session_entries(vec![(card, "cloze:2".to_string(), None)])
            .is_empty(),
        "a group whose spans are gone cannot be asked"
    );
    assert_eq!(
        state
            .session_entries(vec![(card, "cloze:1".to_string(), None)])
            .len(),
        1,
        "the group the card still holds is asked as usual"
    );
}

/// A state showing a revealed session on one card, at the given window width.
fn session_state(width: f32) -> State {
    let mut state = State {
        screen: Screen::Session,
        session: session_on(1),
        ..State::default()
    };
    state.library.grid_width = width;
    state
}

fn drawn_text(state: State) -> Vec<String> {
    let mut runtime = rune::Runtime::new(state, view, (900.0, 600.0), rune::Env::new(true, 1.0));
    runtime.frame(0.1);
    runtime
        .elements()
        .into_iter()
        .filter_map(|element| element.text)
        .collect()
}

#[test]
fn the_reveal_action_shows_its_key() {
    // The keycap is drawn by the button itself and is decorative for assistive technology, so
    // what a test can hold is the decision: a wide window offers the chord, and the label stays
    // the label.
    let state = session_state(900.0);
    assert_eq!(
        crate::keymap::keycap(&state, Command::GradeRemembered).as_deref(),
        Some("J")
    );
    let drawn = drawn_text(session_state(900.0));
    assert!(
        drawn.iter().any(|text| text == "Remembered"),
        "the grade button lost its label: {drawn:?}"
    );
    assert!(
        !drawn.iter().any(|text| text.contains("Remembered  J")),
        "the chord went into the label as well as onto the keycap: {drawn:?}"
    );
}

#[test]
fn a_narrow_window_keeps_the_labels_without_the_keycaps() {
    let drawn = drawn_text(session_state(400.0));
    assert!(
        drawn.iter().any(|text| text == "Remembered"),
        "the label went too"
    );
    assert!(
        !drawn.iter().any(|text| text.contains("  J")),
        "a narrow window still drew a keycap: {drawn:?}"
    );
}

#[test]
fn a_reverse_review_says_so() {
    let mut state = State {
        session: Session::new(
            Queue::Due,
            vec![Entry::reverse(
                1,
                ["What is mitosis?".to_string(), "Cell division".to_string()],
                0,
            )],
        ),
        ..State::default()
    };
    state.screen = Screen::Session;
    let mut runtime = rune::Runtime::new(
        state,
        crate::view,
        (900.0, 600.0),
        rune::Env::new(true, 1.0),
    );
    runtime.frame(0.1);
    let drawn: Vec<String> = runtime
        .elements()
        .into_iter()
        .filter_map(|element| element.text)
        .collect();
    assert!(
        drawn.iter().any(|line| line.starts_with("Reverse")),
        "a card asked from its answer says which way it is being asked: {drawn:?}"
    );

    let forward = State {
        session: Session::new(
            Queue::Due,
            vec![Entry::sides(
                1,
                ["What is mitosis?".to_string(), "Cell division".to_string()],
                0,
            )],
        ),
        ..State::default()
    };
    let mut runtime = rune::Runtime::new(
        forward,
        crate::view,
        (900.0, 600.0),
        rune::Env::new(true, 1.0),
    );
    runtime.frame(0.1);
    let drawn: Vec<String> = runtime
        .elements()
        .into_iter()
        .filter_map(|element| element.text)
        .collect();
    assert!(
        drawn.iter().all(|line| !line.contains("Reverse")),
        "and a forward one does not: {drawn:?}"
    );
}
