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
