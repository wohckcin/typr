use super::*;

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
            .due_prompts(far, 0, 1, None)
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
            .is_some_and(|why| why.contains("is already under that parent")),
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
    assert!(runtime.find("tile-1").is_some(), "the card is a tile");
    assert!(
        runtime.find("new-card-button").is_some(),
        "and a new one started"
    );

    // The tile's body is what the accessibility tree exposes as a button, so a press on it has to
    // open the card, not just select it.
    let (x, y) = runtime.find("tile-body-1").expect("the tile's body");
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.1);
    let mut state = runtime.state;
    assert!(
        matches!(state.screen, Screen::Editor),
        "pressing a tile opens that card"
    );
    assert_eq!(state.editor.card, Some(1), "the tile's own card opens");
    assert_eq!(
        state.editor.buffer.text, "front\n---\nback",
        "the tile's markdown is in the buffer"
    );

    library::Host::new_card(&mut state);
    assert!(matches!(state.screen, Screen::Editor), "a new card opens");
    assert_eq!(state.editor.card, None, "with no id until it is saved");
}

#[test]
fn pressing_a_tile_opens_its_card() {
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
    for _ in 0..3 {
        runtime.frame(0.1);
    }
    let (x, y) = runtime.find("tile-body-1").expect("the tile's body");
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.1);

    let state = runtime.state;
    assert!(
        matches!(state.screen, Screen::Editor),
        "the tile opens its card"
    );
    assert_eq!(state.library.selected_card, Some(1), "and selects it");
}

#[test]
fn the_deck_actions_still_run_from_a_tile() {
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
    for _ in 0..3 {
        runtime.frame(0.1);
    }

    // The footer has to sit inside the tile's own box, or the press never reaches it: a `grow`n
    // body contributed no height to the parent and the tile stayed as short as its footer.
    let elements = runtime.elements();
    let tile = elements
        .iter()
        .find(|e| e.key.as_deref() == Some("tile-1"))
        .expect("the tile")
        .rect;
    let menu = elements
        .iter()
        .find(|e| e.key.as_deref() == Some("menu-1"))
        .expect("the menu button")
        .rect;
    assert!(
        menu[1] + menu[3] <= tile[1] + tile[3] + 0.5
            && menu[0] + menu[2] <= tile[0] + tile[2] + 0.5,
        "the menu sits inside the tile: tile {tile:?} menu {menu:?}"
    );

    let (x, y) = (menu[0] + menu[2] / 2.0, menu[1] + menu[3] / 2.0);
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.1);
    assert_eq!(runtime.state.library.menu, Some(1), "the menu opens");

    let (x, y) = runtime.find("archive-1").expect("archive in the menu");
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    for _ in 0..100 {
        runtime.frame(0.1);
        let mut state = std::mem::take(&mut runtime.state);
        refresh_library(&mut state);
        let archived = state.library.cards().is_empty();
        runtime.state = state;
        if archived {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let state = runtime.state;
    assert!(
        state.library.cards().is_empty(),
        "an archived card leaves the active deck"
    );
    assert_eq!(state.library.menu, None, "and the menu closes");
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

    // The tile's menu offers the move, and the sidebar takes the card.
    let mut preview = State {
        library: Library::seeded(3),
        ..State::default()
    };
    preview.library.menu = Some(1);
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
        .due_prompts(i64::MAX / 4, 0, 1, None)
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
fn a_card_under_an_archived_deck_is_reachable_and_the_deck_can_come_back() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = library_with_a_card(&dir);
    let mut state = State {
        store: Some(store),
        ..State::default()
    };
    refresh_library(&mut state);
    assert_eq!(state.library.cards().len(), 1);

    // Give the card a due date, so a schedule lost by the round trip would show.
    let graded = state
        .store
        .as_ref()
        .expect("store")
        .grade(1, "fwd", Grade::Remembered, None)
        .expect("sent");
    settle(&mut state, graded);
    let before = due_at(&mut state, 1);
    assert!(before.is_some(), "the card has a due date to keep");

    LibraryHost::archive_deck(&mut state, 1);
    wait_for(&mut state, "the deck to be archived", |s| {
        s.library
            .snapshot()
            .decks
            .iter()
            .any(|deck| deck.id == 1 && deck.archived)
    });

    state.library.set_mode(CardMode::Archived);
    assert_eq!(
        state.library.cards().len(),
        1,
        "a card whose deck was archived is in the archived view, or it is nowhere"
    );

    state.library.select_deck(Some(1));
    let mut runtime = rune::Runtime::new(state, view, (900.0, 600.0), rune::Env::new(true, 1.0));
    runtime.frame(0.0);
    assert!(
        runtime.find("unarchive-deck").is_some(),
        "the archived deck offers a way back"
    );

    let mut state = std::mem::take(&mut runtime.state);
    drop(runtime);
    LibraryHost::unarchive_deck(&mut state, 1);
    wait_for(&mut state, "the deck to come back", |s| {
        s.library
            .snapshot()
            .decks
            .iter()
            .any(|deck| deck.id == 1 && !deck.archived)
    });

    state.library.set_mode(CardMode::Active);
    assert_eq!(
        state.library.cards().len(),
        1,
        "and the card is back in the library"
    );
    assert_eq!(
        due_at(&mut state, 1),
        before,
        "the deck coming back does not move the card's due date"
    );
}
