//! Cards, decks and tags as the library screen sees them.

use super::*;

#[test]
fn a_prompt_appears_with_the_second_side_and_survives_edits() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Mixed");

    let note = card(&mut db, deck, "just a note");
    assert!(
        db.prompt_keys(note).expect("keys").is_empty(),
        "a note has no prompt"
    );

    let two_sided = card(&mut db, deck, "front\n---\nback");
    assert_eq!(
        db.prompt_keys(two_sided).expect("keys"),
        vec!["fwd".to_string()]
    );

    // Turned into a note: the prompt and its schedule are kept, the card just stops queueing.
    db.apply(
        &Command::SaveCard {
            id: two_sided,
            markdown: "front".to_string(),
        },
        0,
    )
    .expect("save");
    let sides = db
        .snapshot()
        .expect("snapshot")
        .cards
        .iter()
        .find(|c| c.id == two_sided)
        .expect("card")
        .sides;
    assert_eq!(sides, 1);
    assert_eq!(
        db.prompt_keys(two_sided).expect("keys"),
        vec!["fwd".to_string()]
    );

    // And back again: still exactly one prompt, not two.
    db.apply(
        &Command::SaveCard {
            id: two_sided,
            markdown: "front\n---\nback".to_string(),
        },
        0,
    )
    .expect("save");
    assert_eq!(db.prompt_keys(two_sided).expect("keys").len(), 1);
}

#[test]
fn grading_a_note_is_refused() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Notes");
    let note = card(&mut db, deck, "nothing to recall");

    let error = db
        .apply(
            &Command::Grade {
                card: note,
                key: "fwd".to_string(),
                grade: Grade::Remembered,
                duration_ms: None,
            },
            0,
        )
        .expect_err("no prompt");
    assert!(matches!(error, StoreError::NoPrompt { .. }), "{error}");
}

#[test]
fn a_deck_cannot_be_moved_inside_itself() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let parent = deck(&mut db, "Parent");
    let child = match db.apply(
        &Command::CreateDeck {
            parent: Some(parent),
            name: "Child".to_string(),
        },
        0,
    ) {
        Ok(Outcome::Deck(id)) => id,
        other => panic!("{other:?}"),
    };
    let grandchild = match db.apply(
        &Command::CreateDeck {
            parent: Some(child),
            name: "Grandchild".to_string(),
        },
        0,
    ) {
        Ok(Outcome::Deck(id)) => id,
        other => panic!("{other:?}"),
    };

    let error = db
        .apply(
            &Command::MoveDeck {
                id: parent,
                parent: Some(grandchild),
            },
            0,
        )
        .expect_err("cycle");
    assert!(
        matches!(error, StoreError::Cycle { deck } if deck == parent),
        "{error}"
    );
    let error = db
        .apply(
            &Command::MoveDeck {
                id: parent,
                parent: Some(parent),
            },
            0,
        )
        .expect_err("cycle");
    assert!(matches!(error, StoreError::Cycle { .. }), "{error}");

    // A legal move out of the tree, and one into a sibling branch, both work.
    db.apply(
        &Command::MoveDeck {
            id: grandchild,
            parent: None,
        },
        0,
    )
    .expect("move to root");
    db.apply(
        &Command::MoveDeck {
            id: child,
            parent: None,
        },
        0,
    )
    .expect("move to root");
    let roots = db
        .snapshot()
        .expect("snapshot")
        .decks
        .iter()
        .filter(|deck| deck.parent.is_none())
        .count();
    assert_eq!(roots, 3);
}

#[test]
fn deck_names_are_unique_under_one_parent_and_free_elsewhere() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let first = deck(&mut db, "Japanese");
    let second = deck(&mut db, "Spanish");

    let error = db
        .apply(
            &Command::CreateDeck {
                parent: None,
                name: "JAPANESE".to_string(),
            },
            0,
        )
        .expect_err("duplicate");
    assert!(error.to_string().contains("UNIQUE"), "{error}");

    db.apply(
        &Command::CreateDeck {
            parent: Some(first),
            name: "Verbs".to_string(),
        },
        0,
    )
    .expect("nested");
    db.apply(
        &Command::CreateDeck {
            parent: Some(second),
            name: "Verbs".to_string(),
        },
        0,
    )
    .expect("nested elsewhere");
}

