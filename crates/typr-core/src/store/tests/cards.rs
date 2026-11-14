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
        vec!["fwd".to_string(), "rev".to_string()],
        "two sides are two questions, one from each end"
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
        vec!["fwd".to_string(), "rev".to_string()],
        "both rows stay: the card stopped queueing, it did not forget anything"
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
    assert_eq!(
        db.prompt_keys(two_sided).expect("keys"),
        vec!["fwd".to_string(), "rev".to_string()],
        "two sides again, and no second row for either direction"
    );
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
    assert!(
        matches!(&error, StoreError::DuplicateName { what: "deck under that parent", name } if name == "JAPANESE"),
        "{error:?}"
    );

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
        db.due_prompts(NOON, 0, 1, None).expect("due").len(),
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
    assert!(db.due_prompts(NOON, 0, 1, None).expect("due").is_empty());
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
    assert_eq!(db.due_prompts(NOON, 0, 1, None).expect("due").len(), 1);
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
        .new_prompts(NOON, 0, 1, None)
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
        .due_prompts(NOON, 0, 1, None)
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
    store.create_deck(None, "Keep").expect("sent");
    store.create_deck(None, "Gone").expect("sent");
    store.create_deck(Some(2), "Under gone").expect("sent");
    store.create_card(1, "front\n---\nback").expect("sent");
    store.create_card(1, "a note with one side").expect("sent");
    store.create_card(1, "front\n---\nback").expect("sent");
    store.create_card(3, "front\n---\nback").expect("sent");
    store.create_card(2, "front\n---\nback").expect("sent");
    store.create_card(1, "front\n---\nback").expect("sent");
    wait_for(&store, 9);
    let trashed = store.snapshot().cards[2].id;
    let archived = store.snapshot().cards[5].id;
    store.trash_card(trashed).expect("sent");
    store.archive_card(archived, true).expect("sent");
    store.archive_deck(2, true).expect("sent");
    wait_for(&store, 3);

    let snapshot = store.snapshot();
    let from_snapshot: Vec<Id> = snapshot
        .cards_matching(&ViewSpec {
            mode: CardMode::Active,
            ..ViewSpec::default()
        })
        .into_iter()
        .filter(|card| card.sides >= 2)
        .map(|card| card.id)
        .collect();
    let mut expected = from_snapshot.clone();
    expected.sort_unstable();

    store.new_prompts(0, 0, 7, None).expect("sent");
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

#[test]
fn a_queue_can_be_scoped_to_one_deck() {
    let dir = TempDir::new().expect("tempdir");
    let mut db = open_temp(&dir);
    let verbs = deck(&mut db, "Verbs");
    let nouns = deck(&mut db, "Nouns");
    let in_verbs = card(&mut db, verbs, " hablar\n---\n to speak");
    let in_nouns = card(&mut db, nouns, " perro\n---\n dog");
    schedule(&mut db, in_verbs, NOON - HOUR);
    schedule(&mut db, in_nouns, NOON - HOUR);

    let cards = |prompts: Vec<super::super::DuePrompt>| -> Vec<Id> {
        prompts.into_iter().map(|prompt| prompt.card).collect()
    };
    assert_eq!(
        cards(db.due_prompts(NOON, 0, 1, Some(verbs)).expect("due")),
        vec![in_verbs],
        "the deck's own cards are offered"
    );
    assert_eq!(
        cards(db.due_prompts(NOON, 0, 1, None).expect("due")),
        vec![in_verbs, in_nouns],
        "and unscoped is every deck"
    );
}

#[test]
fn a_search_matches_a_card_by_its_tag() {
    let dir = TempDir::new().expect("tempdir");
    let mut db = open_temp(&dir);
    let subject = deck(&mut db, "Spanish");
    let tagged = card(&mut db, subject, " subjunctive\n---\n mood");
    db.apply(
        &Command::SetTags {
            card: tagged,
            tags: vec!["Grammar".to_string()],
        },
        NOON,
    )
    .expect("tags");

    assert_eq!(
        matching(&db, "grammar", None, CardMode::Active),
        vec![tagged],
        "a tag is part of what a card is searched by"
    );
}

#[test]
fn a_deck_scope_covers_the_decks_under_it() {
    let dir = TempDir::new().expect("tempdir");
    let mut db = open_temp(&dir);
    let verbs = deck(&mut db, "Verbs");
    let irregular = match db
        .apply(
            &Command::CreateDeck {
                parent: Some(verbs),
                name: "Irregular".to_string(),
            },
            0,
        )
        .expect("child deck")
    {
        Outcome::Deck(id) => id,
        other => panic!("expected a deck, got {other:?}"),
    };
    let nouns = deck(&mut db, "Nouns");
    let under = card(&mut db, irregular, " ir\n---\n to go");
    let beside = card(&mut db, nouns, " perro\n---\n dog");
    schedule(&mut db, under, NOON - HOUR);
    schedule(&mut db, beside, NOON - HOUR);

    let cards = |prompts: Vec<super::super::DuePrompt>| -> Vec<Id> {
        prompts.into_iter().map(|prompt| prompt.card).collect()
    };
    assert_eq!(
        cards(db.due_prompts(NOON, 0, 1, Some(verbs)).expect("due")),
        vec![under],
        "a card in a deck under the scope is in the scope"
    );
}

