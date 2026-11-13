//! The binary's behaviour: what a frame, a key and a click do to the whole app.

use crate::*;
use typr::clock::LEARNING_STEP_MS;

/// One due card, revealed to its last side, forgotten, and acknowledged.
fn forgotten_due_card() -> State {
    let mut session = Session::new(
        Queue::Due,
        vec![Entry {
            card: 7,
            sides: vec!["front".to_string(), "back".to_string()],
        }],
    );
    while session.next_side().is_some() {
        session.reveal();
    }
    assert!(session.grade(Grade::Forgot).is_some(), "graded");
    session.sent(42);
    assert!(session.acknowledge(42, 1_000), "acknowledged");
    State {
        session,
        ..State::default()
    }
}

/// A library on disk holding one deck and one two-sided card, and a handle onto it.
fn library_with_one_card(dir: &tempfile::TempDir) -> Handle {
    let path = dir.path().join("library.sqlite3");
    {
        let db = typr::store::Db::open(&path).expect("open");
        let deck = match db
            .apply(
                &typr::store::Command::CreateDeck {
                    parent: None,
                    name: "Biology".to_string(),
                },
                0,
            )
            .expect("deck")
        {
            Outcome::Deck(id) => id,
            other => panic!("expected a deck, got {other:?}"),
        };
        db.apply(
            &typr::store::Command::CreateCard {
                deck,
                markdown: "front\n---\nback".to_string(),
            },
            0,
        )
        .expect("card");
    }
    Handle::spawn(
        typr::store::Db::open(&path).expect("open"),
        Box::new(typr::clock::SystemClock),
        std::sync::Arc::new(|| {}),
    )
    .expect("spawn")
}

/// A session showing the library's only card, revealed to its last side.
fn session_on(card: Id) -> Session {
    let mut session = Session::new(
        Queue::Due,
        vec![Entry {
            card,
            sides: vec!["front".to_string(), "back".to_string()],
        }],
    );
    while session.next_side().is_some() {
        session.reveal();
    }
    session
}

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

/// A library with one card that has a record and one that has none.
fn library_with_a_reviewed_and_a_fresh_card(dir: &tempfile::TempDir) -> (Handle, Id, Id) {
    let path = dir.path().join("library.sqlite3");
    let (reviewed, fresh) = {
        let db = typr::store::Db::open(&path).expect("open");
        let deck = match db
            .apply(
                &typr::store::Command::CreateDeck {
                    parent: None,
                    name: "Biology".to_string(),
                },
                0,
            )
            .expect("deck")
        {
            Outcome::Deck(id) => id,
            other => panic!("expected a deck, got {other:?}"),
        };
        let card = |markdown: &str| match db
            .apply(
                &typr::store::Command::CreateCard {
                    deck,
                    markdown: markdown.to_string(),
                },
                0,
            )
            .expect("card")
        {
            Outcome::Card(id) => id,
            other => panic!("expected a card, got {other:?}"),
        };
        let reviewed = card("front\n---\nback");
        card("other front\n---\nother back");
        db.apply(
            &typr::store::Command::Grade {
                card: reviewed,
                key: "fwd".to_string(),
                grade: typr::store::Grade::Remembered,
                duration_ms: None,
            },
            1_000,
        )
        .expect("grade");
        (reviewed, db.snapshot().expect("snapshot").cards[1].id)
    };
    let handle = Handle::spawn(
        typr::store::Db::open(&path).expect("open"),
        Box::new(typr::clock::SystemClock),
        std::sync::Arc::new(|| {}),
    )
    .expect("spawn");
    (handle, reviewed, fresh)
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
fn a_failure_is_drawn_at_the_bottom_of_the_screen() {
    let state = State {
        failure: Some("The library could not be opened: no such file".to_string()),
        ..State::default()
    };
    let mut runtime = rune::Runtime::new(state, view, (900.0, 600.0), rune::Env::new(true, 1.0));
    runtime.frame(0.0);
    assert!(runtime.find("failure").is_some(), "the line is on screen");
    let drawn = runtime
        .elements()
        .into_iter()
        .any(|element| element.key.as_deref() == Some("failure"));
    assert!(drawn, "the failure row is in the tree, not just a label");
}

#[test]
fn a_save_with_no_library_refuses_and_explains() {
    let mut editor = Editor::new(None, "front\n---\nback".to_string());
    editor.buffer.insert(" edited");
    let mut state = State {
        editor,
        ..State::default()
    };
    assert!(!state.persist(), "nothing was stored");
    let failure = state.failure.as_deref().expect("the refusal is reported");
    assert!(failure.contains("not open"), "got {failure}");
    assert!(state.editor.dirty(), "the buffer is still unsaved");
}

#[test]
fn saving_a_new_card_stores_it_and_adopts_its_id() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("library.sqlite3");
    let waker: Waker = std::sync::Arc::new(|| {});
    let handle = Handle::spawn(
        typr::store::Db::open(&path).expect("open"),
        Box::new(typr::clock::SystemClock),
        waker,
    )
    .expect("spawn");

    let mut state = State {
        store: Some(handle),
        ..State::default()
    };
    state.editor = Editor::new(None, "front\n---\nback".to_string());
    assert!(state.persist(), "the card is stored");

    // The store answers on its own thread; drain until the card's id arrives.
    for _ in 0..200 {
        state.drain();
        if state.editor.card.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let card = state.editor.card.expect("the new card adopted its id");
    assert!(!state.editor.dirty(), "the buffer counts as saved");

    drop(state.store.take());
    let db = typr::store::Db::open(&path).expect("reopen");
    let cards = db.snapshot().expect("snapshot").cards;
    assert_eq!(cards.len(), 1, "one card on disk");
    assert_eq!(cards[0].id, card);
    assert_eq!(cards[0].markdown, "front\n---\nback");
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
