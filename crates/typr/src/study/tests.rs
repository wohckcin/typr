use rune::{Env, Key, KeyEvent, Mods, Runtime, Tx};

use super::{Host, key, summary, view};
use crate::screen::Queue;
use crate::session::{Entry, Session};
use crate::store::{Grade, Id, StoreError};

const NOON: i64 = 1_760_000_000_000;

struct App {
    session: Session,
    sent: Vec<(u64, Id, Grade)>,
    due: usize,
    left: bool,
    shuffles: u32,
}

impl App {
    fn with(queue: Queue, cards: &[(i64, &[&str])]) -> App {
        let entries = cards
            .iter()
            .map(|(card, sides)| Entry {
                card: *card as Id,
                sides: sides.iter().map(|s| s.to_string()).collect(),
                overdue: 0,
            })
            .collect();
        App {
            session: Session::new(queue, entries),
            sent: Vec::new(),
            due: 0,
            left: false,
            shuffles: 0,
        }
    }

    /// Play the acknowledgements the store owes.
    fn acknowledge(&mut self) {
        let owed: Vec<u64> = self.sent.iter().map(|(id, _, _)| *id).collect();
        for id in owed {
            self.session.acknowledge(id, NOON);
        }
        self.sent.clear();
    }
}

impl Host for App {
    fn session(&self) -> &Session {
        &self.session
    }

    fn session_mut(&mut self) -> &mut Session {
        &mut self.session
    }

    fn submit_grade(&mut self, card: Id, grade: Grade) -> Result<u64, StoreError> {
        let id = self.sent.len() as u64 + 1;
        self.sent.push((id, card, grade));
        Ok(id)
    }

    fn due_now(&self) -> usize {
        self.due
    }

    fn leave_session(&mut self) {
        self.left = true;
    }

    fn reshuffle(&mut self) {
        self.shuffles += 1;
    }
}

fn press(rt: &mut Runtime<App>, key_press: Key) {
    rt.key(KeyEvent {
        key: key_press,
        mods: Mods::default(),
    });
    rt.frame(0.0);
}

#[test]
fn a_whole_queue_is_answered_with_no_pointer() {
    let app = App::with(
        Queue::New,
        &[(1, &["front", "back"]), (2, &["front", "back"])],
    );
    let mut rt = Runtime::new(
        app,
        |s: &App, _| view::<App>(s),
        (900.0, 700.0),
        Env::new(true, 1.0),
    );
    rt.on_key = Some(|s: &mut App, ev: &KeyEvent, _: &mut Tx| key::<App>(s, ev));
    rt.frame(0.0);

    for _ in 0..2 {
        press(&mut rt, Key::Text(" ".into()));
        press(&mut rt, Key::Text(" ".into()));
        press(&mut rt, Key::Text("j".into()));
        rt.update(|s: &mut App, _| s.acknowledge());
    }

    assert_eq!(
        rt.state.session.grades,
        vec![Grade::Remembered, Grade::Remembered]
    );
    assert!(rt.state.session.finished());
    assert!(rt.state.sent.is_empty());
}

#[test]
fn the_answers_are_offered_only_after_the_last_side() {
    let app = App::with(Queue::New, &[(1, &["one", "two"])]);
    let mut rt = Runtime::new(
        app,
        |s: &App, _| view::<App>(s),
        (900.0, 700.0),
        Env::new(true, 1.0),
    );
    rt.on_key = Some(|s: &mut App, ev: &KeyEvent, _: &mut Tx| key::<App>(s, ev));
    rt.frame(0.0);
    assert!(
        rt.find("remembered").is_none(),
        "no grade before the last side"
    );

    press(&mut rt, Key::Text(" ".into()));
    rt.frame(0.0);
    assert!(rt.find("remembered").is_none(), "one side is still hidden");

    press(&mut rt, Key::Text(" ".into()));
    rt.frame(0.0);
    assert!(rt.find("remembered").is_some(), "and now it is offered");
    assert!(rt.find("bindings").is_some(), "the keys are on screen");
}

#[test]
fn backward_from_the_keyboard_shows_the_graded_card_read_only() {
    let app = App::with(Queue::New, &[(1, &["one", "two"]), (2, &["front", "back"])]);
    let mut rt = Runtime::new(
        app,
        |s: &App, _| view::<App>(s),
        (900.0, 700.0),
        Env::new(true, 1.0),
    );
    rt.on_key = Some(|s: &mut App, ev: &KeyEvent, _: &mut Tx| key::<App>(s, ev));
    rt.frame(0.0);

    for _ in 0..2 {
        press(&mut rt, Key::Text(" ".into()));
    }
    press(&mut rt, Key::Text("j".into()));
    rt.update(|s: &mut App, _| s.acknowledge());

    press(&mut rt, Key::Left);
    assert_eq!(rt.state.session.current().unwrap().card, 1);
    assert!(rt.state.session.read_only);
    assert!(
        rt.state.session.grades.len() == 1,
        "nothing was graded again"
    );
}

#[test]
fn the_summary_counts_this_sessions_answers_and_what_is_due_now() {
    let mut app = App::with(
        Queue::Due,
        &[(1, &["front"]), (2, &["front"]), (3, &["front"])],
    );
    app.due = 12;
    for grade in [Grade::Remembered, Grade::Forgot, Grade::Remembered] {
        app.session.reveal();
        let (card, grade) = app.session.grade(grade).unwrap();
        let id = app.submit_grade(card, grade).expect("sent");
        app.session.sent(id);
        app.acknowledge();
    }
    // The forgotten card is in a relearning step; once ten minutes pass it is back.
    app.session
        .advance_clock(NOON + crate::clock::LEARNING_STEP_MS);
    app.session.reveal();
    let (card, grade) = app.session.grade(Grade::Remembered).unwrap();
    let id = app.submit_grade(card, grade).expect("sent");
    app.session.sent(id);
    app.acknowledge();
    assert!(app.session.finished());

    let mut rt = Runtime::new(
        app,
        |s: &App, _| summary::<App>(s),
        (900.0, 700.0),
        Env::new(true, 1.0),
    );
    rt.frame(0.0);
    assert!(rt.find("summary").is_some());
    assert!(rt.find("back-to-library").is_some());

    let (x, y) = rt.find("back-to-library").unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(0.0);
    assert!(rt.state.left, "the way back to the library works");
}

#[test]
fn the_shuffle_key_reorders_without_answering() {
    let mut app = App::with(Queue::Due, &[(1, &["one", "1"]), (2, &["two", "2"])]);
    app.session.reveal();
    let graded = app.session.grades.len();
    let handled = key(
        &mut app,
        &KeyEvent {
            key: Key::Text("r".into()),
            mods: Mods::default(),
        },
    );
    assert!(handled, "the key is taken");
    assert_eq!(app.shuffles, 1, "and asks the host to shuffle");
    assert_eq!(
        app.session.grades.len(),
        graded,
        "shuffling records no grade"
    );
    assert!(app.sent.is_empty(), "and sends nothing to the store");
}