#[test]
fn a_new_card_queue_can_be_scoped_to_one_deck() {
    let dir = TempDir::new().expect("tempdir");
    let mut db = open_temp(&dir);
    let verbs = deck(&mut db, "Verbs");
    let nouns = deck(&mut db, "Nouns");
    let mine = card(&mut db, verbs, " hablar\n---\n to speak");
    let theirs = card(&mut db, nouns, " perro\n---\n dog");

    let cards = |prompts: Vec<super::super::NewPrompt>| -> Vec<Id> {
        prompts.into_iter().map(|prompt| prompt.card).collect()
    };
    assert_eq!(
        cards(db.new_prompts(NOON, 0, 1, Some(verbs)).expect("new")),
        vec![mine],
        "only the scoped deck's new card is offered"
    );
    let mut all = cards(db.new_prompts(NOON, 0, 1, None).expect("new"));
    all.sort();
    assert_eq!(
        all,
        vec![mine.min(theirs), mine.max(theirs)],
        "and unscoped is every deck"
    );
}

#[test]
fn a_card_with_one_due_prompt_snapshots_its_due_date() {
    let dir = TempDir::new().expect("tempdir");
    let mut db = open_temp(&dir);
    let grammar = deck(&mut db, "Grammar");
    let under = card(&mut db, grammar, " front\n---\n back");
    schedule(&mut db, under, NOON + HOUR);

    let snapshot = db.snapshot().expect("snapshot");
    let summary = snapshot
        .cards
        .iter()
        .find(|candidate| candidate.id == under)
        .expect("the card is in the snapshot");
    assert_eq!(summary.prompts, 1, "one studyable prompt is counted");
    assert_eq!(summary.unscheduled, 0, "it has been scheduled");
    assert_eq!(
        summary.due_at,
        Some(NOON + HOUR),
        "its due time comes through"
    );
}

#[test]
fn a_note_snapshots_no_prompts() {
    let dir = TempDir::new().expect("tempdir");
    let mut db = open_temp(&dir);
    let grammar = deck(&mut db, "Grammar");
    let note = card(&mut db, grammar, " just one side");

    let snapshot = db.snapshot().expect("snapshot");
    let summary = snapshot
        .cards
        .iter()
        .find(|candidate| candidate.id == note)
        .expect("the card is in the snapshot");
    assert_eq!(summary.prompts, 0, "a note carries no prompt");
    assert_eq!(summary.unscheduled, 0);
    assert_eq!(summary.due_at, None, "and so has no due time");
}

#[test]
fn a_diagram_card_holds_the_masks_it_was_given() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Anatomy");
    let id = match db
        .apply(
            &Command::CreateDiagram {
                deck,
                image: "plate.png".to_string(),
                caption: "The heart".to_string(),
            },
            0,
        )
        .expect("diagram")
    {
        Outcome::Card(id) => id,
        other => panic!("creating a diagram: {other:?}"),
    };

    let first = MaskRect::new(0.1, 0.2, 0.3, 0.4);
    let second = MaskRect::new(0.5, 0.1, 0.2, 0.2);
    db.apply(
        &Command::AddMask {
            card: id,
            group: 1,
            rect: first,
        },
        0,
    )
    .expect("first mask");
    db.apply(
        &Command::AddMask {
            card: id,
            group: 2,
            rect: second,
        },
        0,
    )
    .expect("second mask");

    assert_eq!(db.mask_groups(id).expect("groups"), vec![1, 2]);
    assert_eq!(
        db.masks(id).expect("masks"),
        vec![
            MaskRow {
                group: 1,
                rect: first
            },
            MaskRow {
                group: 2,
                rect: second
            },
        ],
        "the corners come back as they went in, ordered by group"
    );

    let picture: (String, String) = db
        .connection_for_test()
        .query_row("SELECT kind, image FROM card WHERE id = ?1", [id], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .expect("the card row");
    assert_eq!(picture, ("diagram".to_string(), "plate.png".to_string()));
}