#[test]
fn a_moved_card_keeps_its_prompt_and_due_date() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let source = deck(&mut db, "Source");
    let destination = deck(&mut db, "Destination");
    let card = card(&mut db, source, "front\n---\nback");
    schedule(&mut db, card, NOON - 3 * HOUR);

    let before = db.prompt(card, "fwd").expect("prompt").expect("exists");
    db.apply(
        &Command::MoveCard {
            id: card,
            deck: destination,
        },
        NOON,
    )
    .expect("move");

    assert_eq!(
        db.prompt(card, "fwd").expect("prompt").expect("exists"),
        before
    );
    let moved = db
        .snapshot()
        .expect("snapshot")
        .cards
        .into_iter()
        .find(|row| row.id == card)
        .expect("card");
    assert_eq!(moved.deck, destination);
    assert_eq!(
        db.due_prompts(NOON, 0, 1).expect("due").len(),
        1,
        "still due, now under the new deck"
    );
}

#[test]
fn tags_match_without_case_and_keep_the_first_casing() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Tags");
    let first = card(&mut db, deck, "front\n---\nback");
    let second = card(&mut db, deck, "front\n---\nback");
    let heading = card(&mut db, deck, "# Heading\n\ntext");

    db.apply(
        &Command::SetTags {
            card: first,
            tags: vec!["Grammar".into(), "grammar".into(), "verbs".into()],
        },
        0,
    )
    .expect("tags");
    db.apply(
        &Command::SetTags {
            card: second,
            tags: vec!["GRAMMAR".into()],
        },
        0,
    )
    .expect("tags");

    let summary = |id| {
        db.snapshot()
            .expect("snapshot")
            .cards
            .into_iter()
            .find(|card| card.id == id)
            .expect("card")
    };
    assert_eq!(
        summary(first).tags,
        vec!["Grammar".to_string(), "verbs".to_string()]
    );
    assert_eq!(
        summary(second).tags,
        vec!["Grammar".to_string()],
        "the first casing is library-wide"
    );
    assert_eq!(
        db.cards_with_tag("grammar").expect("filter"),
        vec![first, second]
    );
    assert_eq!(
        db.tag_names().expect("names"),
        vec!["Grammar".to_string(), "verbs".to_string()]
    );
    assert!(summary(heading).tags.is_empty(), "a heading is not a tag");
}

#[test]
fn search_folds_case_and_diacritics_within_the_deck_subtree() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let spanish = deck(&mut db, "Spanish");
    let songs = match db.apply(
        &Command::CreateDeck {
            parent: Some(spanish),
            name: "Songs".to_string(),
        },
        0,
    ) {
        Ok(Outcome::Deck(id)) => id,
        other => panic!("{other:?}"),
    };
    let other = deck(&mut db, "Other");

    let accented = card(&mut db, songs, "Canción\n---\nuna canción");
    let elsewhere = card(&mut db, other, "canción de cuna\n---\notra");
    let unrelated = card(&mut db, other, "Grammar\n---\nthe rules");

    assert_eq!(
        matching(&db, "cancion", None, CardMode::Active),
        vec![accented, elsewhere]
    );
    assert_eq!(
        matching(&db, "CANCION", Some(spanish), CardMode::Active),
        vec![accented],
        "subtree only"
    );
    assert_eq!(
        matching(&db, "grammar", None, CardMode::Active),
        vec![unrelated]
    );
    assert!(
        matching(&db, "   ", None, CardMode::Active).is_empty(),
        "blank matches nothing"
    );
}

