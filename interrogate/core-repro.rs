//! Repro harness for the audit: each test states what the intent requires, and fails when the
//! code does not do it.

use typr_core::store::{CardMode, CardSummary, Command, Db, Deck, Grade, Outcome, Phase, Snapshot};

fn deck(db: &Db, name: &str) -> i64 {
    match db
        .apply(
            &Command::CreateDeck {
                parent: None,
                name: name.to_string(),
            },
            0,
        )
        .expect("deck")
    {
        Outcome::Deck(id) => id,
        other => panic!("expected a deck, got {other:?}"),
    }
}

fn card(db: &Db, deck: i64, markdown: &str) -> i64 {
    match db
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
    }
}

fn grade(db: &Db, card: i64, grade: Grade, now_ms: i64) -> Outcome {
    db.apply(
        &Command::Grade {
            card,
            key: "fwd".to_string(),
            grade,
            duration_ms: None,
        },
        now_ms,
    )
    .expect("grade")
}

fn new_queue(db: &Db, now_ms: i64) -> Vec<i64> {
    match db
        .apply(
            &Command::NewPrompts {
                now_ms,
                offset_ms: 0,
                seed: 7,
            },
            now_ms,
        )
        .expect("new prompts")
    {
        Outcome::New(prompts) => prompts.into_iter().map(|prompt| prompt.card).collect(),
        other => panic!("expected new prompts, got {other:?}"),
    }
}

fn due_queue(db: &Db, now_ms: i64) -> Vec<i64> {
    match db
        .apply(
            &Command::DuePrompts {
                now_ms,
                offset_ms: 0,
                seed: 7,
            },
            now_ms,
        )
        .expect("due prompts")
    {
        Outcome::Due(prompts) => prompts.into_iter().map(|prompt| prompt.card).collect(),
        other => panic!("expected due prompts, got {other:?}"),
    }
}

/// review-scheduling: "A `learn`-phase `forgot` SHALL leave scheduling state unchanged", and
/// study-session: the new-card queue holds "the prompts of studyable cards that have no scheduling
/// state". A new card forgotten while learning therefore has to stay in the new-card queue.
#[test]
fn a_new_card_forgotten_while_learning_stays_in_the_new_queue() {
    let dir = tempfile::tempdir().expect("temp dir");
    let db = Db::open(&dir.path().join("typr.sqlite3")).expect("open");
    let deck = deck(&db, "Biology");
    let card = card(&db, deck, "front\n---\nback");

    assert_eq!(new_queue(&db, 1_000), vec![card], "the card starts out new");

    let outcome = grade(&db, card, Grade::Forgot, 1_000);
    let prompt = db.prompt(card, "fwd").expect("prompt").expect("row");
    assert_eq!(
        (prompt.phase, prompt.due_at),
        (Phase::Learn, None),
        "the forgotten learning cycle left the schedule alone"
    );
    println!("grade outcome: {outcome:?}");
    println!("prompt after: {prompt:?}");

    let new = new_queue(&db, 1_000);
    let due = due_queue(&db, 1_000);
    println!("new queue: {new:?}");
    println!("due queue: {due:?}");

    assert_eq!(
        new,
        vec![card],
        "the card is still new, so it has to be offered by the new-card queue"
    );
}

/// study-session: the new-card queue after a learn-phase failure, seen from a second session.
#[test]
fn a_forgotten_new_card_is_offered_again_in_a_later_session() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("typr.sqlite3");
    {
        let db = Db::open(&path).expect("open");
        let deck = deck(&db, "Biology");
        let card = card(&db, deck, "front\n---\nback");
        grade(&db, card, Grade::Forgot, 1_000);
    }
    // A restart: the same library, opened again.
    let db = Db::open(&path).expect("reopen");
    let new = new_queue(&db, 2_000);
    println!("new queue one restart later: {new:?}");
    assert_eq!(new.len(), 1, "the card is unstudied and must be offered");
}

/// card-library: the text search is over "card content and tags".
#[test]
fn the_search_matches_a_card_by_its_tag() {
    let tagged = CardSummary {
        id: 1,
        deck: 1,
        sides: 2,
        tags: vec!["Grammar".to_string()],
        archived: false,
        trashed_at: None,
        updated_at: 0,
        preview: "hello".to_string(),
        markdown: "hello\n---\nworld".to_string(),
        search_text: typr_core::store::folded("hello\n---\nworld"),
    };
    let snapshot = Snapshot {
        decks: vec![Deck {
            id: 1,
            parent: None,
            name: "Biology".to_string(),
            archived: false,
        }],
        cards: vec![tagged],
        ..Snapshot::default()
    };

    let by_content = snapshot.cards_matching("hello", None, None, CardMode::Active);
    let by_tag = snapshot.cards_matching("grammar", None, None, CardMode::Active);
    println!("content match: {}", by_content.len());
    println!("tag match: {}", by_tag.len());

    assert_eq!(by_content.len(), 1, "content search works");
    assert_eq!(by_tag.len(), 1, "a tag is part of what a card is searched by");
}