#[test]
fn merging_two_rectangles_leaves_one_group() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Anatomy");
    let id = match db
        .apply(
            &Command::CreateDiagram {
                deck,
                image: "plate.png".to_string(),
                caption: String::new(),
            },
            0,
        )
        .expect("diagram")
    {
        Outcome::Card(id) => id,
        other => panic!("creating a diagram: {other:?}"),
    };

    let left = MaskRect::new(0.1, 0.1, 0.2, 0.2);
    let right = MaskRect::new(0.6, 0.1, 0.2, 0.2);
    db.apply(
        &Command::AddMask {
            card: id,
            group: 1,
            rect: left,
        },
        0,
    )
    .expect("left");
    db.apply(
        &Command::AddMask {
            card: id,
            group: 2,
            rect: right,
        },
        0,
    )
    .expect("right");
    assert_eq!(db.mask_groups(id).expect("groups"), vec![1, 2]);

    db.apply(
        &Command::MergeMasks {
            card: id,
            rects: vec![left, right],
            group: 1,
        },
        0,
    )
    .expect("merge");

    assert_eq!(
        db.mask_groups(id).expect("groups"),
        vec![1],
        "two rectangles can be one answer"
    );
    assert_eq!(db.masks(id).expect("masks").len(), 2, "both survived");

    db.apply(
        &Command::RemoveMask {
            card: id,
            rect: left,
        },
        0,
    )
    .expect("remove");
    assert_eq!(db.masks(id).expect("masks").len(), 1);

    // A rectangle has to sit inside the picture, or its cover would land on nothing.
    let outside = db.apply(
        &Command::AddMask {
            card: id,
            group: 1,
            rect: MaskRect::new(0.9, 0.9, 0.3, 0.3),
        },
        0,
    );
    assert!(
        matches!(outside, Err(StoreError::MaskOutsideImage { card }) if card == id),
        "{outside:?}"
    );
}

#[test]
fn a_diagram_card_gets_one_prompt_per_group() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Anatomy");
    let id = match db
        .apply(
            &Command::CreateDiagram {
                deck,
                image: "plate.png".to_string(),
                caption: String::new(),
            },
            0,
        )
        .expect("diagram")
    {
        Outcome::Card(id) => id,
        other => panic!("creating a diagram: {other:?}"),
    };
    assert!(
        db.prompt_keys(id).expect("keys").is_empty(),
        "a diagram without rectangles asks nothing"
    );

    let left = MaskRect::new(0.1, 0.1, 0.2, 0.2);
    let right = MaskRect::new(0.6, 0.1, 0.2, 0.2);
    db.apply(
        &Command::AddMask {
            card: id,
            group: 1,
            rect: left,
        },
        0,
    )
    .expect("group one");
    db.apply(
        &Command::AddMask {
            card: id,
            group: 2,
            rect: right,
        },
        0,
    )
    .expect("group two");
    assert_eq!(
        db.prompt_keys(id).expect("keys"),
        vec!["mask:1".to_string(), "mask:2".to_string()],
        "one prompt per group, and no text prompt because the card holds no text"
    );

    db.apply(
        &Command::RemoveMask {
            card: id,
            rect: right,
        },
        0,
    )
    .expect("remove");
    let retired: i64 = db
        .connection_for_test()
        .query_row(
            "SELECT retired FROM prompt WHERE card_id = ?1 AND key = 'mask:2'",
            [id],
            |row| row.get(0),
        )
        .expect("the retired row");
    assert_eq!(retired, 1, "a group with no rectangles is retired");
}

/// Removing every rectangle of a group retires its prompt but keeps what was answered on it: the
/// review history is a record of what happened, not a cache of the card as it stands.
#[test]
fn removing_a_group_leaves_its_records() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Anatomy");
    let id = match db
        .apply(
            &Command::CreateDiagram {
                deck,
                image: "plate.png".to_string(),
                caption: "The plate".to_string(),
            },
            0,
        )
        .expect("diagram")
    {
        Outcome::Card(id) => id,
        other => panic!("creating a diagram: {other:?}"),
    };
    let rect = MaskRect::new(0.1, 0.1, 0.2, 0.2);
    db.apply(
        &Command::AddMask {
            card: id,
            group: 1,
            rect,
        },
        0,
    )
    .expect("the rectangle");
    db.apply(
        &Command::Grade {
            card: id,
            key: "mask:1".to_string(),
            grade: Grade::Remembered,
            duration_ms: Some(1200),
        },
        0,
    )
    .expect("answering the diagram");

    db.apply(&Command::RemoveMask { card: id, rect }, 0)
        .expect("remove the only rectangle");

    let studyable: i64 = db
        .connection_for_test()
        .query_row(
            "SELECT count(*) FROM studyable_prompt WHERE card_id = ?1",
            [id],
            |row| row.get(0),
        )
        .expect("the studyable prompts");
    assert_eq!(
        studyable, 0,
        "the group has no rectangles left, so it is not asked"
    );
    let reviews: i64 = db
        .connection_for_test()
        .query_row(
            "SELECT count(*) FROM review_log WHERE card_id = ?1 AND key = 'mask:1'",
            [id],
            |row| row.get(0),
        )
        .expect("the reviews");
    assert_eq!(
        reviews, 1,
        "what was answered on the group is still on disk"
    );
    let retired: i64 = db
        .connection_for_test()
        .query_row(
            "SELECT retired FROM prompt WHERE card_id = ?1 AND key = 'mask:1'",
            [id],
            |row| row.get(0),
        )
        .expect("the retired prompt");
    assert_eq!(retired, 1, "the prompt is retired rather than deleted");
}