#[test]
fn archiving_a_card_takes_it_out_of_the_queue_and_restoring_keeps_its_date() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Archive me");
    let card = card(&mut db, deck, "front\n---\nback");
    schedule(&mut db, card, NOON - HOUR);

    db.apply(
        &Command::ArchiveCard {
            id: card,
            archived: true,
        },
        NOON,
    )
    .expect("archive");
    assert!(db.due_prompts(NOON, 0, 1).expect("due").is_empty());
    let prompt = db
        .prompt(card, "fwd")
        .expect("prompt")
        .expect("still stored");
    assert_eq!(
        prompt.due_at,
        Some(NOON - HOUR),
        "archiving is not a schedule change"
    );

    db.apply(
        &Command::ArchiveCard {
            id: card,
            archived: false,
        },
        NOON,
    )
    .expect("restore");
    assert_eq!(db.due_prompts(NOON, 0, 1).expect("due").len(), 1);
    assert_eq!(
        db.prompt(card, "fwd")
            .expect("prompt")
            .expect("stored")
            .due_at,
        Some(NOON - HOUR)
    );
}

#[test]
fn a_card_under_an_archived_deck_leaves_the_queues_while_a_sibling_deck_stays() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let parent = deck(&mut db, "Parent");
    let hidden = match db.apply(
        &Command::CreateDeck {
            parent: Some(parent),
            name: "Hidden".to_string(),
        },
        0,
    ) {
        Ok(Outcome::Deck(id)) => id,
        other => panic!("{other:?}"),
    };
    let visible = match db.apply(
        &Command::CreateDeck {
            parent: Some(parent),
            name: "Visible".to_string(),
        },
        0,
    ) {
        Ok(Outcome::Deck(id)) => id,
        other => panic!("{other:?}"),
    };
    let hidden_card = card(&mut db, hidden, "hidden\n---\nside");
    let visible_card = card(&mut db, visible, "visible\n---\nside");
    db.apply(
        &Command::ArchiveDeck {
            id: hidden,
            archived: true,
        },
        NOON,
    )
    .expect("archive");

    let new: Vec<Id> = db
        .new_prompts(NOON, 0, 1)
        .expect("new")
        .into_iter()
        .map(|p| p.card)
        .collect();
    assert!(
        !new.contains(&hidden_card),
        "an archived ancestor hides the card"
    );
    assert!(new.contains(&visible_card));

    schedule(&mut db, hidden_card, NOON - HOUR);
    schedule(&mut db, visible_card, NOON - HOUR);
    let due: Vec<Id> = db
        .due_prompts(NOON, 0, 1)
        .expect("due")
        .into_iter()
        .map(|p| p.card)
        .collect();
    assert_eq!(due, vec![visible_card]);
}

#[test]
fn a_restored_card_keeps_its_history_and_an_emptied_one_loses_everything() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Trash");
    let kept = card(&mut db, deck, "kept\n---\nside");
    let doomed = card(&mut db, deck, "doomed\n---\nside");
    schedule(&mut db, doomed, NOON - HOUR);
    db.apply(
        &Command::Grade {
            card: doomed,
            key: "fwd".to_string(),
            grade: Grade::Remembered,
            duration_ms: None,
        },
        NOON,
    )
    .expect("grade");
    assert_eq!(db.record_total().expect("total"), 2);

    db.apply(&Command::TrashCard { id: doomed }, NOON)
        .expect("trash");
    assert!(
        matching(&db, "doomed", None, CardMode::Active).is_empty(),
        "trash is not searchable"
    );
    db.apply(&Command::RestoreCard { id: doomed }, NOON)
        .expect("restore");
    assert_eq!(
        db.review_count(doomed).expect("count"),
        2,
        "restoring keeps history"
    );

    db.apply(&Command::TrashCard { id: doomed }, NOON)
        .expect("trash again");
    db.apply(&Command::EmptyTrash, NOON).expect("empty");

    assert_eq!(
        db.record_total().expect("total"),
        0,
        "the log went with the card"
    );
    let cards = db.snapshot().expect("snapshot").cards;
    assert_eq!(
        cards.iter().map(|row| row.id).collect::<Vec<_>>(),
        vec![kept]
    );
    assert!(matching(&db, "doomed", None, CardMode::Trash).is_empty());
    assert!(db.prompt(doomed, "fwd").expect("prompt").is_none());
}

