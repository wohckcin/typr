use super::*;

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
    // No deck yet: this call stores the deck, and the card follows its answer.
    assert!(!state.persist(), "the card waits for its deck");

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

/// The editor is a door, not a trap: Escape leaves it, and asks first when the buffer is dirty.
#[test]
fn escaping_the_editor_returns_to_the_library_and_asks_first_when_dirty() {
    let mut state = State {
        screen: Screen::Editor,
        editor: Editor::new(None, "front\n---\nback".to_string()),
        ..State::default()
    };
    let escape = KeyEvent {
        key: Key::Escape,
        mods: Mods::default(),
    };
    let mut tx = Tx::default();

    assert!(on_key(&mut state, &escape, &mut tx), "the escape is taken");
    assert_eq!(state.screen, Screen::Library, "a clean buffer leaves");
    assert!(!state.editor.asking, "nothing to answer");

    // A dirty buffer asks, and stays put until the question is answered.
    state.screen = Screen::Editor;
    state.editor.buffer.replace_text("front\n---\nback\nedited");
    assert!(on_key(&mut state, &escape, &mut tx), "the escape is taken");
    assert_eq!(state.screen, Screen::Editor, "an unsaved card stays open");
    assert!(state.editor.asking, "the confirmation is on screen");

    state
        .editor
        .resolve(editor::Choice::Discard)
        .expect("discard");
    state.screen = Screen::Library;
    assert!(!state.editor.asking, "the question is gone");
}

#[test]
fn a_save_the_library_refuses_leaves_the_card_dirty() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = library_with_a_card(&dir);
    store.close();

    let mut state = State {
        store: Some(store),
        ..State::default()
    };
    state.editor = crate::editor::Editor::new(Some(1), "front\n---\nback".to_string());
    state
        .editor
        .buffer
        .replace_text("front\n---\nchanged".to_string());

    assert!(!state.persist(), "a refused save is not a save");
    assert!(
        state.editor.dirty(),
        "the card still holds work the library never took"
    );
    assert!(state.failure.is_some(), "and the reader is told why");
}

#[test]
fn a_deck_this_call_did_not_ask_for_does_not_write_the_waiting_card() {
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
    state.pending_new_card = Some("waiting\n---\ntext".to_string());

    // The sidebar makes a deck, which answers with a deck id this state never asked a card for.
    library::Host::new_deck(&mut state);
    // Long enough for the deck and for any card it might have carried.
    for _ in 0..100 {
        let mut wake = None;
        frame_step(&mut state, 0, &mut wake);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    let snapshot = state.library.snapshot().clone();
    assert_eq!(snapshot.decks.len(), 1, "the sidebar deck was made");
    assert!(
        snapshot.cards.is_empty(),
        "the waiting text is not written into a deck this call did not ask for"
    );
    assert!(
        state.pending_new_card.is_some(),
        "the text is still waiting"
    );
}

#[test]
fn a_deck_send_that_fails_lets_the_waiting_card_go() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("library.sqlite3");
    let waker: Waker = std::sync::Arc::new(|| {});
    let handle = Handle::spawn(
        typr_core::store::Db::open(&path).expect("open"),
        Box::new(typr_core::clock::SystemClock),
        waker,
    )
    .expect("spawn");
    handle.close();
    let mut state = State {
        store: Some(handle),
        ..State::default()
    };
    state.screen = Screen::Editor;
    state.editor = Editor::new(None, "front\n---\nback".to_string());

    assert!(!state.persist(), "nothing was stored");
    assert!(
        state.pending_new_card.is_none(),
        "a text whose deck could not be made is not left armed"
    );
    assert!(state.failure.is_some(), "the refusal is on the line");
}

#[test]
fn a_card_that_still_needs_a_deck_is_not_reported_saved() {
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

    // A library with no deck yet stores the deck first, so the card is not saved by this call.
    assert!(
        !state.persist(),
        "the card cannot be stored before its deck"
    );
    assert!(
        state.pending_new_card.is_some(),
        "the text waits for the deck"
    );

    // Once the deck and the card land, the same buffer is saved.
    for _ in 0..200 {
        let mut wake = None;
        frame_step(&mut state, 0, &mut wake);
        if state.editor.card.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(state.editor.card.is_some(), "the card was stored");
    assert!(!state.editor.dirty(), "the stored buffer is saved");
}

#[test]
fn a_second_save_of_a_new_card_does_not_make_a_second_card() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = library_with_a_card(&dir);
    let mut state = State {
        store: Some(store),
        ..State::default()
    };

    LibraryHost::new_card(&mut state);
    wait_for(&mut state, "the draft answer", |s| {
        s.draft_request.is_none()
    });
    state
        .editor
        .buffer
        .replace_text("one\n---\ntwo".to_string());

    assert!(state.persist(), "the first save goes out");
    assert!(
        !state.persist(),
        "a second save while the first is in flight is refused"
    );

    wait_for(&mut state, "the new card to be adopted", |s| {
        s.editor.card.is_some()
    });
    wait_for(&mut state, "the library to hold both cards", |s| {
        s.library.cards().len() == 2
    });
    assert_eq!(
        state.library.cards().len(),
        2,
        "one card from the fixture and one new card, not three"
    );
}