#[test]
fn a_card_that_holds_text_cannot_become_a_diagram() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Anatomy");
    let text = match db
        .apply(
            &Command::CreateCard {
                deck,
                markdown: "front\n---\nback".to_string(),
            },
            0,
        )
        .expect("text card")
    {
        Outcome::Card(id) => id,
        other => panic!("creating a card: {other:?}"),
    };

    let refused = db.apply(
        &Command::SetDiagram {
            card: text,
            image: "plate.png".to_string(),
            caption: String::new(),
        },
        0,
    );
    assert!(
        matches!(refused, Err(StoreError::NotEmpty { card, holds: "text" }) if card == text),
        "text on the card is kept, not hidden: {refused:?}"
    );
    let kind: String = db
        .connection_for_test()
        .query_row("SELECT kind FROM card WHERE id = ?1", [text], |row| {
            row.get(0)
        })
        .expect("the kind");
    assert_eq!(kind, "text", "a refused switch leaves the card as it was");

    // An empty card switches and takes a prompt per group.
    let empty = match db
        .apply(
            &Command::CreateCard {
                deck,
                markdown: String::new(),
            },
            0,
        )
        .expect("empty card")
    {
        Outcome::Card(id) => id,
        other => panic!("creating a card: {other:?}"),
    };
    db.apply(
        &Command::SetDiagram {
            card: empty,
            image: "plate.png".to_string(),
            caption: "A plate".to_string(),
        },
        0,
    )
    .expect("an empty card becomes a diagram");
    db.apply(
        &Command::AddMask {
            card: empty,
            group: 1,
            rect: MaskRect::new(0.1, 0.1, 0.2, 0.2),
        },
        0,
    )
    .expect("a rectangle");
    assert_eq!(
        db.prompt_keys(empty).expect("keys"),
        vec!["mask:1".to_string()],
        "the switch left the card asking its own group"
    );
}

#[test]
fn a_diagram_is_not_overwritten_by_text() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Anatomy");
    let id = match db
        .apply(
            &Command::CreateDiagram {
                deck,
                image: "plate.png".to_string(),
                caption: String::new(),
            },
            0,
        )
        .expect("diagram")
    {
        Outcome::Card(id) => id,
        other => panic!("creating a diagram: {other:?}"),
    };

    let refused = db.apply(
        &Command::SaveCard {
            id,
            markdown: "front\n---\nback".to_string(),
        },
        0,
    );
    assert!(
        matches!(refused, Err(StoreError::NotEmpty { card, holds: "a picture" }) if card == id),
        "text written over a diagram would never be asked: {refused:?}"
    );
    let markdown: String = db
        .connection_for_test()
        .query_row("SELECT markdown FROM card WHERE id = ?1", [id], |row| {
            row.get(0)
        })
        .expect("the markdown");
    assert_eq!(markdown, "", "the diagram still holds no text");
}

/// One card as the library lists it.
fn summary(db: &Db, id: Id) -> CardSummary {
    db.snapshot()
        .expect("snapshot")
        .cards
        .into_iter()
        .find(|card| card.id == id)
        .expect("the card is in the snapshot")
}

/// Set a deck's template, or clear it with `None`.
fn deck_template(db: &mut Db, deck: Id, template: Option<&str>) {
    db.apply(
        &Command::SetDeckTemplate {
            deck,
            template: template.map(str::to_string),
        },
        0,
    )
    .expect("deck template");
}

/// Set a card's own text template and name template; `None` clears either.
fn card_templates(db: &mut Db, card: Id, template: Option<&str>, name: Option<&str>) {
    db.apply(
        &Command::SetCardTemplate {
            card,
            template: template.map(str::to_string),
            name: name.map(str::to_string),
        },
        0,
    )
    .expect("card templates");
}

/// Set a card's own template, or clear it with `None`.
fn card_template(db: &mut Db, card: Id, template: Option<&str>) {
    card_templates(db, card, template, None);
}

/// Replace a card's fields.
fn fields(db: &mut Db, card: Id, pairs: &[(&str, &str)]) {
    let pairs: Vec<(String, String)> = pairs
        .iter()
        .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
        .collect();
    db.apply(
        &Command::SetFields {
            card,
            fields: pairs,
        },
        0,
    )
    .expect("fields");
}

