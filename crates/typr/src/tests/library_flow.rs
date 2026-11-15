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
            .is_some_and(|why| why.contains("a deck under that parent called")
                && why.contains("already exists")),
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

/// A key event for one letter, as a keyboard sends it.
fn letter_press(letter: &str) -> KeyEvent {
    KeyEvent {
        key: Key::Text(letter.to_string()),
        mods: Mods::default(),
    }
}

#[test]
fn the_library_keys_do_what_the_table_says() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = library_with_one_card(&dir);
    let mut tx = Tx::default();
    let mut state = State {
        store: Some(store),
        ..State::default()
    };

    // `n` asks for the new-card queue.
    assert!(on_key(&mut state, &letter_press("n"), &mut tx));
    assert_eq!(state.wanted, Some(Queue::New));

    // `c` opens a card with no id, which is the editor on a new one.
    state.wanted = None;
    assert!(on_key(&mut state, &letter_press("c"), &mut tx));
    assert_eq!(state.screen, Screen::Editor);
    assert_eq!(state.editor.card, None);

    // `s` asks for the due queue, and a key the table does not bind is refused.
    state.screen = Screen::Library;
    assert!(on_key(&mut state, &letter_press("s"), &mut tx));
    assert_eq!(state.wanted, Some(Queue::Due));
    assert!(!on_key(&mut state, &letter_press("q"), &mut tx));
}

#[test]
fn an_open_panel_keeps_its_keystrokes() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = library_with_one_card(&dir);
    let mut tx = Tx::default();
    let mut state = State {
        store: Some(store),
        ..State::default()
    };
    state.panel.open = true;

    // Typing a letter belongs to the filter, not to the command behind the dialog.
    assert!(
        !on_key(&mut state, &letter_press("n"), &mut tx),
        "a letter behind the panel is refused"
    );
    assert_eq!(state.wanted, None, "no session was asked for");

    // Escape closes the panel itself.
    let escape = KeyEvent {
        key: Key::Escape,
        mods: Mods::default(),
    };
    assert!(on_key(&mut state, &escape, &mut tx));
    assert!(!state.panel.open);
}

#[test]
fn typing_in_the_palette_does_not_grade() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = library_with_one_card(&dir);
    let mut tx = Tx::default();
    let mut state = State {
        store: Some(store),
        screen: Screen::Session,
        session: session_on(1),
        ..State::default()
    };
    state.palette.open = true;

    // `j` would grade if the palette were not open; here it belongs to the filter.
    assert!(!on_key(&mut state, &letter_press("j"), &mut tx));
    assert_eq!(state.session.remaining(), 1, "nothing was graded");
}

#[test]
fn the_row_toggles_reverse_for_one_card() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("typr.sqlite3");
    let store = {
        let db = Db::open(&path).expect("open");
        let deck = match db
            .apply(
                &typr_core::store::Command::CreateDeck {
                    parent: None,
                    name: "Languages".to_string(),
                },
                0,
            )
            .expect("deck")
        {
            typr_core::store::Outcome::Deck(id) => id,
            other => panic!("creating a deck: {other:?}"),
        };
        for markdown in ["front\n---\nback", "front\n---\nback"] {
            db.apply(
                &typr_core::store::Command::CreateCard {
                    deck,
                    markdown: markdown.to_string(),
                },
                0,
            )
            .expect("card");
        }
        // Both cards are asked from both ends until one of them says otherwise.
        db.apply(
            &typr_core::store::Command::SetDeckReverse {
                deck,
                reverse: Some(true),
            },
            0,
        )
        .expect("reverse on for the deck");
        Handle::spawn(
            db,
            Box::new(typr_core::clock::SystemClock),
            std::sync::Arc::new(|| {}),
        )
        .expect("spawn")
    };

    let mut state = State {
        store: Some(store),
        seeded: true,
        ..State::default()
    };
    refresh_library(&mut state);
    assert_eq!(
        reverse_queue(&path),
        vec![1, 2],
        "the deck turns both cards around"
    );

    // What the tile's menu button calls.
    library::Host::set_card_reverse(&mut state, 1, Some(false));
    settle_last(&mut state);
    assert_eq!(
        reverse_queue(&path),
        vec![2],
        "the card that said no stops queueing in reverse, its sibling does not"
    );
    assert_eq!(
        state
            .library
            .snapshot()
            .cards
            .iter()
            .find(|card| card.id == 1)
            .expect("card 1")
            .reverse,
        Some(false),
        "and the drawn card carries its own setting"
    );
}

