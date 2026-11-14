//! Verification probes for interrogate findings 2, 8, 9.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use typr_core::clock::Clock;
use typr_core::store::{Command, Db, Grade, Handle, Id, Outcome, Store, StoreError, Waker};

const NOON: i64 = 1_760_000_000_000;

/// A clock that panics on the Nth reading.
struct Stops(AtomicUsize, usize);

impl Clock for Stops {
    fn now_ms(&self) -> i64 {
        let seen = self.0.fetch_add(1, Ordering::SeqCst) + 1;
        assert!(seen != self.1, "the clock fell over");
        NOON
    }
}

fn deck_and_card(db: &Db, deck: &str, card: &str) -> (Id, Id) {
    let Ok(Outcome::Deck(id)) = db.apply(
        &Command::CreateDeck {
            parent: None,
            name: deck.into(),
        },
        NOON,
    ) else {
        panic!("deck")
    };
    let Ok(Outcome::Card(card_id)) = db.apply(
        &Command::CreateCard {
            deck: id,
            markdown: card.into(),
        },
        NOON,
    ) else {
        panic!("card")
    };
    (id, card_id)
}

/// study-session / "A session is scoped to a deck": the store offers no way to ask for one deck.
#[test]
fn the_due_queue_has_no_deck_scope() {
    let dir = tempfile::tempdir().expect("tmp");
    let db = Db::open(&dir.path().join("typr.sqlite3")).expect("open");
    let (verbs, verb) = deck_and_card(&db, "Verbs", "run\n---\nran");
    let (_nouns, noun) = deck_and_card(&db, "Nouns", "cat\n---\ncats");
    for card in [verb, noun] {
        db.apply(
            &Command::Grade {
                card,
                key: "fwd".into(),
                grade: Grade::Forgot,
                duration_ms: None,
            },
            NOON,
        )
        .expect("learn forgot");
        db.apply(
            &Command::Grade {
                card,
                key: "fwd".into(),
                grade: Grade::Remembered,
                duration_ms: None,
            },
            NOON,
        )
        .expect("graduate");
    }
    let due: Vec<Id> = db
        .due_prompts(NOON + 20 * 60_000, 0, 7)
        .expect("due")
        .into_iter()
        .map(|p| p.card)
        .collect();
    println!("deck Verbs asked alone would hold {verb}; the due queue returns {due:?}");
    println!("prompt_keys for the noun card: {:?}", db.prompt_keys(noun).unwrap());
    println!("note: the only deck the caller can name is {verbs}, and no query takes it");
    assert!(
        due.contains(&noun),
        "the due queue would have excluded the sibling deck's card if it were scoped"
    );
}

/// study-session / "Prompts falling due mid-session join the running queue": nothing but the
/// session's own relearning steps can arrive.
#[test]
fn only_acknowledge_and_clock_advance_mutate_a_session() {
    let source = std::fs::read_to_string("src/session.rs").expect("session.rs");
    let calls: Vec<&str> = source
        .lines()
        .filter(|line| line.contains(".arrive("))
        .collect();
    println!("calls to arrive inside session.rs: {calls:?}");
    let outside = std::process::Command::new("grep")
        .args(["-rn", "arrive(", "--include=*.rs", "."])
        .output()
        .expect("grep");
    println!("{}", String::from_utf8_lossy(&outside.stdout));
}

/// diagnostics / "The backend reports failures without logging": a panic in the second message of
/// a batch is reported against the first message's id.
#[test]
fn a_panic_in_a_batch_is_attributed_to_the_wrong_request() {
    let dir = tempfile::tempdir().expect("tmp");
    let db = Db::open(&dir.path().join("typr.sqlite3")).expect("open");
    let Ok(Outcome::Deck(deck)) = db.apply(
        &Command::CreateDeck {
            parent: None,
            name: "D".into(),
        },
        NOON,
    ) else {
        panic!("deck")
    };
    // Both commands are queued before the worker starts reading, so they share one batch.
    // The clock panics on the batch's second reading, i.e. inside the second message.
    let clock = Box::new(Stops(AtomicUsize::new(0), 2));
    let waker: Waker = Arc::new(|| {});
    let handle = Handle::spawn(Db::open(&dir.path().join("second.sqlite3")).expect("db"), clock, waker)
        .expect("spawn");
    let first = handle.create_deck(None, "A").expect("first sent");
    let second = handle.create_deck(None, "B").expect("second sent");
    let mut events = Vec::new();
    for _ in 0..40 {
        events.extend(handle.poll());
        if events.len() >= 2 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    drop(handle);
    let answered: Vec<u64> = events.iter().map(|event| event.id).collect();
    let panicked: Vec<String> = events
        .iter()
        .filter_map(|event| match &event.result {
            Err(StoreError::WorkerPanicked { message }) => Some(message.clone()),
            _ => None,
        })
        .collect();
    println!("deck {deck}: first={first} second={second} answered={answered:?} panics={panicked:?}");
    assert!(
        events.iter().all(|event| event.id != second),
        "the second request got no answer at all, but reported {answered:?}"
    );
}