#[test]
fn a_card_reports_the_template_it_inherits() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Spanish");
    let inherited = "<<Word>>\n---\n<<Meaning>>";
    deck_template(&mut db, deck, Some(inherited));

    let id = card(&mut db, deck, "");
    fields(&mut db, id, &[("Word", "gato"), ("Meaning", "cat")]);

    let listed = summary(&db, id);
    assert_eq!(listed.template.as_deref(), Some(inherited));
    assert_eq!(listed.template_from, Some(deck), "it comes from the deck");
    assert_eq!(
        listed.fields,
        vec![
            ("Meaning".to_string(), "cat".to_string()),
            ("Word".to_string(), "gato".to_string()),
        ],
        "fields come back ordered by name"
    );

    let own = card(&mut db, deck, "");
    card_template(&mut db, own, Some("<<Word>>"));
    let listed = summary(&db, own);
    assert_eq!(listed.template.as_deref(), Some("<<Word>>"));
    assert_eq!(listed.template_from, None, "the card carries this one");

    // A deck of its own also wins over the one above it.
    let child = match db
        .apply(
            &Command::CreateDeck {
                parent: Some(deck),
                name: "Verbs".to_string(),
            },
            0,
        )
        .expect("child deck")
    {
        Outcome::Deck(id) => id,
        other => panic!("creating a deck: {other:?}"),
    };
    deck_template(&mut db, child, Some("<<Word>>\n---\n<<Tense>>"));
    let nested = card(&mut db, child, "");
    let listed = summary(&db, nested);
    assert_eq!(listed.template_from, Some(child), "the nearest deck wins");
}

#[test]
fn the_stored_text_is_what_the_template_renders() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Spanish");
    let id = card(&mut db, deck, "");
    card_template(
        &mut db,
        id,
        Some("<<Word>>\n---\n<<Meaning>>\n\n{{#Example}}e.g. <<Example>>{{/Example}}"),
    );

    fields(&mut db, id, &[("Word", "gato"), ("Meaning", "cat")]);
    let listed = summary(&db, id);
    assert_eq!(listed.markdown, "gato\n---\ncat\n\n");
    assert_eq!(listed.sides, 2, "one separator makes the two sides");
    assert_eq!(
        db.prompt_keys(id).expect("keys"),
        vec!["fwd".to_string(), "rev".to_string()],
        "a rendered card is a card like any other"
    );

    fields(
        &mut db,
        id,
        &[
            ("Word", "gato"),
            ("Meaning", "cat"),
            ("Example", "el gato duerme"),
        ],
    );
    let listed = summary(&db, id);
    assert_eq!(listed.markdown, "gato\n---\ncat\n\ne.g. el gato duerme");
    assert!(
        listed.search_text.contains("duerme"),
        "search reaches what the card now says: {}",
        listed.search_text
    );
}

#[test]
fn a_deck_template_change_rewrites_its_cards() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Spanish");
    let first = card(&mut db, deck, "written by hand");
    let second = card(&mut db, deck, "also by hand");
    let own = card(&mut db, deck, "keeps its own");
    card_template(&mut db, own, Some("<<Word>>"));
    for id in [first, second, own] {
        fields(&mut db, id, &[("Word", "gato"), ("Meaning", "cat")]);
    }

    deck_template(&mut db, deck, Some("<<Word>>\n---\n<<Meaning>>"));

    for id in [first, second] {
        assert_eq!(
            summary(&db, id).markdown,
            "gato\n---\ncat",
            "card {id} follows the deck"
        );
        assert_eq!(summary(&db, id).sides, 2);
    }
    assert_eq!(
        summary(&db, own).markdown,
        "gato",
        "a card with its own template does not follow the deck's"
    );

    // Cleared: every card keeps the text it has, and the fields stay for the next template.
    deck_template(&mut db, deck, None);
    let listed = summary(&db, first);
    assert_eq!(listed.markdown, "gato\n---\ncat");
    assert_eq!(listed.template, None);
    assert_eq!(listed.fields.len(), 2, "the values are still there");
}

#[test]
fn a_deck_move_rerenders_against_the_new_template() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let plain = deck(&mut db, "Unsorted");
    let spanish = deck(&mut db, "Spanish");
    deck_template(&mut db, spanish, Some("<<Word>>\n---\n<<Meaning>>"));

    let id = card(&mut db, plain, "typed by hand");
    fields(&mut db, id, &[("Word", "gato"), ("Meaning", "cat")]);
    assert_eq!(
        summary(&db, id).markdown,
        "typed by hand",
        "no template above it, so its text is its own"
    );

    db.apply(&Command::MoveCard { id, deck: spanish }, 0)
        .expect("move");
    assert_eq!(summary(&db, id).markdown, "gato\n---\ncat");

    db.apply(&Command::MoveCard { id, deck: plain }, 0)
        .expect("move back");
    assert_eq!(
        summary(&db, id).markdown,
        "gato\n---\ncat",
        "leaving the deck leaves the text it was rendered to"
    );
}

#[test]
fn a_fifty_card_deck_renders_in_one_pass() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Five hundred");
    let ids: Vec<Id> = (0..50)
        .map(|index| {
            let id = card(&mut db, deck, "");
            fields(
                &mut db,
                id,
                &[("Word", &format!("word {index}")), ("Meaning", "meaning")],
            );
            id
        })
        .collect();

    deck_template(&mut db, deck, Some("<<Word>>\n---\n<<Meaning>>"));

    let snapshot = db.snapshot().expect("snapshot");
    for id in &ids {
        let listed = snapshot
            .cards
            .iter()
            .find(|card| card.id == *id)
            .expect("card");
        assert_eq!(listed.sides, 2, "card {id} renders two sides");
        assert!(
            listed.markdown.starts_with("word "),
            "card {id} has its own value: {}",
            listed.markdown
        );
        assert_eq!(listed.prompts, 1, "card {id} holds exactly one fwd prompt");
    }
}

