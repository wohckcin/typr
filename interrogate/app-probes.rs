// App-level probes from the code review at `c77c60b`. These are functions, not a test file: they
// belong at the end of `crates/typr/src/tests.rs`, whose fixtures (`library_with_one_card`,
// `session_on`, `settle`, `refresh_library`) they call.
//
// Each probe asserts what a spec requires and fails where the app does not do it.
//
// ```sh
// git worktree add -q --detach /tmp/typr-repro HEAD
// cat interrogate/app-probes.rs >> /tmp/typr-repro/crates/typr/src/tests.rs   # once only
// cd /tmp/typr-repro
// CARGO_TARGET_DIR=/tmp/typr-repro-target cargo test -p typr --bin typr -- probe_ -- --nocapture
// ```
//
// Nothing imports this file, so it never compiles here and the suite stays green.

#[test]
fn probe_unsaved_text_is_never_written_as_a_draft() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("library.sqlite3");
    let handle = library_with_one_card(&dir);
    let mut state = State {
        store: Some(handle),
        ..State::default()
    };
    refresh_library(&mut state);
    library::Host::new_card(&mut state);
    state.editor.buffer.replace_text("half typed question");
    assert!(state.editor.dirty(), "the editor knows it is unsaved");

    // Frames run, as they would while the user types; a draft flush would happen here.
    for _ in 0..5 {
        let mut wake = None;
        frame_step(&mut state, 0, &mut wake);
        std::thread::sleep(Duration::from_millis(10));
    }
    drop(state.store.take());

    let db = typr_core::store::Db::open(&path).expect("reopen");
    let draft = db.draft(0).expect("draft query");
    println!("draft for a new card after typing and frames: {draft:?}");
    assert!(
        draft.is_some(),
        "card-editor: 'Unsaved text survives a restart' — nothing was written"
    );
}

#[test]
fn probe_tags_typed_before_save_are_stored() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("library.sqlite3");
    let handle = library_with_one_card(&dir);
    let mut state = State {
        store: Some(handle),
        ..State::default()
    };
    refresh_library(&mut state);
    library::Host::new_card(&mut state);
    state.editor.buffer.replace_text("front\n---\nback");
    state.editor.tags.replace_text("biology, exam");
    assert!(state.editor.tag_list() == vec!["biology", "exam"], "tags parsed");
    assert!(state.persist(), "the card is sent");
    for _ in 0..200 {
        state.drain();
        if state.editor.card.is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let card = state.editor.card.expect("adopted");
    drop(state.store.take());

    let db = typr_core::store::Db::open(&path).expect("reopen");
    let snapshot = db.snapshot().expect("snapshot");
    let stored = snapshot
        .cards
        .iter()
        .find(|summary| summary.id == card)
        .expect("the card");
    println!("tags stored with card {card}: {:?}", stored.tags);
    assert!(
        !stored.tags.is_empty(),
        "the tags typed before Save were dropped"
    );
}

#[test]
fn probe_saving_twice_before_the_answer_creates_two_cards() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("library.sqlite3");
    let handle = library_with_one_card(&dir);
    let mut state = State {
        store: Some(handle),
        ..State::default()
    };
    refresh_library(&mut state);
    library::Host::new_card(&mut state);
    state.editor.buffer.replace_text("front\n---\nback");
    assert!(state.persist(), "first save");
    assert!(state.persist(), "second save before the first answer");
    for _ in 0..200 {
        state.drain();
        if state.editor.card.is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    drop(state.store.take());

    let db = typr_core::store::Db::open(&path).expect("reopen");
    let cards = db.snapshot().expect("snapshot").cards;
    println!("cards after two saves of one new card: {}", cards.len());
    assert_eq!(cards.len(), 1, "one save of one card made {} cards", cards.len());
}

