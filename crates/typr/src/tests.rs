//! The binary's behaviour: what a frame, a key and a click do to the whole app.

use crate::*;
use rune::{Key, KeyEvent, Mods};
use typr_core::clock::LEARNING_STEP_MS;

/// One due card, revealed to its last side, forgotten, and acknowledged.
fn forgotten_due_card() -> State {
    let mut session = Session::new(
        Queue::Due,
        vec![Entry {
            card: 7,
            sides: vec!["front".to_string(), "back".to_string()],
            overdue: 0,
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
        let db = typr_core::store::Db::open(&path).expect("open");
        let deck = match db
            .apply(
                &typr_core::store::Command::CreateDeck {
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
            &typr_core::store::Command::CreateCard {
                deck,
                markdown: "front\n---\nback".to_string(),
            },
            0,
        )
        .expect("card");
    }
    Handle::spawn(
        typr_core::store::Db::open(&path).expect("open"),
        Box::new(typr_core::clock::SystemClock),
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
            overdue: 0,
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
        let db = typr_core::store::Db::open(&path).expect("open");
        let deck = match db
            .apply(
                &typr_core::store::Command::CreateDeck {
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
                &typr_core::store::Command::CreateCard {
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
            &typr_core::store::Command::Grade {
                card: reviewed,
                key: "fwd".to_string(),
                grade: typr_core::store::Grade::Remembered,
                duration_ms: None,
            },
            1_000,
        )
        .expect("grade");
        (reviewed, db.snapshot().expect("snapshot").cards[1].id)
    };
    let handle = Handle::spawn(
        typr_core::store::Db::open(&path).expect("open"),
        Box::new(typr_core::clock::SystemClock),
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
        typr_core::store::Db::open(&path).expect("open"),
        Box::new(typr_core::clock::SystemClock),
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
    let db = typr_core::store::Db::open(&path).expect("reopen");
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

/// Draw the library screen from whatever the store has published.
fn refresh_library(state: &mut State) {
    let snapshot = state.store.as_ref().expect("store").snapshot();
    state.library.set_snapshot(snapshot);
}

/// Wait for one command's answer, then draw what the store now holds.
fn settle(state: &mut State, id: u64) {
    for _ in 0..200 {
        let answered = state
            .store
            .as_ref()
            .expect("store")
            .poll()
            .iter()
            .any(|event| event.id == id);
        if answered {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    refresh_library(state);
}

#[test]
fn the_archived_and_trash_views_restore_and_destroy() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = library_with_one_card(&dir);
    let mut state = State {
        store: Some(store),
        seeded: true,
        ..State::default()
    };
    refresh_library(&mut state);
    assert_eq!(state.library.cards().len(), 1, "the card starts active");

    let archived = state
        .store
        .as_ref()
        .expect("store")
        .archive_card(1, true)
        .expect("sent");
    settle(&mut state, archived);
    assert!(
        state.library.cards().is_empty(),
        "an archived card leaves the active view"
    );
    state.library.set_mode(CardMode::Archived);
    assert_eq!(
        state.library.cards().len(),
        1,
        "the archived view shows what the active view hides"
    );

    // The restore button in that view goes through this host method.
    library::Host::restore_card(&mut state, 1);
    for _ in 0..200 {
        refresh_library(&mut state);
        if state.library.cards().is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(
        state.library.cards().is_empty(),
        "a restored card leaves the archive"
    );
    state.library.set_mode(CardMode::Active);
    assert_eq!(
        state.library.cards().len(),
        1,
        "the restored card is active again"
    );

    let trashed = state
        .store
        .as_ref()
        .expect("store")
        .trash_card(1)
        .expect("sent");
    settle(&mut state, trashed);
    state.library.set_mode(CardMode::Trash);
    assert_eq!(
        state.library.cards().len(),
        1,
        "the trash view shows what was deleted"
    );

    // Emptying is irreversible, so it is asked for through a dialog.
    let mut preview = State {
        library: Library::seeded_mixed(),
        ..State::default()
    };
    preview.library.set_mode(CardMode::Trash);
    preview.library.confirming_empty = true;
    let mut runtime = rune::Runtime::new(preview, view, (900.0, 600.0), rune::Env::new(true, 1.0));
    runtime.frame(0.0);
    assert!(
        runtime.find("confirm-empty").is_some(),
        "the question offers the irreversible action"
    );
    assert!(
        runtime.find("cancel-empty").is_some(),
        "and a way to refuse it"
    );

    library::Host::empty_trash(&mut state);
    for _ in 0..200 {
        refresh_library(&mut state);
        if state.library.cards().is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(
        state.library.cards().is_empty(),
        "emptying the trash destroys what it held"
    );
    assert!(
        state
            .store
            .as_ref()
            .expect("store")
            .snapshot()
            .cards
            .is_empty(),
        "and the library holds no trace of it"
    );
}

#[test]
fn archiving_a_card_and_a_deck_takes_them_out_of_the_library() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = library_with_one_card(&dir);
    let mut state = State {
        store: Some(store),
        seeded: true,
        ..State::default()
    };
    refresh_library(&mut state);
    assert_eq!(state.library.cards().len(), 1, "the card starts active");

    // Give it a schedule, so archiving has a due date to preserve.
    let graded = state
        .store
        .as_ref()
        .expect("store")
        .grade(1, "fwd", Grade::Remembered, None)
        .expect("sent");
    settle(&mut state, graded);
    let far = i64::MAX / 4;
    let due = |state: &mut State| {
        let id = state
            .store
            .as_ref()
            .expect("store")
            .due_prompts(far, 0, 1)
            .expect("sent");
        for _ in 0..200 {
            let answer = state
                .store
                .as_ref()
                .expect("store")
                .poll()
                .into_iter()
                .find(|event| event.id == id);
            if let Some(event) = answer {
                if let Ok(Outcome::Due(prompts)) = event.result {
                    return prompts.len();
                }
                panic!("expected the due queue");
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        panic!("no answer for the due queue");
    };
    assert_eq!(due(&mut state), 1, "the graded card is scheduled");

    library::Host::archive_card(&mut state, 1);
    for _ in 0..200 {
        refresh_library(&mut state);
        if state.library.cards().is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(
        state.library.cards().is_empty(),
        "an archived card leaves the active view"
    );
    assert_eq!(due(&mut state), 0, "and leaves the due queue");

    library::Host::restore_card(&mut state, 1);
    for _ in 0..200 {
        refresh_library(&mut state);
        if !state.library.cards().is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(
        state.library.cards().len(),
        1,
        "restoring brings it back to the active view"
    );
    assert_eq!(due(&mut state), 1, "and its due date came back with it");

    // A deck archive takes everything under it out of the library.
    library::Host::archive_deck(&mut state, 1);
    for _ in 0..200 {
        refresh_library(&mut state);
        if state.library.cards().is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(
        state.library.cards().is_empty(),
        "a card under an archived deck leaves the active view"
    );
    assert_eq!(due(&mut state), 0, "and leaves the due queue with it");
}

#[test]
fn deleting_a_card_keeps_its_history_until_the_trash_is_emptied() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = library_with_one_card(&dir);
    let mut state = State {
        store: Some(store),
        seeded: true,
        ..State::default()
    };
    refresh_library(&mut state);
    let graded = state
        .store
        .as_ref()
        .expect("store")
        .grade(1, "fwd", Grade::Remembered, None)
        .expect("sent");
    settle(&mut state, graded);

    /// The records the store holds for one card, as the history panel would count them.
    fn reviews(state: &mut State, card: Id) -> u32 {
        let id = state
            .store
            .as_ref()
            .expect("store")
            .history(card)
            .expect("sent");
        for _ in 0..200 {
            let answer = state
                .store
                .as_ref()
                .expect("store")
                .poll()
                .into_iter()
                .find(|event| event.id == id);
            if let Some(event) = answer {
                if let Ok(Outcome::History { reviews, .. }) = event.result {
                    return reviews;
                }
                panic!("expected the card's records");
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        panic!("no answer for the records");
    }
    assert_eq!(reviews(&mut state, 1), 1, "the grade was recorded");

    library::Host::trash_card(&mut state, 1);
    for _ in 0..200 {
        refresh_library(&mut state);
        if state.library.cards().is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(
        state.library.cards().is_empty(),
        "a deleted card leaves the active view"
    );
    state.library.set_mode(CardMode::Trash);
    assert_eq!(
        state.library.cards().len(),
        1,
        "the trash holds what was deleted"
    );

    library::Host::restore_card(&mut state, 1);
    for _ in 0..200 {
        refresh_library(&mut state);
        if state.library.cards().is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    state.library.set_mode(CardMode::Active);
    assert_eq!(
        state.library.cards().len(),
        1,
        "restoring puts it back in the library"
    );
    assert_eq!(
        reviews(&mut state, 1),
        1,
        "and its history came back with it"
    );

    // Deleting again and emptying the trash is the only way the record goes.
    library::Host::trash_card(&mut state, 1);
    for _ in 0..200 {
        refresh_library(&mut state);
        if state.library.cards().is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    library::Host::empty_trash(&mut state);
    for _ in 0..200 {
        if state
            .store
            .as_ref()
            .expect("store")
            .snapshot()
            .cards
            .is_empty()
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(
        state
            .store
            .as_ref()
            .expect("store")
            .snapshot()
            .cards
            .is_empty(),
        "the emptied trash holds nothing"
    );
    assert_eq!(
        reviews(&mut state, 1),
        0,
        "and its review records went with it"
    );
}

/// Wait for the published library to hold `count` decks, then draw it.
fn wait_for_decks(state: &mut State, count: usize) {
    for _ in 0..200 {
        refresh_library(state);
        if state.library.snapshot().decks.len() == count {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!(
        "expected {count} decks, found {}",
        state.library.snapshot().decks.len()
    );
}

#[test]
fn decks_are_created_renamed_and_moved_from_the_sidebar() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = library_with_one_card(&dir);
    let mut state = State {
        store: Some(store),
        seeded: true,
        ..State::default()
    };
    wait_for_decks(&mut state, 1);

    // The sidebar offers the three deck actions.
    let mut drawn = State {
        library: Library::seeded_mixed(),
        ..State::default()
    };
    drawn.library.selected_deck = Some(1);
    let mut runtime = rune::Runtime::new(drawn, view, (900.0, 600.0), rune::Env::new(true, 1.0));
    runtime.frame(0.0);
    for key in ["new-deck", "rename-deck", "move-deck"] {
        assert!(runtime.find(key).is_some(), "{key} is offered");
    }
    drop(runtime);

    library::Host::new_deck(&mut state);
    wait_for_decks(&mut state, 2);
    library::Host::new_deck(&mut state);
    wait_for_decks(&mut state, 3);
    let names: Vec<String> = state
        .library
        .snapshot()
        .decks
        .iter()
        .map(|deck| deck.name.clone())
        .collect();
    let existing = state.library.snapshot().decks[0].name.clone();
    assert_eq!(
        names,
        vec![existing, "Deck 1".to_string(), "Deck 2".to_string()],
        "a new deck gets a name nothing else holds"
    );
    let third = state.library.snapshot().decks[2].id;
    let second = state.library.snapshot().decks[1].id;

    // Renaming goes through the field the row draws.
    state.library.selected_deck = Some(third);
    library::Host::begin_rename(&mut state);
    assert_eq!(state.library.renaming, Some(third), "the field opens");
    state.library.rename_field().replace_text("Languages");
    assert!(library::Host::finish_rename(&mut state));
    let mut name = String::new();
    for _ in 0..200 {
        refresh_library(&mut state);
        name = state
            .library
            .snapshot()
            .decks
            .iter()
            .find(|deck| deck.id == third)
            .map(|deck| deck.name.clone())
            .expect("the renamed deck");
        if name == "Languages" {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(name, "Languages", "the new name is stored");

    // Moving makes the third deck a child of the second.
    library::Host::move_deck(&mut state, third, Some(second));
    let mut parent = None;
    for _ in 0..200 {
        refresh_library(&mut state);
        parent = state
            .library
            .snapshot()
            .decks
            .iter()
            .find(|deck| deck.id == third)
            .and_then(|deck| deck.parent);
        if parent == Some(second) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(
        parent,
        Some(second),
        "the deck now sits under its new parent"
    );

    // Two decks at the top cannot share a name, and the screen says so.
    let taken = state.library.snapshot().decks[0].name.clone();
    state.library.selected_deck = Some(second);
    library::Host::begin_rename(&mut state);
    state.library.rename_field().replace_text(taken);
    library::Host::finish_rename(&mut state);
    for _ in 0..200 {
        // Answers reach the screen through the frame step, which is what drain does.
        let mut wake = None;
        frame_step(&mut state, 0, &mut wake);
        if state.failure.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(
        state
            .failure
            .as_deref()
            .is_some_and(|why| why.contains("UNIQUE")),
        "a duplicate name is refused out loud, got {:?}",
        state.failure
    );
}

#[test]
fn archiving_a_deck_that_holds_cards_asks_first() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = library_with_one_card(&dir);
    let mut state = State {
        store: Some(store),
        seeded: true,
        ..State::default()
    };
    refresh_library(&mut state);
    let deck = state.library.cards()[0].deck;
    assert_eq!(state.library.cards().len(), 1, "the deck holds one card");

    // A deck with cards asks before it archives them.
    library::Host::request_archive_deck(&mut state, deck);
    assert_eq!(
        state.library.confirming_archive,
        Some(deck),
        "the question waits for an answer"
    );
    assert_eq!(
        state.library.cards().len(),
        1,
        "and nothing has been archived yet"
    );

    let mut preview = State {
        library: Library::seeded_mixed(),
        ..State::default()
    };
    preview.library.confirming_archive = Some(1);
    let mut runtime = rune::Runtime::new(preview, view, (900.0, 600.0), rune::Env::new(true, 1.0));
    runtime.frame(0.0);
    assert!(
        runtime.find("confirm-archive-deck").is_some(),
        "the question offers to go ahead"
    );
    assert!(
        runtime.find("cancel-archive-deck").is_some(),
        "and a way to refuse it"
    );
    drop(runtime);

    library::Host::archive_deck(&mut state, deck);
    for _ in 0..200 {
        refresh_library(&mut state);
        if state.library.cards().is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(
        state.library.cards().is_empty(),
        "archiving takes the cards with it"
    );

    // A deck that holds nothing archives without a question.
    library::Host::new_deck(&mut state);
    for _ in 0..200 {
        refresh_library(&mut state);
        if state.library.snapshot().decks.len() == 2 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let empty = state
        .library
        .snapshot()
        .decks
        .iter()
        .map(|deck| deck.id)
        .find(|id| *id != deck)
        .expect("the new deck");
    state.library.selected_deck = Some(empty);
    state.library.confirming_archive = None;
    library::Host::request_archive_deck(&mut state, empty);
    assert_eq!(
        state.library.confirming_archive, None,
        "an empty deck archives without asking"
    );
}

#[test]
fn a_card_opens_in_the_editor_with_the_tags_it_holds() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = library_with_one_card(&dir);
    let tagged = store
        .set_tags(1, &["Grammar".to_string(), "verbs".to_string()])
        .expect("sent");
    let mut state = State {
        store: Some(store),
        seeded: true,
        ..State::default()
    };
    settle(&mut state, tagged);
    refresh_library(&mut state);

    library::Host::open_card(&mut state, 1);
    assert!(matches!(state.screen, Screen::Editor), "the editor opens");
    assert_eq!(state.editor.card, Some(1), "on the card that was clicked");
    assert_eq!(
        state.editor.tag_list(),
        vec!["Grammar", "verbs"],
        "carrying the tags the card holds"
    );
    assert!(!state.editor.dirty(), "with nothing to save yet");

    // The field is drawn, and so is the way in from the list.
    let mut runtime = rune::Runtime::new(
        state,
        crate::view,
        (900.0, 600.0),
        rune::Env::new(true, 1.0),
    );
    runtime.frame(0.0);
    assert!(runtime.find("tags").is_some(), "the tag field is drawn");
    let mut state = runtime.state;

    // Saving new tags reaches the store and comes back on the card.
    state.editor.tags.replace_text("Grammar, nouns".to_string());
    let tags = state.editor.tag_list();
    editor::Host::set_tags(&mut state, 1, &tags);
    for _ in 0..200 {
        let snapshot = state.store.as_ref().expect("store").snapshot();
        if snapshot
            .cards
            .iter()
            .any(|card| card.tags == vec!["Grammar".to_string(), "nouns".to_string()])
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let snapshot = state.store.as_ref().expect("store").snapshot();
    assert_eq!(
        snapshot.cards[0].tags,
        vec!["Grammar".to_string(), "nouns".to_string()],
        "the card now carries what was typed"
    );
}

#[test]
fn the_library_offers_a_way_into_the_editor() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = library_with_one_card(&dir);
    let mut state = State {
        store: Some(store),
        seeded: true,
        ..State::default()
    };
    settle(&mut state, 0);
    refresh_library(&mut state);

    let mut runtime = rune::Runtime::new(
        state,
        crate::view,
        (900.0, 600.0),
        rune::Env::new(true, 1.0),
    );
    runtime.frame(0.0);
    assert!(runtime.find("edit-1").is_some(), "a card can be opened");
    assert!(runtime.find("new-card").is_some(), "and a new one started");
    let mut state = runtime.state;

    library::Host::new_card(&mut state);
    assert!(matches!(state.screen, Screen::Editor), "a new card opens");
    assert_eq!(state.editor.card, None, "with no id until it is saved");
}

#[test]
fn a_card_moves_to_another_deck_and_keeps_its_schedule() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = library_with_one_card(&dir);
    let mut state = State {
        store: Some(store),
        seeded: true,
        ..State::default()
    };
    refresh_library(&mut state);
    let from = state.library.cards()[0].deck;

    let graded = state
        .store
        .as_ref()
        .expect("store")
        .grade(1, "fwd", Grade::Remembered, None)
        .expect("sent");
    settle(&mut state, graded);
    let made = state
        .store
        .as_ref()
        .expect("store")
        .create_deck(None, "French")
        .expect("sent");
    settle(&mut state, made);
    let to = state
        .library
        .snapshot()
        .decks
        .iter()
        .map(|deck| deck.id)
        .find(|id| *id != from)
        .expect("the new deck");

    // The row offers the move, and the sidebar takes the card.
    let mut preview = State {
        library: Library::seeded(3),
        ..State::default()
    };
    let mut runtime = rune::Runtime::new(
        preview,
        crate::view,
        (900.0, 600.0),
        rune::Env::new(true, 1.0),
    );
    runtime.frame(0.0);
    assert!(
        runtime.find("move-card-1").is_some(),
        "a card can be moved to another deck"
    );
    preview = runtime.state;
    drop(preview);

    library::Host::move_card(&mut state, 1, to);
    for _ in 0..200 {
        refresh_library(&mut state);
        if state.library.cards()[0].deck == to {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(
        state.library.cards()[0].deck,
        to,
        "the card lives in its new deck"
    );

    state.library.select_deck(Some(from));
    assert!(
        state.library.cards().is_empty(),
        "and has left the old deck's list"
    );
    state.library.select_deck(Some(to));
    assert_eq!(
        state.library.cards().len(),
        1,
        "while the new deck lists it"
    );

    // The schedule came with it: a graded card is still due.
    let id = state
        .store
        .as_ref()
        .expect("store")
        .due_prompts(i64::MAX / 4, 0, 1)
        .expect("sent");
    let mut due = None;
    for _ in 0..200 {
        let answer = state
            .store
            .as_ref()
            .expect("store")
            .poll()
            .into_iter()
            .find(|event| event.id == id);
        if let Some(event) = answer {
            if let Ok(Outcome::Due(prompts)) = event.result {
                due = Some(prompts.len());
                break;
            }
            panic!("expected the due queue");
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(due, Some(1), "and its due date survived the move");
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
                Entry {
                    card: 1,
                    sides: vec!["a".to_string(), "b".to_string()],
                    overdue: 0,
                },
                Entry {
                    card: 2,
                    sides: vec!["c".to_string(), "d".to_string()],
                    overdue: 0,
                },
                Entry {
                    card: 3,
                    sides: vec!["e".to_string(), "f".to_string()],
                    overdue: 3,
                },
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
            vec![Entry {
                card: 1,
                sides: vec!["front".to_string(), "back".to_string()],
                overdue: 0,
            }],
        ),
        screen: Screen::Session,
        ..State::default()
    };
    assert!(state.store.is_none(), "the library is not open");

    // Answer the card: reveal both sides, then grade with the keyboard.
    for _ in 0..2 {
        let space = KeyEvent {
            key: Key::Text(" ".into()),
            mods: Mods::default(),
        };
        assert!(study::key(&mut state, &space), "the reveal key is taken");
    }
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