/// Wait until every in-flight command has been answered and drawn.
///
/// The answers leave `State::sent` only when a frame drains them, so the loop drives a frame rather
/// than polling the store directly.
fn settle_last(state: &mut State) {
    for _ in 0..200 {
        let mut wake = None;
        crate::frame_step(state, 0, &mut wake);
        if state.sent.is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    refresh_library(state);
}

/// The cards whose reverse prompt the new queue would offer, read from a second connection.
fn reverse_queue(path: &std::path::Path) -> Vec<Id> {
    let db = Db::open(path).expect("reopen");
    let mut cards: Vec<Id> = db
        .new_prompts(0, 0, 1, None)
        .expect("new")
        .into_iter()
        .filter(|prompt| prompt.key == "rev")
        .map(|prompt| prompt.card)
        .collect();
    cards.sort_unstable();
    cards
}

#[test]
fn a_deck_can_be_set_back_to_inherit() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("typr.sqlite3");
    let store = {
        let db = Db::open(&path).expect("open");
        let parent = match db
            .apply(
                &typr_core::store::Command::CreateDeck {
                    parent: None,
                    name: "Languages".to_string(),
                },
                0,
            )
            .expect("deck")
        {
            typr_core::store::Outcome::Deck(id) => id,
            other => panic!("creating a deck: {other:?}"),
        };
        let child = match db
            .apply(
                &typr_core::store::Command::CreateDeck {
                    parent: Some(parent),
                    name: "Japanese".to_string(),
                },
                0,
            )
            .expect("child deck")
        {
            typr_core::store::Outcome::Deck(id) => id,
            other => panic!("creating a child deck: {other:?}"),
        };
        db.apply(
            &typr_core::store::Command::CreateCard {
                deck: child,
                markdown: "front\n---\nback".to_string(),
            },
            0,
        )
        .expect("card");
        db.apply(
            &typr_core::store::Command::SetDeckReverse {
                deck: parent,
                reverse: Some(true),
            },
            0,
        )
        .expect("reverse on for the parent");
        Handle::spawn(
            db,
            Box::new(typr_core::clock::SystemClock),
            std::sync::Arc::new(|| {}),
        )
        .expect("spawn")
    };

    let mut state = State {
        store: Some(store),
        seeded: true,
        ..State::default()
    };
    refresh_library(&mut state);
    assert_eq!(
        reverse_queue(&path),
        vec![1],
        "the child deck inherits the parent's setting"
    );

    let child = state
        .library
        .snapshot()
        .decks
        .iter()
        .find(|deck| deck.name == "Japanese")
        .expect("child deck")
        .id;
    library::Host::set_deck_reverse(&mut state, child, Some(false));
    settle_last(&mut state);
    assert!(
        reverse_queue(&path).is_empty(),
        "the child's own setting wins over its parent's"
    );

    // The three-state control's middle position: hand the decision back up.
    library::Host::set_deck_reverse(&mut state, child, None);
    settle_last(&mut state);
    assert_eq!(
        reverse_queue(&path),
        vec![1],
        "inheriting again follows the parent"
    );
    assert_eq!(
        state
            .library
            .snapshot()
            .decks
            .iter()
            .find(|deck| deck.id == child)
            .expect("child deck")
            .reverse,
        None,
        "and the drawn deck says it inherits"
    );
}