#[test]
fn probe_a_card_under_an_archived_deck_is_listed_somewhere() {
    let dir = tempfile::tempdir().expect("temp dir");
    let handle = library_with_one_card(&dir);
    let mut state = State {
        store: Some(handle),
        ..State::default()
    };
    refresh_library(&mut state);
    let deck = state.library.snapshot().decks[0].id;
    library::Host::archive_deck(&mut state, deck);
    for _ in 0..200 {
        state.drain();
        if state.library.snapshot().decks[0].archived {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    refresh_library(&mut state);
    let active = state.library.cards().len();
    state.library.set_mode(CardMode::Archived);
    let archived = state.library.cards().len();
    state.library.set_mode(CardMode::Trash);
    let trash = state.library.cards().len();
    println!("after archiving the only deck: active={active} archived={archived} trash={trash}");
    assert!(
        archived == 1 || active == 1,
        "card-library: an archived card 'remains visible in an archived view' — listed nowhere"
    );
}

#[test]
fn probe_the_leave_dialog_closes_a_card_it_could_not_save() {
    // The store is present but already closed, so the send is refused rather than absent: that is
    // the shape the app has after a worker panic.
    let dir = tempfile::tempdir().expect("tempdir");
    let mut store = library_with_one_card(&dir);
    store.close();
    let mut editor = Editor::new(None, "half typed question".to_string());
    editor.buffer.insert(" edited");
    let mut state = State {
        screen: Screen::Editor,
        editor,
        store: Some(store),
        ..State::default()
    };
    assert!(state.editor.dirty(), "unsaved");
    let saved = editor::Host::persist(&mut state);
    println!(
        "persist -> {saved}, dirty {}, failure {:?}",
        state.editor.dirty(),
        state.failure
    );
    assert!(
        !saved && state.editor.dirty(),
        "a refused save reported success and cleared the unsaved marker"
    );
}

#[test]
fn probe_the_summary_counts_due_prompts_from_the_start_of_the_session() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("library.sqlite3");
    {
        let db = typr_core::store::Db::open(&path).expect("open");
        let Ok(Outcome::Deck(deck)) = db.apply(
            &typr_core::store::Command::CreateDeck {
                parent: None,
                name: "Biology".to_string(),
            },
            0,
        ) else {
            panic!("deck")
        };
        db.apply(
            &typr_core::store::Command::CreateCard {
                deck,
                markdown: "front\n---\nback".to_string(),
            },
            0,
        )
        .expect("card");
        // Make the one prompt due long ago, so the due session holds it.
        db.apply(
            &typr_core::store::Command::Grade {
                card: 1,
                key: "fwd".to_string(),
                grade: Grade::Remembered,
                duration_ms: None,
            },
            0,
        )
        .expect("graduate");
        let raw = rusqlite::Connection::open(&path).expect("raw");
        raw.execute("UPDATE prompt SET due_at = 1", []).expect("due");
    }
    let handle = Handle::spawn(
        typr_core::store::Db::open(&path).expect("open"),
        Box::new(typr_core::clock::SystemClock),
        std::sync::Arc::new(|| {}),
    )
    .expect("spawn");
    let mut state = State {
        store: Some(handle),
        ..State::default()
    };
    refresh_library(&mut state);
    state.start_session(Queue::Due);
    for _ in 0..200 {
        state.drain();
        if state.session.current().is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(state.session.current().is_some(), "the due card is up");
    while state.session.next_side().is_some() {
        state.session.reveal();
    }
    let _ = study::Host::submit_grade(&mut state, 1, Grade::Remembered);
    for _ in 0..200 {
        let mut wake = None;
        frame_step(&mut state, i64::MAX / 4, &mut wake);
        if matches!(state.screen, Screen::Summary) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let summary = state.due_now;
    drop(state.store.take());
    let db = typr_core::store::Db::open(&path).expect("reopen");
    let actually_due = db.due_prompts(typr_core::clock::SystemClock.now_ms(), 0, 1).expect("due").len();
    println!("summary says {summary} due; the store says {actually_due} due");
    assert_eq!(summary, actually_due, "the summary is stale");
}

#[test]
fn probe_a_new_card_is_lost_when_every_root_deck_is_archived() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("library.sqlite3");
    {
        let db = typr_core::store::Db::open(&path).expect("open");
        let Ok(Outcome::Deck(deck)) = db.apply(
            &typr_core::store::Command::CreateDeck {
                parent: None,
                name: "Library".to_string(),
            },
            0,
        ) else {
            panic!("deck")
        };
        let due = db.apply(
            &typr_core::store::Command::ArchiveDeck {
                id: deck,
                archived: true,
            },
            0,
        );
        println!("archiving the only root deck: {due:?}");
    }
    let handle = Handle::spawn(
        typr_core::store::Db::open(&path).expect("open"),
        Box::new(typr_core::clock::SystemClock),
        std::sync::Arc::new(|| {}),
    )
    .expect("spawn");
    let mut state = State {
        store: Some(handle),
        ..State::default()
    };
    refresh_library(&mut state);
    assert!(state.library.selected_deck.is_none(), "nothing selected");
    library::Host::new_card(&mut state);
    state.editor.buffer.replace_text("front\n---\nback");
    let sent = state.persist();
    for _ in 0..200 {
        let mut wake = None;
        frame_step(&mut state, 0, &mut wake);
        std::thread::sleep(Duration::from_millis(5));
    }
    drop(state.store.take());
    let db = typr_core::store::Db::open(&path).expect("reopen");
    let cards = db.snapshot().expect("snapshot").cards.len();
    println!(
        "persist={sent} cards={cards} pending={:?} failure={:?} dirty={}",
        state.pending_new_card,
        state.failure,
        state.editor.dirty()
    );
    assert!(cards == 1, "the new card never reached a deck");
}

#[test]
fn probe_the_app_shows_no_side_when_a_session_opens() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("library.sqlite3");
    {
        let db = typr_core::store::Db::open(&path).expect("open");
        let Ok(Outcome::Deck(deck)) = db.apply(
            &typr_core::store::Command::CreateDeck {
                parent: None,
                name: "Biology".to_string(),
            },
            0,
        ) else {
            panic!("deck")
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
    let handle = Handle::spawn(
        typr_core::store::Db::open(&path).expect("open"),
        Box::new(typr_core::clock::SystemClock),
        std::sync::Arc::new(|| {}),
    )
    .expect("spawn");
    let mut state = State {
        store: Some(handle),
        ..State::default()
    };
    refresh_library(&mut state);
    state.start_session(Queue::New);
    for _ in 0..200 {
        state.drain();
        if state.session.current().is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(matches!(state.screen, Screen::Session), "the session opened");
    println!(
        "session opened on card {:?} with revealed={} of {} sides",
        state.session.current().map(|entry| entry.card),
        state.session.revealed(),
        state.session.current().map_or(0, |entry| entry.sides.len())
    );
    assert!(
        state.session.revealed() >= 1,
        "study-session: 'SHALL show the first side of a prompt's card' — the app opens on Side 0"
    );
}
