//! Saved views and pins, as the store holds them.

use super::*;
use rusqlite::params;

fn spec() -> ViewSpec {
    ViewSpec {
        deck: None,
        query: "gato".to_string(),
        tags: vec!["exam".to_string(), "biology".to_string()],
        mode: CardMode::Active,
        pinned_only: true,
        sort: Sort::DueDate,
        dir: SortDir::Descending,
    }
}

#[test]
fn a_view_round_trips_through_the_store() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Spanish");

    let id = match db
        .apply(
            &Command::CreateView {
                spec: ViewSpec {
                    deck: Some(deck),
                    ..spec()
                },
                name: "Exam week".to_string(),
            },
            NOON,
        )
        .expect("create view")
    {
        Outcome::View(id) => id,
        other => panic!("creating a view: {other:?}"),
    };

    let stored = |db: &Db| db.snapshot().expect("snapshot").views;
    let views: Vec<SavedView> = stored(&db);
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].id, id);
    assert_eq!(views[0].name, "Exam week");
    assert_eq!(
        views[0].spec,
        ViewSpec {
            deck: Some(deck),
            ..spec()
        },
        "every part of the filter came back, including the order"
    );

    let other = ViewSpec {
        query: "cancion".to_string(),
        tags: Vec::new(),
        pinned_only: false,
        sort: Sort::FirstSide,
        dir: SortDir::Ascending,
        ..ViewSpec::default()
    };
    let updated = db
        .apply(
            &Command::UpdateView {
                id,
                spec: other.clone(),
            },
            NOON,
        )
        .expect("update");
    assert_eq!(updated, Outcome::Changed(1));
    assert_eq!(stored(&db)[0].spec, other, "an update keeps the name");

    let renamed = db
        .apply(
            &Command::RenameView {
                id,
                name: "  Verb drills  ".to_string(),
            },
            NOON,
        )
        .expect("rename");
    assert_eq!(renamed, Outcome::Changed(1));
    assert_eq!(
        stored(&db)[0].name,
        "Verb drills",
        "the name is trimmed on the way in"
    );

    let deleted = db.apply(&Command::DeleteView { id }, NOON).expect("delete");
    assert_eq!(deleted, Outcome::Changed(1));
    assert!(
        stored(&db).is_empty(),
        "deleting a view removes only the view"
    );
    assert_eq!(
        db.snapshot().expect("snapshot").cards.len(),
        0,
        "and no card of the library went with it"
    );
}

#[test]
fn two_views_cannot_share_a_name() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let first = |db: &mut Db, name: &str| {
        db.apply(
            &Command::CreateView {
                spec: spec(),
                name: name.to_string(),
            },
            NOON,
        )
    };
    first(&mut db, "Due this week").expect("first");

    let error = first(&mut db, "due this WEEK").expect_err("duplicate");
    assert!(
        matches!(&error, StoreError::DuplicateName { what: "view", name } if name == "due this WEEK"),
        "{error:?}"
    );
    assert_eq!(
        db.snapshot().expect("snapshot").views.len(),
        1,
        "the refused view left nothing behind"
    );
}

#[test]
fn a_pin_is_a_property_of_the_card() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Spanish");
    let id = card(&mut db, deck, "gato\n---\ncat");

    assert!(!db.snapshot().expect("snapshot").cards[0].pinned);
    let pinned = db
        .apply(
            &Command::SetPinned {
                card: id,
                pinned: true,
            },
            NOON,
        )
        .expect("pin");
    assert_eq!(pinned, Outcome::Changed(1));
    assert!(db.snapshot().expect("snapshot").cards[0].pinned);
}

#[test]
fn the_snapshot_carries_what_the_orders_rank_by() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Spanish");
    let first = card(&mut db, deck, "gato\n---\ncat");
    let second = card(&mut db, deck, "perro\n---\ndog");
    // A card remembers when it was written, not only when it was last touched.
    db.connection_for_test()
        .execute(
            "UPDATE card SET created_at = ?2 WHERE id = ?1",
            params![first, 1_000],
        )
        .expect("age one card");
    schedule(&mut db, second, NOON + HOUR);

    let snapshot = db.snapshot().expect("snapshot");
    let of = |id: Id| {
        snapshot
            .cards
            .iter()
            .find(|card| card.id == id)
            .expect("card")
    };
    assert_eq!(of(second).created_at, 0, "written at the clock's zero");
    assert_eq!(
        of(first).created_at,
        1_000,
        "created_at is the card's own, not its last edit"
    );
    assert_eq!(of(second).reviews, 1, "a review is counted for the order");
    assert_eq!(of(first).reviews, 0);
}