#[test]
fn the_editor_says_a_card_inherits_its_reverse_setting() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = library_with_one_card(&dir);
    let mut state = State {
        store: Some(store),
        seeded: true,
        ..State::default()
    };
    refresh_library(&mut state);

    library::Host::open_card(&mut state, 1);
    assert_eq!(state.screen, Screen::Editor, "the card opens in the editor");
    assert_eq!(
        state.editor.reverse, None,
        "a card that never had the setting inherits its deck's"
    );

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
        drawn.iter().any(|line| line == "Reverse: Inherit"),
        "the header names the state: {drawn:?}"
    );
}

#[test]
fn a_deck_template_is_edited_and_its_cards_follow() {
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
    let card = match db
        .apply(
            &typr_core::store::Command::CreateCard {
                deck,
                markdown: "typed by hand".to_string(),
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
    let snapshot = state.store.as_ref().expect("store").snapshot();
    state.library.set_snapshot(snapshot);
    state.library.select_deck(Some(deck));

    LibraryHost::begin_template(&mut state);
    assert_eq!(state.library.templating, Some(deck));
    state
        .library
        .template_field()
        .replace_text("<<Word>>\n---\n<<Meaning>>".to_string());
    assert!(
        LibraryHost::finish_template(&mut state),
        "the template was sent"
    );

    for _ in 0..200 {
        state.drain();
        let texts: Vec<String> = state
            .library
            .snapshot()
            .cards
            .iter()
            .map(|card| card.markdown.clone())
            .collect();
        if texts.iter().any(|text| text == "gato\n---\n") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let listed = state
        .library
        .snapshot()
        .cards
        .iter()
        .find(|own| own.id == card)
        .cloned()
        .expect("the card");
    assert_eq!(
        listed.markdown, "gato\n---\n",
        "the card follows the deck's template"
    );
    let stored_deck = state
        .library
        .snapshot()
        .decks
        .iter()
        .find(|own| own.id == deck)
        .cloned()
        .expect("the deck");
    assert_eq!(
        stored_deck.template.as_deref(),
        Some("<<Word>>\n---\n<<Meaning>>")
    );
}

/// A library with one deck, two cards, one of them pinned, and a view that asks for pins alone.
fn view_fixture() -> (tempfile::TempDir, i64) {
    use typr_core::store::{CardMode, Sort, SortDir, ViewSpec};

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
    let mut cards = Vec::new();
    for text in ["perro\n---\ndog", "gato\n---\ncat"] {
        let id = match db
            .apply(
                &typr_core::store::Command::CreateCard {
                    deck,
                    markdown: text.to_string(),
                },
                0,
            )
            .expect("card")
        {
            typr_core::store::Outcome::Card(id) => id,
            other => panic!("creating a card: {other:?}"),
        };
        cards.push(id);
    }
    // The pinned card is the one on the right: the view asks for pins alone, so only it is listed.
    db.apply(
        &typr_core::store::Command::SetPinned {
            card: cards[1],
            pinned: true,
        },
        0,
    )
    .expect("pin");
    let view = match db
        .apply(
            &typr_core::store::Command::CreateView {
                spec: ViewSpec {
                    deck: Some(deck),
                    query: String::new(),
                    tags: Vec::new(),
                    mode: CardMode::Active,
                    pinned_only: true,
                    sort: Sort::FirstSide,
                    dir: SortDir::Ascending,
                },
                name: "Exam week".to_string(),
            },
            0,
        )
        .expect("view")
    {
        typr_core::store::Outcome::View(id) => id,
        other => panic!("creating a view: {other:?}"),
    };
    (dir, view)
}

/// A state over the library the fixture wrote, with the first snapshot already published.
fn view_state(dir: &tempfile::TempDir) -> State {
    let db = typr_core::store::Db::open(&dir.path().join("library.sqlite3")).expect("open");
    let waker: Waker = std::sync::Arc::new(|| {});
    let handle = Handle::spawn(db, Box::new(typr_core::clock::SystemClock), waker).expect("spawn");
    let mut state = State {
        store: Some(handle),
        ..State::default()
    };
    let snapshot = state.store.as_ref().expect("store").snapshot();
    state.library.set_snapshot(snapshot);
    state
}

#[test]
fn selecting_a_view_applies_every_part_of_it() {
    use typr_core::store::Sort;

    let (dir, view) = view_fixture();
    let mut state = view_state(&dir);
    let deck = state.library.snapshot().decks[0].id;
    let pinned = state
        .library
        .snapshot()
        .cards
        .iter()
        .find(|card| card.pinned)
        .map(|card| card.id)
        .expect("the pinned card");
    state.library.select_deck(Some(deck));

    state.library.select_view(view);
    assert_eq!(
        state.library.selected_deck, None,
        "the tree selection clears"
    );
    assert_eq!(
        state.library.spec().deck,
        Some(deck),
        "the scope still filters"
    );
    assert!(
        state.library.pinned_only,
        "the pin filter is part of the view"
    );
    assert_eq!(state.library.sort, Sort::FirstSide, "and so is the order");
    assert_eq!(
        state
            .library
            .cards()
            .iter()
            .map(|card| card.id)
            .collect::<Vec<_>>(),
        vec![pinned],
        "one pinned card, and the view's sort is what orders it"
    );
}

#[test]
fn a_view_can_be_renamed_and_deleted() {
    let (dir, view) = view_fixture();
    let mut state = view_state(&dir);
    state.library.select_view(view);

    state.library.begin_view_rename(view);
    state
        .library
        .view_field()
        .replace_text("Verb drills".to_string());
    assert!(
        LibraryHost::finish_view_rename(&mut state),
        "the new name was sent"
    );
    for _ in 0..200 {
        state.drain();
        if state
            .library
            .views()
            .iter()
            .any(|own| own.name == "Verb drills")
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(
        state.library.views()[0].name,
        "Verb drills",
        "the renamed view is the one the sidebar lists"
    );

    LibraryHost::delete_view(&mut state, view);
    for _ in 0..200 {
        state.drain();
        if state.library.views().is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(state.library.views().is_empty(), "the view is gone");
    assert_eq!(
        state.library.selected_view, None,
        "and the list stops calling itself a view"
    );
    assert_eq!(
        state.library.snapshot().cards.len(),
        2,
        "deleting a view kept every card it matched"
    );
}

#[test]
fn the_pin_button_pins_and_unpins_a_card() {
    let (dir, _) = view_fixture();
    let state = view_state(&dir);
    let mut runtime = rune::Runtime::new(
        state,
        |s: &State, _: &mut rune::Cx<State>| crate::library::view::<State>(s),
        (1200.0, 700.0),
        rune::Env::new(true, 1.0),
    );
    runtime.frame(0.0);
    let (x, y) = runtime.find("pin-1").expect("the pin button");
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);

    let pinned = |runtime: &rune::Runtime<State>| {
        runtime
            .state
            .library
            .snapshot()
            .cards
            .iter()
            .find(|card| card.id == 1)
            .is_some_and(|card| card.pinned)
    };
    for _ in 0..200 {
        runtime.state.drain();
        runtime.frame(0.1);
        if pinned(&runtime) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(pinned(&runtime), "the press pinned the card in the store");
}

#[test]
fn deleting_the_view_that_is_showing_returns_to_the_whole_library() {
    let (dir, view) = view_fixture();
    let mut state = view_state(&dir);
    state.library.select_view(view);
    assert_eq!(state.library.cards().len(), 1, "the view narrows the list");

    state.delete_view(view);
    for _ in 0..200 {
        state.drain();
        if state.library.snapshot().views.is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    assert!(
        state.library.snapshot().views.is_empty(),
        "the view is gone"
    );
    assert_eq!(state.library.query.text, "", "the search went with it");
    assert_eq!(state.library.tag, None, "the tag went with it");
    assert!(!state.library.pinned_only, "the pin filter went with it");
    assert_eq!(state.library.cards().len(), 2, "every card is listed again");
}

#[test]
fn a_cram_session_with_nothing_selected_does_not_open() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = library_with_one_card(&dir);
    let mut tx = Tx::default();
    let mut state = State {
        store: Some(store),
        ..State::default()
    };
    // The one card carries no tag, so the filter selects nothing at all.
    state.library.tag = Some("exam".to_string());

    assert!(on_key(&mut state, &letter_press("x"), &mut tx));
    wait_for(&mut state, "the cram answer", |state| {
        state.cram_request.is_none()
    });

    assert!(
        matches!(state.screen, Screen::Library),
        "a run over nothing would open on a summary of no answers"
    );
    assert_ne!(
        state.session.mode(),
        Mode::Cram,
        "no session was opened for an empty filter"
    );
    assert!(
        state
            .failure
            .as_deref()
            .is_some_and(|failure| failure.contains("Nothing to cram")),
        "the screen says there is nothing to cram: {:?}",
        state.failure
    );
}

#[test]
fn cramming_a_tag_with_nothing_due_visits_exactly_its_cards() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("library.sqlite3");
    let db = typr_core::store::Db::open(&path).expect("open");
    db.apply(
        &typr_core::store::Command::SetSetting {
            key: typr_core::store::SettingKey::NewPerDay,
            value: typr_core::store::SettingValue::Number(0),
        },
        0,
    )
    .expect("the cap to none");
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
        typr_core::store::Outcome::Card(id) => id,
        other => panic!("creating a card: {other:?}"),
    };
    let perro = card("perro\n---\ndog");
    let gato = card("gato\n---\ncat");
    let sol = card("sol\n---\nsun");
    for tagged in [perro, gato] {
        db.apply(
            &typr_core::store::Command::SetTags {
                card: tagged,
                tags: vec!["exam".to_string()],
            },
            0,
        )
        .expect("tag");
    }
    // Neither queue would start: nothing is due, and the cap holds every new card back.
    let now = typr_core::clock::SystemClock.now_ms();
    assert!(
        db.due_prompts(now, 0, 1, None)
            .expect("due queue")
            .is_empty(),
        "nothing is due"
    );
    assert!(
        db.new_prompts(now, 0, 1, None)
            .expect("new queue")
            .is_empty(),
        "the cap holds the fresh cards back"
    );

    let waker: Waker = std::sync::Arc::new(|| {});
    let handle = Handle::spawn(db, Box::new(typr_core::clock::SystemClock), waker).expect("spawn");
    let mut tx = Tx::default();
    let mut state = State {
        store: Some(handle),
        ..State::default()
    };
    refresh_library(&mut state);
    state.library.tag = Some("exam".to_string());
    refresh_library(&mut state);
    assert_eq!(state.library.cards().len(), 2, "the tag narrows the list");

    assert!(on_key(&mut state, &letter_press("x"), &mut tx));
    wait_for(&mut state, "the cram session", |state| {
        matches!(state.screen, Screen::Session)
    });

    assert_eq!(state.session.mode(), Mode::Cram, "the run knows its mode");
    let mut walked = state.session.order();
    walked.sort_unstable();
    let mut expected = vec![perro, gato];
    expected.sort_unstable();
    assert_eq!(walked, expected, "it walks the tagged cards and no other");
    assert_ne!(walked, vec![sol], "the untagged card stays out");
    assert!(state.session.grades.is_empty(), "nothing is answered yet");
}

#[test]
fn importing_a_mochi_export_from_the_library_adds_its_cards() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = library_with_one_card(&dir);
    let mut state = State {
        store: Some(store),
        seeded: true,
        ..State::default()
    };
    state.editor.attachments = Attachments::detached(dir.path().join("attachments"));
    refresh_library(&mut state);
    let source =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/mochi-import.mochi");
    let _ = library::Host::import_file(&mut state, source).expect("sent");
    // The import is answered on the store thread; the screen follows when it lands.
    wait_for(&mut state, "the imported cards", |s| {
        s.library.snapshot().cards.len() == 3
            || s.failure
                .as_deref()
                .is_some_and(|line| line != "Importing…")
    });
    assert_eq!(
        state.library.snapshot().cards.len(),
        3,
        "the import answered instead of adding cards: {:?}",
        state.failure
    );
    let snapshot = state.library.snapshot();
    assert_eq!(snapshot.decks.len(), 2, "the archive's deck arrived");
    assert_eq!(snapshot.cards.len(), 3, "both archive cards arrived");
    assert_eq!(
        snapshot
            .cards
            .iter()
            .filter(|card| card.kind == CardKind::Diagram)
            .count(),
        1,
        "the picture card stayed a diagram"
    );
    assert!(
        snapshot
            .cards
            .iter()
            .any(|card| card.image.as_deref() == Some("pixel.png") && card.masks.len() == 1),
        "the picture and its mask arrived"
    );
    assert!(
        state
            .failure
            .as_deref()
            .is_some_and(|line| line.contains("Imported 2 cards across 1 deck")),
        "the line reports what was imported: {:?}",
        state.failure
    );
}

#[test]
fn exporting_and_importing_a_deck_through_the_library_round_trips() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = library_with_one_card(&dir);
    let mut state = State {
        store: Some(store),
        seeded: true,
        ..State::default()
    };
    refresh_library(&mut state);
    state.library.selected_deck = Some(state.library.snapshot().decks[0].id);
    let target = dir.path().join("biology.csv");
    let _ = library::Host::export_file(&mut state, target.clone()).expect("sent");
    wait_for(&mut state, "the export", |s| {
        target.exists() && s.failure.as_deref() == Some("Exported.")
    });
    let _ = library::Host::import_file(&mut state, target).expect("sent");
    wait_for(&mut state, "the re-import", |s| {
        s.library.snapshot().cards.len() == 2
    });
    assert_eq!(
        state.library.snapshot().cards.len(),
        2,
        "the exported card came back beside the one it came from"
    );
    assert!(
        state
            .failure
            .as_deref()
            .is_some_and(|line| line.contains("Imported 1 card:")),
        "the line reports the re-import: {:?}",
        state.failure
    );
}

#[test]
fn a_card_added_below_is_placed_where_it_was_asked() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut state = view_state(&dir);
    let deck = state
        .store
        .as_ref()
        .expect("store")
        .create_deck(None, "Notebook")
        .expect("the deck is created") as i64;
    for text in ["one", "two", "three"] {
        state
            .store
            .as_ref()
            .expect("store")
            .create_card(deck, text)
            .expect("the card is created");
    }
    for _ in 0..200 {
        state.drain();
        if state.library.snapshot().cards.len() == 3 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    // The snapshot lists by `updated_at`, so pick the card by its text rather than by position.
    let second = state
        .library
        .snapshot()
        .cards
        .iter()
        .find(|card| card.markdown == "two")
        .map(|card| card.id)
        .expect("the second card");
    state
        .library
        .set_view(deck, typr_core::library_view::ViewMode::Notebook);
    state.library.selected_deck = Some(deck);
    state.library.selected_card = Some(second);

    let mut tx = rune::Tx::default();
    assert!(typr::library::key(
        &mut state,
        &rune::KeyEvent {
            key: rune::Key::Text("o".to_string()),
            mods: rune::Mods::default(),
        },
        &mut tx,
    ));
    assert_eq!(state.screen, Screen::Editor, "the new card opens");
    state
        .editor
        .buffer
        .replace_text("two and a half".to_string());
    state.editor.refresh();
    assert!(state.persist());

    let mut texts: Vec<String> = Vec::new();
    for _ in 0..400 {
        state.drain();
        let mut order: Vec<(String, i64, Id)> = state
            .library
            .snapshot()
            .cards
            .iter()
            .map(|card| (card.markdown.clone(), card.position, card.id))
            .collect();
        order.sort_by_key(|(_, position, id)| (*position, *id));
        texts = order.iter().map(|(text, _, _)| text.clone()).collect();
        // The create publishes before the move does, so wait for the order itself rather than for
        // a position to become non-zero: the deck already held cards above zero.
        if texts == ["one", "two", "two and a half", "three"] {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!(
        "the new card never arrived in place: {texts:?} sent={:?} failure={:?}",
        state.sent, state.failure
    );
}

#[test]
fn moving_a_card_up_survives_a_restart() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("library.sqlite3");
    let mut state = view_state(&dir);
    let deck = state
        .store
        .as_ref()
        .expect("store")
        .create_deck(None, "Order")
        .expect("the deck is created") as i64;
    for text in ["one", "two", "three"] {
        state
            .store
            .as_ref()
            .expect("store")
            .create_card(deck, text)
            .expect("the card is created");
    }
    for _ in 0..200 {
        state.drain();
        if state.library.snapshot().cards.len() == 3 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let second = state
        .library
        .snapshot()
        .cards
        .iter()
        .find(|card| card.markdown == "two")
        .expect("the second card exists")
        .id;
    state.library.selected_deck = Some(deck);
    state.library.selected_card = Some(second);

    let mut tx = rune::Tx::default();
    assert!(typr::library::key(
        &mut state,
        &rune::KeyEvent {
            key: rune::Key::Up,
            mods: rune::Mods {
                alt: true,
                ..rune::Mods::default()
            },
        },
        &mut tx,
    ));
    let mut moved = false;
    for _ in 0..200 {
        state.drain();
        let order = typr::library::Library::deck_order(&state.library, deck);
        if order.first().map(|card| card.id) == Some(second) {
            moved = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(moved, "the second card takes the first place");
    drop(state);

    // A fresh open of the same file is the restart.
    let db = typr_core::store::Db::open(&path).expect("reopen");
    let snapshot = db.snapshot().expect("snapshot");
    let mut order: Vec<(i64, i64, String)> = snapshot
        .cards
        .iter()
        .map(|card| (card.position, card.id, card.markdown.clone()))
        .collect();
    order.sort();
    let texts: Vec<&str> = order.iter().map(|(_, _, text)| text.as_str()).collect();
    assert_eq!(texts, ["two", "one", "three"], "the order was written down");
}

#[test]
fn the_slash_focuses_the_search_field() {
    let (dir, _view) = view_fixture();
    let mut state = view_state(&dir);
    state.library.select_deck(None);
    let drawn = state;
    let mut runtime = rune::Runtime::new(
        drawn,
        |s: &State, _: &mut rune::Cx<State>| crate::library::view::<State>(s),
        (900.0, 600.0),
        rune::Env::new(true, 1.0),
    );
    runtime.on_key = Some(crate::on_key);
    runtime.frame(0.0);
    runtime.key(rune::KeyEvent {
        key: rune::Key::Text("/".into()),
        mods: rune::Mods::default(),
    });
    runtime.frame(0.0);
    runtime.key(rune::KeyEvent {
        key: rune::Key::Text("g".into()),
        mods: rune::Mods::default(),
    });
    runtime.frame(0.0);
    assert_eq!(
        runtime.state.library.query.text, "g",
        "the slash reached the field, and the next key went into it"
    );
}

#[test]
fn a_library_that_did_not_open_says_so_in_the_sidebar() {
    let state = State::default();
    assert!(state.store.is_none(), "the fixture has no library");
    let mut runtime = rune::Runtime::new(
        state,
        |s: &State, _: &mut rune::Cx<State>| crate::library::view::<State>(s),
        (900.0, 600.0),
        rune::Env::new(true, 1.0),
    );
    runtime.frame(0.0);
    let drawn: Vec<(String, Option<String>)> = runtime
        .elements()
        .iter()
        .filter(|element| !element.exiting)
        .filter(|element| {
            element.key.as_deref() == Some("sidebar-state-line")
                || element.key.as_deref() == Some("sidebar-state-reason")
        })
        .map(|element| {
            (
                element.key.clone().unwrap_or_default(),
                element.text.clone(),
            )
        })
        .collect();
    assert!(
        drawn.contains(&(
            "sidebar-state-line".to_string(),
            Some("Library closed".to_string())
        )),
        "a screen with no store draws the closed state, not the open one: {drawn:?}"
    );
    assert!(
        drawn
            .iter()
            .any(|(_, text)| text.as_deref() == Some("no library is open")),
        "and it names the reason: {drawn:?}"
    );
}
