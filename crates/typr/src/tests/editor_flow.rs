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

/// A diagram is authored before it exists: the picture and the rectangles are drawn on a card that
/// has no id yet, so they have to wait for it rather than be dropped.
#[test]
fn rectangles_drawn_before_the_card_exists_are_stored_with_it() {
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
    state.editor = Editor::new(None, String::new());
    state.editor.attachments =
        typr::attachments::Attachments::detached(dir.path().join("attachments"));
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/plate.png");
    state.editor.choose_image(&fixture).expect("a picture");
    state.editor.add_rect(MaskRect::new(0.1, 0.1, 0.2, 0.2));
    // No deck yet, so the deck is stored first and the card follows its answer.
    state.persist();

    for _ in 0..200 {
        state.drain();
        if state.editor.card.is_some() && state.pending_masks.is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let card = state.editor.card.expect("the card adopted its id");
    assert!(
        state.pending_masks.is_empty(),
        "the rectangles were handed over once the card had an id"
    );

    drop(state.store.take());
    let db = typr_core::store::Db::open(&path).expect("reopen");
    let snapshot = db.snapshot().expect("snapshot");
    let stored = snapshot
        .cards
        .iter()
        .find(|found| found.id == card)
        .expect("the card");
    assert_eq!(stored.kind, CardKind::Diagram, "the card is a diagram");
    let masks = db.masks(card).expect("the rectangles");
    assert_eq!(
        masks.len(),
        1,
        "the rectangle drawn before the save is there"
    );
    assert_eq!(masks[0].group, 1);
}

#[test]
fn a_field_typed_into_a_templated_card_is_stored_and_rendered() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("library.sqlite3");
    let db = typr_core::store::Db::open(&path).expect("open");
    let deck = match db
        .apply(
            &typr_core::store::Command::CreateDeck {
                parent: None,
                name: "Spanish".to_string(),
            },
            0,
        )
        .expect("deck")
    {
        typr_core::store::Outcome::Deck(id) => id,
        other => panic!("creating a deck: {other:?}"),
    };
    db.apply(
        &typr_core::store::Command::SetDeckTemplate {
            deck,
            template: Some("<<Word>>\n---\n<<Meaning>>".to_string()),
        },
        0,
    )
    .expect("template");
    let card = match db
        .apply(
            &typr_core::store::Command::CreateCard {
                deck,
                markdown: String::new(),
            },
            0,
        )
        .expect("card")
    {
        typr_core::store::Outcome::Card(id) => id,
        other => panic!("creating a card: {other:?}"),
    };
    db.apply(
        &typr_core::store::Command::SetFields {
            card,
            fields: vec![("Word".to_string(), "gato".to_string())],
        },
        0,
    )
    .expect("fields");

    let waker: Waker = std::sync::Arc::new(|| {});
    let handle = Handle::spawn(db, Box::new(typr_core::clock::SystemClock), waker).expect("spawn");
    let mut state = State {
        store: Some(handle),
        ..State::default()
    };

    // Nothing has been written yet, so the library has to be handed the store's first snapshot.
    let snapshot = state.store.as_ref().expect("store").snapshot();
    state.library.set_snapshot(snapshot);
    state.open_card(card);
    assert!(state.editor.templated(), "the deck's template is in force");
    let meaning = state
        .editor
        .fields
        .iter()
        .position(|field| field.name == "Meaning")
        .expect("the template's second field");
    state.editor.fields[meaning]
        .value
        .replace_text("cat".to_string());
    state.editor.refresh();
    assert_eq!(state.editor.text(), "gato\n---\ncat");

    assert!(state.persist(), "the fields were sent");
    for _ in 0..200 {
        state.drain();
        if !state.editor.dirty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(!state.editor.dirty(), "the fields are stored");

    drop(state.store.take());
    let reopened = typr_core::store::Db::open(&path).expect("reopen");
    let snapshot = reopened.snapshot().expect("snapshot");
    let stored = snapshot
        .cards
        .iter()
        .find(|found| found.id == card)
        .expect("the card");
    assert_eq!(
        stored.markdown, "gato\n---\ncat",
        "the text follows the fields"
    );
    assert_eq!(stored.sides, 2);
    assert_eq!(
        reopened.due_prompts(1_000, 0, 0, None).expect("due").len(),
        0
    );
}

#[test]
fn a_new_card_in_a_templated_deck_is_typed_through_fields() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("library.sqlite3");
    let db = typr_core::store::Db::open(&path).expect("open");
    let deck = match db
        .apply(
            &typr_core::store::Command::CreateDeck {
                parent: None,
                name: "Spanish".to_string(),
            },
            0,
        )
        .expect("deck")
    {
        typr_core::store::Outcome::Deck(id) => id,
        other => panic!("creating a deck: {other:?}"),
    };
    db.apply(
        &typr_core::store::Command::SetDeckTemplate {
            deck,
            template: Some("<<Word>>\n---\n<<Meaning>>".to_string()),
        },
        0,
    )
    .expect("template");

    let waker: Waker = std::sync::Arc::new(|| {});
    let handle = Handle::spawn(db, Box::new(typr_core::clock::SystemClock), waker).expect("spawn");
    let mut state = State {
        store: Some(handle),
        ..State::default()
    };
    let snapshot = state.store.as_ref().expect("store").snapshot();
    state.library.set_snapshot(snapshot);
    state.library.selected_deck = Some(deck);

    // The deck already carries a template, so the new card is templated from the first keystroke.
    state.new_card();
    assert!(
        state.editor.templated(),
        "a new card under a templated deck types through fields"
    );
    assert_eq!(
        state
            .editor
            .fields
            .iter()
            .map(|field| field.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Word", "Meaning"],
        "the table lists the fields the template asks for"
    );

    state.editor.fields[0]
        .value
        .replace_text("gato".to_string());
    state.editor.fields[1].value.replace_text("cat".to_string());
    state.editor.refresh();
    // The card does not exist yet, so the answer is "not stored yet", as for a typed card.
    assert!(!state.persist(), "the card is still being created");
    // Two writes follow the create — the fields and then the tags — so the wait is on the text
    // the store ends up holding, not on the editor's flag.
    let mut rendered = String::new();
    for _ in 0..200 {
        state.drain();
        let snapshot = state.library.snapshot();
        rendered = snapshot
            .cards
            .iter()
            .find(|card| card.deck == deck)
            .map(|card| card.markdown.clone())
            .unwrap_or_default();
        if rendered == "gato\n---\ncat" {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(rendered, "gato\n---\ncat", "the store rendered the fields");
    assert!(!state.editor.dirty(), "the card is saved");

    let snapshot = state.library.snapshot();
    let stored = snapshot
        .cards
        .iter()
        .find(|card| card.deck == deck)
        .cloned()
        .expect("the card was created");
    assert_eq!(stored.markdown, "gato\n---\ncat", "the store rendered it");
    assert_eq!(
        state.editor.card,
        Some(stored.id),
        "the editor adopted the id the store handed back"
    );
    assert!(!state.editor.dirty(), "the card is saved");
}

/// A library with a card, two cards that refer to it, and one that names nothing.
fn referencing_library(dir: &tempfile::TempDir) -> (Handle, Id, Id, Id, Id) {
    use typr_core::store::{Command, Db, Outcome};

    let path = dir.path().join("library.sqlite3");
    let (target, first, second, dangling);
    {
        let db = Db::open(&path).expect("open");
        let deck = match db
            .apply(
                &Command::CreateDeck {
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
                &Command::CreateCard {
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
        target = card("Krebs cycle\n---\nThe second stage");
        first = card("Notes\n---\nSee [[Krebs cycle]]");
        second = card("More notes\n---\nAlso [[Krebs cycle]]");
        dangling = card("Loose\n---\nSee [[Glycolysis]]");
    }
    let handle = Handle::spawn(
        Db::open(&path).expect("open"),
        Box::new(typr_core::clock::SystemClock),
        std::sync::Arc::new(|| {}),
    )
    .expect("spawn");
    (handle, target, first, second, dangling)
}

/// The editor on `card`, with the library published from the store it came from.
fn editing(handle: Handle, card: Id) -> State {
    let mut state = State {
        store: Some(handle),
        ..State::default()
    };
    refresh_library(&mut state);
    <State as crate::library::Host>::open_card(&mut state, card);
    state
}

/// The centre of the drawn link reading `label`, in window points.
fn link_at(runtime: &rune::Runtime<State>, label: &str) -> Option<(f32, f32)> {
    runtime
        .elements()
        .into_iter()
        .find(|element| {
            element.role == "link"
                && (element.label.as_deref() == Some(label)
                    || element.text.as_deref() == Some(label))
        })
        .map(|element| {
            (
                element.rect[0] + element.rect[2] * 0.5,
                element.rect[1] + element.rect[3] * 0.5,
            )
        })
}

/// A reference drawn in the preview opens the card it names.
#[test]
fn a_reference_opens_the_card_it_names() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (handle, target, referrer, _, _) = referencing_library(&dir);
    let state = editing(handle, referrer);
    assert_eq!(state.editor.card, Some(referrer));
    assert!(
        state.editor.unresolved().is_empty(),
        "the reference resolves, so nothing is reported"
    );
    let mut runtime = rune::Runtime::new(
        state,
        crate::view,
        (900.0, 600.0),
        rune::Env::new(true, 1.0),
    );
    for _ in 0..30 {
        runtime.frame(1.0 / 60.0);
    }
    let (x, y) = link_at(&runtime, "Krebs cycle").expect("the reference is drawn as a link");
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.0);
    assert_eq!(
        runtime.state.editor.card,
        Some(target),
        "the reference opened the card it names"
    );
}

/// A reference that names no card is still the author's sentence, and the editor says so.
#[test]
fn an_unresolved_reference_says_so() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (handle, _, _, _, dangling) = referencing_library(&dir);
    let state = editing(handle, dangling);
    let mut runtime = rune::Runtime::new(
        state,
        crate::view,
        (900.0, 600.0),
        rune::Env::new(true, 1.0),
    );
    for _ in 0..30 {
        runtime.frame(1.0 / 60.0);
    }
    assert!(
        runtime.find("unresolved-0").is_some(),
        "the unresolved reference is reported where the preview is"
    );
    let shown: Vec<String> = runtime
        .elements()
        .into_iter()
        .filter_map(|element| element.text)
        .collect();
    assert!(
        shown.iter().any(|text| text.contains("[[Glycolysis]]")),
        "it names the reference that resolves to nothing: {shown:?}"
    );
}

/// The other half of a reference: every card that names this one, and a way to reach it.
#[test]
fn the_backlink_panel_lists_every_referrer() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (handle, target, first, second, _) = referencing_library(&dir);
    let state = editing(handle, target);
    assert_eq!(
        state.editor.backlinks.len(),
        2,
        "both referrers are listed on the card they name"
    );
    let mut runtime = rune::Runtime::new(
        state,
        crate::view,
        (900.0, 600.0),
        rune::Env::new(true, 1.0),
    );
    for _ in 0..30 {
        runtime.frame(1.0 / 60.0);
    }
    for card in [first, second] {
        assert!(
            runtime.find(&format!("backlink-{card}")).is_some(),
            "the referrer {card} is a control"
        );
    }
    let (x, y) = runtime
        .find(&format!("backlink-{first}"))
        .expect("the first referrer is drawn");
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.0);
    assert_eq!(
        runtime.state.editor.card,
        Some(first),
        "the backlink opened the card that refers here"
    );
}

/// Typing `[[` offers the library's names, and choosing one finishes the reference.
#[test]
fn typing_two_brackets_offers_names_and_a_choice_fills_them() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (handle, _, referrer, _, _) = referencing_library(&dir);
    let mut state = editing(handle, referrer);
    state.editor.buffer.replace_text("See [[Kr".to_string());
    state.editor.settle();
    let mut runtime = rune::Runtime::new(
        state,
        crate::view,
        (900.0, 600.0),
        rune::Env::new(true, 1.0),
    );
    for _ in 0..30 {
        runtime.frame(1.0 / 60.0);
    }
    assert!(
        runtime.find("reference-picker").is_some(),
        "the picker is drawn while a reference is being typed"
    );
    let (x, y) = runtime
        .find("reference-option-0")
        .expect("the matching card is offered");
    assert!(
        runtime
            .elements()
            .iter()
            .any(
                |element| element.key.as_deref() == Some("reference-option-0")
                    && element.label.as_deref() == Some("Krebs cycle")
            ),
        "the option is the card's name"
    );
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.0);
    assert_eq!(
        runtime.state.editor.buffer.text, "See [[Krebs cycle]]",
        "the choice finishes the reference and closes it"
    );
}