#[test]
fn a_view_whose_deck_is_gone_falls_back_to_every_card() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Spanish");
    card(&mut db, deck, "gato\n---\ncat");
    card(&mut db, deck, "perro\n---\ndog");

    // A scope no deck carries: what a view holds once the deck it was saved on is gone.
    db.apply(
        &Command::CreateView {
            spec: ViewSpec {
                deck: Some(404),
                ..ViewSpec::default()
            },
            name: "Spanish".to_string(),
        },
        NOON,
    )
    .expect("create view");

    let snapshot = db.snapshot().expect("snapshot");
    let view = &snapshot.views[0];
    assert!(
        view.scope_missing(&snapshot),
        "the view says its deck is gone"
    );
    assert_eq!(
        snapshot.cards_matching(&view.spec).len(),
        2,
        "a scope no deck carries shows the library, not an empty list"
    );
}

#[test]
fn a_pin_survives_the_trash() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Spanish");
    let id = card(&mut db, deck, "gato\n---\ncat");
    schedule(&mut db, id, NOON);
    db.apply(
        &Command::SetPinned {
            card: id,
            pinned: true,
        },
        NOON,
    )
    .expect("pin");
    let reviews: i64 = db
        .connection_for_test()
        .query_row(
            "SELECT COUNT(*) FROM review_log WHERE card_id = ?1",
            params![id],
            |row| row.get::<_, i64>(0),
        )
        .expect("count");

    db.apply(&Command::TrashCard { id }, NOON).expect("trash");
    let trashed = db.snapshot().expect("snapshot");
    assert!(
        trashed
            .cards
            .iter()
            .any(|card| card.id == id && card.pinned),
        "the pin is still on the trashed card"
    );

    db.apply(&Command::RestoreCard { id }, NOON)
        .expect("restore");
    let restored = db
        .snapshot()
        .expect("snapshot")
        .cards
        .into_iter()
        .find(|card| card.id == id)
        .expect("the card");
    assert!(restored.pinned, "and after the restore");
    assert_eq!(
        db.connection_for_test()
            .query_row(
                "SELECT COUNT(*) FROM review_log WHERE card_id = ?1",
                params![id],
                |row| row.get::<_, i64>(0),
            )
            .expect("count"),
        reviews,
        "the history is the same on both sides of the trash"
    );
}

#[test]
fn a_pinned_card_is_listed_without_a_deck() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let top = deck(&mut db, "Spanish");
    let nested = match db
        .apply(
            &Command::CreateDeck {
                parent: Some(top),
                name: "Verbs".to_string(),
            },
            NOON,
        )
        .expect("nested deck")
    {
        Outcome::Deck(id) => id,
        other => panic!("creating a deck: {other:?}"),
    };
    let neighbour = card(&mut db, top, "gato\n---\ncat");
    let buried = card(&mut db, nested, "hablar\n---\nto speak");
    db.apply(
        &Command::SetPinned {
            card: buried,
            pinned: true,
        },
        NOON,
    )
    .expect("pin");

    let spec = ViewSpec {
        pinned_only: true,
        ..ViewSpec::default()
    };
    let snapshot = db.snapshot().expect("snapshot");
    let listed: Vec<Id> = snapshot
        .cards_matching(&spec)
        .into_iter()
        .map(|card| card.id)
        .collect();
    assert_eq!(
        listed,
        vec![buried],
        "the pinned card in the nested deck is listed and its neighbour is not"
    );
    assert!(!listed.contains(&neighbour));
}

/// The ids a cram run would walk for `spec`.
fn cram_selection(db: &Db, spec: &ViewSpec) -> Vec<Id> {
    db.cards_for_view(spec)
        .expect("the cram selection")
        .into_iter()
        .map(|card| card.id)
        .collect()
}

#[test]
fn a_cram_selection_matches_the_list() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let spanish = deck(&mut db, "Spanish");
    let verbs = deck(&mut db, "Verbs");
    let perro = card(&mut db, spanish, "perro\n---\ndog");
    let gato = card(&mut db, spanish, "gato\n---\ncat");
    let correr = card(&mut db, verbs, "correr\n---\nto run");
    let sol = card(&mut db, spanish, "sol");
    db.apply(
        &Command::SetTags {
            card: gato,
            tags: vec!["exam".to_string()],
        },
        NOON,
    )
    .expect("tag");

    let specs = [
        ("everything", ViewSpec::default(), 3),
        (
            "one word",
            ViewSpec {
                query: "gato".to_string(),
                ..ViewSpec::default()
            },
            1,
        ),
        (
            "one tag",
            ViewSpec {
                tags: vec!["exam".to_string()],
                ..ViewSpec::default()
            },
            1,
        ),
        (
            "one deck",
            ViewSpec {
                deck: Some(verbs),
                ..ViewSpec::default()
            },
            1,
        ),
    ];
    for (what, spec, selected) in specs {
        let snapshot = db.snapshot().expect("snapshot");
        let listed: Vec<Id> = snapshot
            .cards_matching(&spec)
            .into_iter()
            .filter(|card| card.prompts > 0)
            .map(|card| card.id)
            .collect();
        assert_eq!(
            cram_selection(&db, &spec),
            listed,
            "the drill and the list agree on {what}"
        );
        assert_eq!(listed.len(), selected, "the {what} spec selects {selected}");
    }

    assert!(
        !cram_selection(&db, &ViewSpec::default()).contains(&sol),
        "a single-sided note is listed but has nothing to ask"
    );
    assert!(
        cram_selection(&db, &ViewSpec::default()).contains(&perro),
        "a plain two-sided card is asked"
    );

    db.apply(&Command::TrashCard { id: correr }, NOON)
        .expect("trash");
    let trash = ViewSpec {
        mode: CardMode::Trash,
        ..ViewSpec::default()
    };
    assert_eq!(
        db.snapshot()
            .expect("snapshot")
            .cards_matching(&trash)
            .len(),
        1,
        "the trash lists the card"
    );
    assert!(
        cram_selection(&db, &trash).is_empty(),
        "and nothing in the trash can be asked"
    );
}

