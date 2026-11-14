//! The binary's behaviour: what a frame, a key and a click do to the whole app.
pub(crate) use crate::*;
pub(crate) use rune::{Key, KeyEvent, Mods};
pub(crate) use typr::library::Host as LibraryHost;
pub(crate) use typr_core::clock::{Clock, LEARNING_STEP_MS};

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use std::time::Duration;

use typr_core::store::Db;

mod diagnostics;
mod editor_flow;
mod library_flow;
mod session_flow;

/// A clock that panics the first time the worker reads it, which is how a stoppage starts.
struct Stops(AtomicUsize);

impl Clock for Stops {
    fn now_ms(&self) -> i64 {
        if self.0.fetch_add(1, AtomicOrdering::SeqCst) == 0 {
            panic!("the clock fell over");
        }
        0
    }
}

/// One due card, revealed to its last side, forgotten, and acknowledged.
fn forgotten_due_card() -> State {
    let mut session = Session::new(
        Queue::Due,
        vec![Entry::sides(
            7,
            ["front".to_string(), "back".to_string()],
            0,
        )],
    );
    while session.next_side().is_some() {
        session.reveal();
    }
    assert!(session.grade(Grade::Forgot).is_some(), "graded");
    session.sent(42);
    assert!(session.acknowledge(42, 1_000), "acknowledged");
    State {
        session,
        ..State::default()
    }
}