#[test]
fn emptying_a_field_can_make_a_card_a_note() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Spanish");
    deck_template(&mut db, deck, Some("<<Word>>\n---\n<<Answer>>"));
    let id = card(&mut db, deck, "");
    fields(&mut db, id, &[("Word", "gato"), ("Answer", "cat")]);
    assert_eq!(summary(&db, id).prompts, 1);

    fields(&mut db, id, &[("Word", "gato")]);

    let listed = summary(&db, id);
    assert_eq!(
        listed.markdown, "gato\n---\n",
        "the placeholder renders empty and the separator stays"
    );
    assert_eq!(listed.sides, 1);
    assert_eq!(
        db.prompt_keys(id).expect("keys"),
        vec!["fwd".to_string(), "rev".to_string()],
        "the prompt rows stay, the card just stops queueing"
    );
    assert_eq!(
        db.due_prompts(1_000, 0, 0, None).expect("due").len(),
        0,
        "a note does not queue"
    );
}

#[test]
fn search_reaches_a_field_the_template_does_not_use() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Spanish");
    deck_template(&mut db, deck, Some("<<Word>>"));
    let id = card(&mut db, deck, "");
    fields(&mut db, id, &[("Word", "gato"), ("Note", "duerme de día")]);

    let found: Vec<Id> = db
        .snapshot()
        .expect("snapshot")
        .cards
        .iter()
        .filter(|card| card.search_text.contains("duerme"))
        .map(|card| card.id)
        .collect();
    assert_eq!(found, vec![id], "a value search reaches the value alone");
}

#[test]
fn clearing_a_template_leaves_the_cards_readable() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Spanish");
    let inherited = card(&mut db, deck, "");
    let own = card(&mut db, deck, "");
    fields(&mut db, inherited, &[("Word", "gato")]);
    fields(&mut db, own, &[("Word", "perro")]);
    card_template(&mut db, own, Some("<<Word>>!\n---\nA dog"));

    deck_template(&mut db, deck, Some("<<Word>>\n---\nAn animal"));
    assert_eq!(summary(&db, inherited).markdown, "gato\n---\nAn animal");
    assert_eq!(summary(&db, own).markdown, "perro!\n---\nA dog");

    // The deck's template goes: both cards keep the text they already say, and their prompts with
    // it, because there is nothing left to rewrite them with.
    deck_template(&mut db, deck, None);
    assert_eq!(summary(&db, inherited).template, None);
    assert_eq!(
        summary(&db, own).template.as_deref(),
        Some("<<Word>>!\n---\nA dog"),
        "the card's own template is not the deck's to clear"
    );
    for id in [inherited, own] {
        let listed = summary(&db, id);
        assert_eq!(listed.sides, 2, "card {id} still has its two sides");
        assert_eq!(listed.prompts, 1, "card {id} still queues");
        assert_eq!(listed.fields.len(), 1, "card {id} keeps its values");
    }
    assert_eq!(summary(&db, inherited).markdown, "gato\n---\nAn animal");
    assert_eq!(summary(&db, own).markdown, "perro!\n---\nA dog");

    // The card's own template goes the same way.
    card_template(&mut db, own, None);
    assert_eq!(summary(&db, own).markdown, "perro!\n---\nA dog");
    assert_eq!(summary(&db, own).sides, 2);
}

#[test]
fn renaming_a_deck_rerenders_its_cards() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Spanish");
    deck_template(
        &mut db,
        deck,
        Some("<<Word>>\n---\n<<Meaning>> (from <<Deck>>)"),
    );
    let id = card(&mut db, deck, "hand typed");
    fields(&mut db, id, &[("Word", "gato"), ("Meaning", "cat")]);

    db.apply(
        &Command::RenameDeck {
            id: deck,
            name: "Español".to_string(),
        },
        0,
    )
    .expect("rename");

    assert_eq!(
        summary(&db, id).markdown,
        "gato\n---\ncat (from Español)",
        "the deck name a card renders with is its own, so a rename rewrites its text"
    );
    assert!(
        summary(&db, id).search_text.contains("espanol"),
        "search follows the text that changed"
    );
}

#[test]
fn a_templated_card_cannot_take_a_raw_markdown_edit() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Spanish");
    deck_template(&mut db, deck, Some("<<Word>>\n---\n<<Meaning>>"));
    let id = card(&mut db, deck, "hand typed");
    fields(&mut db, id, &[("Word", "gato"), ("Meaning", "cat")]);
    let rendered = summary(&db, id).markdown;

    let refused = db.apply(
        &Command::SaveCard {
            id,
            markdown: "something else\n---\nentirely".to_string(),
        },
        0,
    );

    assert!(
        matches!(
            refused,
            Err(StoreError::NotEmpty {
                card,
                holds: "a template"
            }) if card == id
        ),
        "a hand-written text would be overwritten by the next field edit: {refused:?}"
    );
    assert_eq!(summary(&db, id).markdown, rendered, "the text is untouched");
}