#[test]
fn the_tags_typed_before_a_save_go_on_the_card() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = library_with_a_card(&dir);
    let mut state = State {
        store: Some(store),
        ..State::default()
    };

    LibraryHost::new_card(&mut state);
    wait_for(&mut state, "the draft answer", |s| {
        s.draft_request.is_none()
    });
    state
        .editor
        .buffer
        .replace_text("one\n---\ntwo".to_string());
    state.editor.tags.replace_text("Grammar, verbs".to_string());

    assert!(state.persist(), "the save goes out");
    wait_for(&mut state, "the tags to reach the library", |s| {
        s.library
            .cards()
            .iter()
            .any(|card| card.tags == vec!["Grammar".to_string(), "verbs".to_string()])
    });
    assert!(
        !state.editor.dirty(),
        "the card reads as saved once its tags are stored"
    );
}

#[test]
fn text_typed_and_never_saved_is_there_after_a_restart() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("library.sqlite3");
    let store = library_with_a_card(&dir);

    let mut state = State {
        store: Some(store),
        ..State::default()
    };
    LibraryHost::new_card(&mut state);
    wait_for(&mut state, "the draft answer", |s| {
        s.draft_request.is_none()
    });
    state
        .editor
        .buffer
        .replace_text("half a thought".to_string());
    crate::editor::Host::note_typing(&mut state);

    // Dropping the handle flushes the queue the way quitting would.
    drop(state.store.take());
    let db = Db::open(&path).expect("open");
    assert_eq!(
        db.draft(0).expect("draft"),
        Some("half a thought".to_string()),
        "the text of a card that was never saved is on disk"
    );
    drop(db);

    let waker: Waker = Arc::new(|| {});
    let mut reopened = State {
        store: Some(
            Handle::spawn(
                Db::open(&path).expect("open"),
                Box::new(typr_core::clock::SystemClock),
                waker,
            )
            .expect("spawn"),
        ),
        ..State::default()
    };
    LibraryHost::new_card(&mut reopened);
    wait_for(&mut reopened, "the draft to come back", |s| {
        s.editor.buffer.text == "half a thought"
    });
}

/// A diagram can be masked from the keyboard alone: Alt+N places a rectangle in the middle and the
/// Alt arrow chords size it, so the pointer is a convenience rather than the only way in.
#[test]
fn a_mask_can_be_made_without_a_pointer() {
    let mut editor = Editor::new(None, String::new());
    editor.set_kind(typr_core::store::CardKind::Diagram);
    editor.image = Some("plate.png".to_string());
    let mut state = State {
        screen: Screen::Editor,
        editor,
        ..State::default()
    };
    let mut tx = Tx::default();
    let alt = |key: Key| KeyEvent {
        key,
        mods: Mods {
            alt: true,
            ..Mods::default()
        },
    };

    assert!(
        on_key(&mut state, &alt(Key::Text("n".to_string())), &mut tx),
        "Alt+N is taken"
    );
    assert_eq!(state.editor.masks.len(), 1, "a rectangle is on the picture");
    let placed = state.editor.masks[0].rect;
    assert!((placed.w - 0.3).abs() < 1e-6, "it starts a third wide");

    assert!(
        on_key(&mut state, &alt(Key::Right), &mut tx),
        "Alt+Right widens"
    );
    assert!(on_key(&mut state, &alt(Key::Right), &mut tx), "twice");
    let wider = state.editor.masks[0].rect.w;
    assert!(wider > placed.w, "the rectangle is wider: {wider}");

    assert!(
        on_key(&mut state, &alt(Key::Up), &mut tx),
        "Alt+Up shortens"
    );
    let shorter = state.editor.masks[0].rect.h;
    assert!(shorter < placed.h, "the rectangle is shorter: {shorter}");
    on_key(&mut state, &alt(Key::Text("n".to_string())), &mut tx);
    assert_eq!(state.editor.masks.len(), 2, "a second rectangle");
    assert_eq!(
        state.editor.masks[1].group, 2,
        "and it asks in its own group"
    );
}