/// A library on disk holding one deck and one two-sided card, and a handle onto it.
fn library_with_one_card(dir: &tempfile::TempDir) -> Handle {
    let path = dir.path().join("library.sqlite3");
    {
        let db = typr_core::store::Db::open(&path).expect("open");
        let deck = match db
            .apply(
                &typr_core::store::Command::CreateDeck {
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
        db.apply(
            &typr_core::store::Command::CreateCard {
                deck,
                markdown: "front\n---\nback".to_string(),
            },
            0,
        )
        .expect("card");
    }
    Handle::spawn(
        typr_core::store::Db::open(&path).expect("open"),
        Box::new(typr_core::clock::SystemClock),
        std::sync::Arc::new(|| {}),
    )
    .expect("spawn")
}

/// A session showing the library's only card, revealed to its last side.
fn session_on(card: Id) -> Session {
    let mut session = Session::new(Queue::Due, vec![Entry::sides(card, ["front", "back"], 0)]);
    while session.next_side().is_some() {
        session.reveal();
    }
    session
}

/// A library with one card that has a record and one that has none.
fn library_with_a_reviewed_and_a_fresh_card(dir: &tempfile::TempDir) -> (Handle, Id, Id) {
    let path = dir.path().join("library.sqlite3");
    let (reviewed, fresh) = {
        let db = typr_core::store::Db::open(&path).expect("open");
        let deck = match db
            .apply(
                &typr_core::store::Command::CreateDeck {
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
                &typr_core::store::Command::CreateCard {
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
        let reviewed = card("front\n---\nback");
        card("other front\n---\nother back");
        db.apply(
            &typr_core::store::Command::Grade {
                card: reviewed,
                key: "fwd".to_string(),
                grade: typr_core::store::Grade::Remembered,
                duration_ms: None,
            },
            1_000,
        )
        .expect("grade");
        (reviewed, db.snapshot().expect("snapshot").cards[1].id)
    };
    let handle = Handle::spawn(
        typr_core::store::Db::open(&path).expect("open"),
        Box::new(typr_core::clock::SystemClock),
        std::sync::Arc::new(|| {}),
    )
    .expect("spawn");
    (handle, reviewed, fresh)
}

/// Draw the library screen from whatever the store has published.
fn refresh_library(state: &mut State) {
    let snapshot = state.store.as_ref().expect("store").snapshot();
    state.library.set_snapshot(snapshot);
}

/// Wait for one command's answer, then draw what the store now holds.
fn settle(state: &mut State, id: u64) {
    for _ in 0..200 {
        let answered = state
            .store
            .as_ref()
            .expect("store")
            .poll()
            .iter()
            .any(|event| event.id == id);
        if answered {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    refresh_library(state);
}

/// Wait for the published library to hold `count` decks, then draw it.
fn wait_for_decks(state: &mut State, count: usize) {
    for _ in 0..200 {
        refresh_library(state);
        if state.library.snapshot().decks.len() == count {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!(
        "expected {count} decks, found {}",
        state.library.snapshot().decks.len()
    );
}

/// A library with two cards due now and a third that falls due six seconds from now, which
/// leaves the queue it is missing from a margin wide enough for a slow machine.
fn library_with_one_card_due_and_one_almost(dir: &tempfile::TempDir) -> Handle {
    let path = dir.path().join("library.sqlite3");
    {
        let db = typr_core::store::Db::open(&path).expect("open");
        let deck = match db
            .apply(
                &typr_core::store::Command::CreateDeck {
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
        for markdown in ["one\n---\ntwo", "three\n---\nfour", "five\n---\nsix"] {
            db.apply(
                &typr_core::store::Command::CreateCard {
                    deck,
                    markdown: markdown.to_string(),
                },
                0,
            )
            .expect("card");
        }
        let now = typr_core::clock::SystemClock.now_ms();
        // A remembered grade graduates a prompt ten minutes later, so back-dating the grade is
        // what sets the due date: an hour ago for the first two cards, and two seconds ahead for
        // the third, which is still in the future when the session opens.
        for (card, graded_at) in [
            (1, now - 3_600_000),
            (2, now - 3_600_000),
            (3, now + 6_000 - 600_000),
        ] {
            db.apply(
                &typr_core::store::Command::Grade {
                    card,
                    key: "fwd".to_string(),
                    grade: Grade::Remembered,
                    duration_ms: None,
                },
                graded_at,
            )
            .expect("grade");
        }
    }
    Handle::spawn(
        typr_core::store::Db::open(&path).expect("open"),
        Box::new(typr_core::clock::SystemClock),
        std::sync::Arc::new(|| {}),
    )
    .expect("spawn")
}

/// Pump the app until `done` holds, refreshing the library each turn.
fn wait_for(state: &mut State, what: &str, done: impl Fn(&State) -> bool) {
    for _ in 0..300 {
        let now = Clock::now_ms(&typr_core::clock::SystemClock);
        let mut wake = None;
        frame_step(state, now, &mut wake);
        refresh_library(state);
        if done(state) {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("timed out waiting for {what}");
}

/// A library on disk holding one deck and one card, and a handle onto it.
fn library_with_a_card(dir: &tempfile::TempDir) -> Handle {
    library_with_one_card(dir)
}

/// The due date the store holds for a card's prompt, read back through its history.
fn due_at(state: &mut State, card: Id) -> Option<i64> {
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
            match event.result {
                Ok(Outcome::History { rows, .. }) => return rows.last().and_then(|row| row.due_at),
                other => panic!("the history answer was {other:?}"),
            }
        }
        let mut wake = None;
        frame_step(state, 0, &mut wake);
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    panic!("the store never answered the history request");
}

/// A library holding one card with the given markdown, already open.
fn library_with_markdown(dir: &tempfile::TempDir, markdown: &str) -> Handle {
    let path = dir.path().join("library.sqlite3");
    {
        let db = typr_core::store::Db::open(&path).expect("open");
        let deck = match db
            .apply(
                &typr_core::store::Command::CreateDeck {
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
        db.apply(
            &typr_core::store::Command::CreateCard {
                deck,
                markdown: markdown.to_string(),
            },
            0,
        )
        .expect("card");
    }
    Handle::spawn(
        typr_core::store::Db::open(&path).expect("open"),
        Box::new(typr_core::clock::SystemClock),
        std::sync::Arc::new(|| {}),
    )
    .expect("spawn")
}