#[test]
fn a_diagram_in_a_templated_deck_keeps_its_own_text() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Anatomy");
    let id = match db
        .apply(
            &Command::CreateDiagram {
                deck,
                image: "plate.png".to_string(),
                caption: "The heart".to_string(),
            },
            0,
        )
        .expect("diagram")
    {
        Outcome::Card(id) => id,
        other => panic!("creating a diagram: {other:?}"),
    };
    db.apply(
        &Command::AddMask {
            card: id,
            group: 1,
            rect: MaskRect::new(0.1, 0.1, 0.2, 0.2),
        },
        0,
    )
    .expect("mask");

    // The template reaches every card under the deck, and a diagram is asked from its picture.
    deck_template(&mut db, deck, Some("<<Word>>\n---\n<<Meaning>>"));

    assert_eq!(
        db.prompt_keys(id).expect("keys"),
        vec!["mask:1".to_string()],
        "no text prompt is queued by a template the card never shows"
    );
    assert_eq!(
        summary(&db, id).kind,
        CardKind::Diagram,
        "the card is still a diagram"
    );
}

#[test]
fn a_card_created_in_a_templated_deck_is_rendered() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Spanish");
    deck_template(&mut db, deck, Some("<<Word>>\n---\n<<Meaning>>"));

    let id = card(&mut db, deck, "typed while the deck already had a template");

    assert_eq!(
        summary(&db, id).markdown,
        "\n---\n",
        "the fields are empty, so the template renders empty"
    );
    assert_eq!(
        summary(&db, id).sides,
        0,
        "with no fields filled the card has nothing to say yet, so it is a note"
    );
}

#[test]
fn a_stored_name_follows_an_edit() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("typr.sqlite3");
    let mut db = Db::open(&path).expect("open");
    let deck = deck(&mut db, "Biology");
    let id = card(&mut db, deck, "first\n---\nback");
    assert_eq!(summary(&db, id).name, "first");

    db.apply(
        &Command::SaveCard {
            id,
            markdown: "third\n---\nfourth".to_string(),
        },
        0,
    )
    .expect("save");
    assert_eq!(
        summary(&db, id).name,
        "third",
        "the name is what the card says now, not what it said when it was made"
    );

    // The backfill runs for a library arriving at the name column, so reopening this one must
    // leave the stored name alone rather than derive it again from whatever the text holds.
    drop(db);
    let db = Db::open(&path).expect("reopen");
    assert_eq!(summary(&db, id).name, "third");
}

#[test]
fn a_name_template_names_the_card_over_its_text() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Spanish");
    let id = card(&mut db, deck, "");
    card_templates(
        &mut db,
        id,
        Some("<<Word>>\n---\n<<Meaning>>"),
        Some("<<Word>> (<<Kind>>)"),
    );
    fields(
        &mut db,
        id,
        &[
            ("Word", "hablar"),
            ("Kind", "verb"),
            ("Meaning", "to speak"),
        ],
    );

    assert_eq!(
        summary(&db, id).name,
        "hablar (verb)",
        "a name template reads the fields like any other template, and the name is not the text"
    );
    assert_eq!(
        summary(&db, id).markdown,
        "hablar\n---\nto speak",
        "the text template still renders the text"
    );
}

#[test]
fn a_tag_path_is_one_tag() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Spanish");
    let id = card(&mut db, deck, "hablar\n---\nto speak");
    db.apply(
        &Command::SetTags {
            card: id,
            tags: vec!["spanish/verbs".to_string()],
        },
        0,
    )
    .expect("tags");

    assert_eq!(
        db.tag_names().expect("names"),
        vec!["spanish/verbs".to_string()],
        "a path is one tag row, not one row per segment"
    );
    assert_eq!(summary(&db, id).tags, vec!["spanish/verbs".to_string()]);

    let tree = db.snapshot().expect("snapshot").tag_tree();
    assert_eq!(tree.len(), 1, "one root, and its path is the first segment");
    assert_eq!(tree[0].path, "spanish");
    assert_eq!(tree[0].cards, 1, "the parent counts the cards below it");
    assert_eq!(tree[0].children.len(), 1);
    assert_eq!(tree[0].children[0].path, "spanish/verbs");
    assert_eq!(tree[0].children[0].cards, 1);
}

#[test]
fn a_saved_card_records_its_references() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Biology");
    let target = card(&mut db, deck, "Krebs cycle\n---\nThe second stage");
    let referrer = card(
        &mut db,
        deck,
        "The citric acid cycle\n---\nSee [[Krebs cycle]] for the steps",
    );

    assert_eq!(
        summary(&db, referrer).references,
        vec![CardRef {
            id: target,
            name: "Krebs cycle".to_string(),
        }],
        "the save records the card its text names"
    );
    let backlinks = db.backlinks(target).expect("backlinks");
    assert_eq!(backlinks.len(), 1);
    assert_eq!(backlinks[0].id, referrer, "and the target lists it back");
}