#[test]
fn a_percent_or_underscore_in_the_query_matches_itself() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Search");
    card(&mut db, deck, "Off by 100%\n---\nback");
    card(&mut db, deck, "Off by 100 percent\n---\nback");

    assert_eq!(
        matching(&db, "100%", None, CardMode::Active).len(),
        1,
        "only the literal percent matches"
    );
    assert!(
        matching(&db, "_", None, CardMode::Active).is_empty(),
        "an underscore is not a single-character wildcard"
    );
}

#[test]
fn a_deck_moves_under_another_deck() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let a = deck(&mut db, "A");
    let b = deck(&mut db, "B");
    let child = match db.apply(
        &Command::CreateDeck {
            parent: Some(a),
            name: "Child".to_string(),
        },
        0,
    ) {
        Ok(Outcome::Deck(id)) => id,
        other => panic!("{other:?}"),
    };

    assert!(matches!(
        db.apply(
            &Command::MoveDeck {
                id: b,
                parent: Some(a)
            },
            0
        ),
        Ok(Outcome::Changed(1))
    ));
    assert!(matches!(
        db.apply(
            &Command::MoveDeck {
                id: child,
                parent: Some(b)
            },
            0
        ),
        Ok(Outcome::Changed(1))
    ));
    assert!(matches!(
        db.apply(
            &Command::MoveDeck {
                id: b,
                parent: None
            },
            0
        ),
        Ok(Outcome::Changed(1))
    ));

    let snapshot = db.snapshot().expect("a snapshot");
    let parent_of = |id: Id| {
        snapshot
            .decks
            .iter()
            .find(|deck| deck.id == id)
            .expect("the deck")
            .parent
    };
    assert_eq!(parent_of(b), None, "B is back at the top");
    assert_eq!(parent_of(child), Some(b), "the child followed its parent");
    assert_eq!(parent_of(a), None);
}

#[test]
fn the_view_and_the_snapshot_agree_on_what_is_studyable() {
    let dir = TempDir::new().expect("temp dir");
    let store = spawn_temp(&dir);
    store.create_deck(None, "Keep");
    store.create_deck(None, "Gone");
    store.create_deck(Some(2), "Under gone");
    store.create_card(1, "front\n---\nback");
    store.create_card(1, "a note with one side");
    store.create_card(1, "front\n---\nback");
    store.create_card(3, "front\n---\nback");
    store.create_card(2, "front\n---\nback");
    store.create_card(1, "front\n---\nback");
    wait_for(&store, 9);
    let trashed = store.snapshot().cards[2].id;
    let archived = store.snapshot().cards[5].id;
    store.trash_card(trashed);
    store.archive_card(archived, true);
    store.archive_deck(2, true);
    wait_for(&store, 3);

    let snapshot = store.snapshot();
    let from_snapshot: Vec<Id> = snapshot
        .cards_matching("", None, None, CardMode::Active)
        .into_iter()
        .filter(|card| card.sides >= 2)
        .map(|card| card.id)
        .collect();
    let mut expected = from_snapshot.clone();
    expected.sort_unstable();

    store.new_prompts(0, 0, 7);
    let events = wait_for(&store, 1);
    let from_sql = match events.last().map(|event| &event.result) {
        Some(Ok(Outcome::New(prompts))) => {
            let mut ids: Vec<Id> = prompts.iter().map(|prompt| prompt.card).collect();
            ids.sort_unstable();
            ids
        }
        other => panic!("expected the new queue, got {other:?}"),
    };

    assert_eq!(
        from_sql, expected,
        "the view and the snapshot must name the same studyable cards"
    );
    assert_eq!(from_sql.len(), 1, "only the kept two-sided card is left");
}

#[test]
fn a_new_card_cap_that_is_not_a_number_is_refused() {
    let dir = TempDir::new().expect("temp dir");
    let db = open_temp(&dir);
    let error = db
        .apply(
            &Command::SetSetting {
                key: "new_per_day".to_string(),
                value: "plenty".to_string(),
            },
            0,
        )
        .expect_err("refused");
    assert!(
        error.to_string().contains("not a valid value"),
        "got {error}"
    );
    assert_eq!(
        db.settings().expect("settings").new_per_day,
        Settings::DEFAULT_NEW_PER_DAY,
        "and nothing was stored"
    );
}