#[test]
fn a_cram_selection_ignores_due_dates_and_the_cap() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    db.apply(
        &Command::SetSetting {
            key: SettingKey::NewPerDay,
            value: SettingValue::Number(0),
        },
        NOON,
    )
    .expect("today's allowance is spent");
    let biology = deck(&mut db, "Biology");
    let first = card(&mut db, biology, "front\n---\nback");
    let second = card(&mut db, biology, "other\n---\nthing");
    let next_week = NOON + 7 * 24 * HOUR;
    schedule(&mut db, first, next_week);
    schedule(&mut db, second, next_week);

    assert!(
        db.due_prompts(NOON, 0, 1, None)
            .expect("due queue")
            .is_empty(),
        "nothing is due"
    );
    assert!(
        db.new_prompts(NOON, 0, 1, None)
            .expect("new queue")
            .is_empty(),
        "and the cap admits nothing"
    );

    let mut crammed = cram_selection(&db, &ViewSpec::default());
    crammed.sort_unstable();
    assert_eq!(
        crammed,
        vec![first, second],
        "the filter selects both cards, whatever the schedule says"
    );
}

#[test]
fn filtering_a_parent_includes_its_children() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Spanish");
    let verbs = card(&mut db, deck, "hablar\n---\nto speak");
    let nouns = card(&mut db, deck, "gato\n---\ncat");
    let french = card(&mut db, deck, "chat\n---\ncat");
    for (id, tag) in [
        (verbs, "spanish/verbs"),
        (nouns, "spanish/nouns"),
        (french, "french"),
    ] {
        db.apply(
            &Command::SetTags {
                card: id,
                tags: vec![tag.to_string()],
            },
            0,
        )
        .expect("tags");
    }

    let snapshot = db.snapshot().expect("snapshot");
    let mut listed: Vec<Id> = snapshot
        .cards_matching(&ViewSpec {
            tags: vec!["spanish".to_string()],
            ..ViewSpec::default()
        })
        .into_iter()
        .map(|card| card.id)
        .collect();
    listed.sort_unstable();
    assert_eq!(
        listed,
        vec![verbs, nouns],
        "a filter on a parent lists the cards under its children"
    );
}

#[test]
fn a_deck_without_a_view_reads_as_grid() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Spanish");
    let snapshot = db.snapshot().expect("snapshot");
    let found = snapshot
        .decks
        .iter()
        .find(|each| each.id == deck)
        .expect("the deck is in the snapshot");
    assert_eq!(
        found.view,
        crate::library_view::ViewMode::Grid,
        "a deck that never chose a view is a grid, not unreadable"
    );
}

#[test]
fn a_cram_sessions_filter_can_be_kept_as_a_view_and_never_on_its_own() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = open_temp(&dir);
    let deck = deck(&mut db, "Biology");
    let _ = card(&mut db, deck, "front\n---\nback");
    db.apply(
        &Command::SetTags {
            card: 1,
            tags: vec!["exam".to_string()],
        },
        NOON,
    )
    .expect("tag");

    let filter = ViewSpec {
        tags: vec!["exam".to_string()],
        ..ViewSpec::default()
    };
    // The cram selection is the filter the drill would walk; nothing is saved yet.
    assert_eq!(cram_selection(&db, &filter), vec![1]);
    assert!(
        db.snapshot().expect("snapshot").views.is_empty(),
        "a view is not created on its own"
    );

    let id = match db
        .apply(
            &Command::CreateView {
                spec: filter.clone(),
                name: "Exam drill".to_string(),
            },
            NOON,
        )
        .expect("save the drill")
    {
        Outcome::View(id) => id,
        other => panic!("saving the drill: {other:?}"),
    };

    let views = db.snapshot().expect("snapshot").views;
    assert_eq!(views.len(), 1, "only the saved drill exists");
    assert_eq!(views[0].id, id);
    assert_eq!(views[0].name, "Exam drill");
    assert_eq!(views[0].spec, filter, "the filter is stored unchanged");
    assert_eq!(
        cram_selection(&db, &views[0].spec),
        vec![1],
        "it selects the same cards"
    );
}