#[test]
fn a_backlink_disappears_when_the_target_is_renamed_away() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Biology");
    let target = card(&mut db, deck, "Krebs cycle\n---\nThe second stage");
    let referrer = card(&mut db, deck, "Notes\n---\nSee [[Krebs cycle]]");
    assert_eq!(db.backlinks(target).expect("backlinks").len(), 1);

    db.apply(
        &Command::SaveCard {
            id: target,
            markdown: "The citric acid cycle\n---\nThe second stage".to_string(),
        },
        0,
    )
    .expect("save");

    assert!(
        db.backlinks(target).expect("backlinks").is_empty(),
        "a name that moved leaves no backlink behind"
    );
    assert!(
        summary(&db, referrer).references.is_empty(),
        "nothing answers to the old name any more"
    );
    assert!(
        summary(&db, referrer).markdown.contains("[[Krebs cycle]]"),
        "the author's words stay, so the broken reference is still there to fix"
    );
}

#[test]
fn resolution_prefers_the_nearest_deck() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let near = deck(&mut db, "Biology");
    let far = deck(&mut db, "Advanced");
    let local = card(&mut db, near, "Introduction\n---\nThe local one");
    let distant = card(&mut db, far, "Introduction\n---\nThe distant one");

    let here = card(
        &mut db,
        far,
        "Lecture one\n---\nStart with [[Introduction]]",
    );
    assert_eq!(
        summary(&db, here).references[0].id,
        distant,
        "the referring card's own deck comes first"
    );
    let there = card(
        &mut db,
        near,
        "Lecture two\n---\nStart with [[Introduction]]",
    );
    assert_eq!(
        summary(&db, there).references[0].id,
        local,
        "and each deck's notes point at that deck's card"
    );

    let elsewhere = deck(&mut db, "Chemistry");
    let other = card(
        &mut db,
        elsewhere,
        "Reading\n---\nStart with [[Introduction]]",
    );
    assert_eq!(
        summary(&db, other).references[0].id,
        local,
        "outside either deck the oldest card answers, so one text always opens one card"
    );
}

/// Every prompt row a card schedules, read back in the order its keys are named.
fn prompt_rows(db: &Db, card: Id) -> Vec<(String, Option<Folded>)> {
    db.prompt_keys(card)
        .expect("prompt keys")
        .into_iter()
        .map(|key| {
            let row = db.prompt(card, &key).expect("prompt row");
            (key, row)
        })
        .collect()
}

/// The side count the list shows for a card.
fn sides_of(db: &Db, card: Id) -> u32 {
    db.snapshot()
        .expect("snapshot")
        .cards
        .into_iter()
        .find(|summary| summary.id == card)
        .expect("the card is in the snapshot")
        .sides
}

#[test]
fn adding_an_embed_moves_no_prompt_or_due_date() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Biology");
    // The target's body carries a side separator. If an embed were expanded before sides were
    // counted, that separator would become a side of the card naming it; this card proves it does
    // not.
    let target = card(
        &mut db,
        deck,
        "Mitosis\n\nMitosis is how one cell becomes two.\n---\nSecond line.",
    );
    assert_eq!(name_of(&db, target), "Mitosis");
    let card = card(&mut db, deck, "front\n---\nback");

    // Grade first, so every row has a due date that a rebuild would move if the text changed shape.
    let keys = db.prompt_keys(card).expect("prompt keys");
    for (offset, key) in keys.iter().enumerate() {
        let at = 1_700_000_000_000 + i64::try_from(offset).expect("offset fits") * 60_000;
        db.apply(
            &Command::Grade {
                card,
                key: key.clone(),
                grade: Grade::Remembered,
                duration_ms: None,
            },
            at,
        )
        .expect("grade");
    }
    let before = prompt_rows(&db, card);
    assert!(
        before
            .iter()
            .any(|(_, row)| row.is_some_and(|row| row.due_at.is_some())),
        "no graded row carries a due date, so this test would prove nothing: {before:?}"
    );
    let sides = sides_of(&db, card);
    assert_eq!(sides, 2, "the card under test should start with two sides");

    db.apply(
        &Command::SaveCard {
            id: card,
            markdown: "front\n\n![[Mitosis]]\n---\nback".to_string(),
        },
        1_700_000_100_000,
    )
    .expect("save the card with an embed");

    assert_eq!(
        db.prompt_keys(card).expect("prompt keys"),
        keys,
        "an embed added or removed a prompt"
    );
    assert_eq!(prompt_rows(&db, card), before, "an embed moved a schedule");
    assert_eq!(
        sides_of(&db, card),
        sides,
        "an embed changed the card's side count"
    );
}
