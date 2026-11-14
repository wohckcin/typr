use rune::{Env, Key, KeyEvent, Mods, Runtime, Tx};

use super::{Host, key, summary, view};
use typr_core::keymap::{Surface, bindings_of};
use typr_core::screen::Queue;
use typr_core::session::{Entry, Session};
use typr_core::store::{Grade, Id, StoreError};

const NOON: i64 = 1_760_000_000_000;

struct App {
    session: Session,
    sent: Vec<(u64, Id, Grade)>,
    /// The prompt key each grade was sent under, so a test can say which prompt was answered.
    keys: Vec<(Id, String)>,
    due: usize,
    left: bool,
    shuffles: u32,
    /// The size the picture pane reads as laid out, which places a diagram face's covers.
    pane: (f32, f32),
}

impl App {
    fn with(queue: Queue, cards: &[(i64, &[&str])]) -> App {
        let entries = cards
            .iter()
            .map(|(card, sides)| Entry::sides(*card as Id, sides.iter().map(|s| s.to_string()), 0))
            .collect();
        App {
            session: Session::new(queue, entries),
            sent: Vec::new(),
            keys: Vec::new(),
            due: 0,
            left: false,
            shuffles: 0,
            pane: (400.0, 300.0),
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

    fn submit_grade(&mut self, card: Id, key: &str, grade: Grade) -> Result<u64, StoreError> {
        let id = self.sent.len() as u64 + 1;
        self.sent.push((id, card, grade));
        self.keys.push((card, key.to_string()));
        Ok(id)
    }

    fn picture_pane(&self) -> (f32, f32) {
        self.pane
    }

    fn set_picture_pane(&mut self, width: f32, height: f32) {
        self.pane = (width, height);
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
        "the card opens on its first side, so the grade is not offered yet"
    );

    press(&mut rt, Key::Text(" ".into()));
    rt.frame(0.0);
    assert!(
        rt.find("remembered").is_some(),
        "the last side is shown and the grade is offered"
    );
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
        let id = app.submit_grade(card, "fwd", grade).expect("sent");
        app.session.sent(id);
        app.acknowledge();
    }
    // The forgotten card is in a relearning step; once ten minutes pass it is back.
    app.session
        .advance_clock(NOON + typr_core::clock::LEARNING_STEP_MS);
    app.session.reveal();
    let (card, grade) = app.session.grade(Grade::Remembered).unwrap();
    let id = app.submit_grade(card, "fwd", grade).expect("sent");
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

#[test]
fn the_session_draws_the_bindings_the_table_holds() {
    fn draw(s: &App, _: &mut rune::Cx<App>) -> rune::El {
        view::<App>(s)
    }
    let app = App::with(Queue::New, &[(1, &["a", "b"])]);
    let mut runtime = Runtime::new(app, draw, (900.0, 600.0), Env::new(true, 1.0));
    runtime.frame(0.1);
    let drawn: Vec<String> = runtime
        .elements()
        .into_iter()
        .filter_map(|element| element.text)
        .collect();
    for binding in bindings_of(Surface::Session) {
        let row = format!("{}  {}", binding.chord, binding.command.label());
        assert!(
            drawn.contains(&row),
            "the session does not draw {row:?}; it draws {drawn:?}"
        );
    }
}

use typr_core::store::MaskRect;

fn draw_diagram(s: &App, _: &mut rune::Cx<App>) -> rune::El {
    view(s)
}

fn diagram_app(masks: Vec<MaskRect>) -> App {
    let mut app = App::with(Queue::Due, &[]);
    app.session = Session::new(
        Queue::Due,
        vec![Entry::mask_group(
            4,
            "mask:2",
            "plate.png",
            Some("The chambers of the heart".to_string()),
            masks,
            0,
        )],
    );
    app
}

fn render(app: App) -> Runtime<App> {
    let mut runtime = Runtime::new(app, draw_diagram, (900.0, 700.0), Env::new(true, 1.0));
    // The picture pane's size arrives on the first layout, so covers are placed on the second frame.
    runtime.frame(0.0);
    runtime.frame(0.1);
    runtime
}

fn keys_at(runtime: &Runtime<App>, prefix: &str) -> Vec<String> {
    runtime
        .elements()
        .into_iter()
        .filter_map(|element| element.key)
        .filter(|key| key.starts_with(prefix))
        .collect()
}

#[test]
fn a_diagram_review_covers_only_its_own_group() {
    // Each state is measured in its own runtime: Rune keeps a keyed element after it leaves the tree,
    // so `elements()` from one run still lists covers a later frame removed.
    let covered = keys_at(
        &render(diagram_app(vec![
            MaskRect::new(0.1, 0.1, 0.2, 0.2),
            MaskRect::new(0.2, 0.1, 0.1, 0.1),
        ])),
        "cover-",
    );
    assert_eq!(
        covered.len(),
        2,
        "one cover per rectangle of the group being asked, and none for a group that is not"
    );

    let mut uncovered_app = diagram_app(vec![MaskRect::new(0.1, 0.1, 0.2, 0.2)]);
    assert!(uncovered_app.session.reveal(), "the reveal is the answer");
    let uncovered = keys_at(&render(uncovered_app), "cover-");
    assert!(
        uncovered.is_empty(),
        "an uncovered group has nothing over it: {uncovered:?}"
    );
}

#[test]
fn a_diagram_review_shows_its_caption() {
    let app = diagram_app(vec![MaskRect::new(0.1, 0.1, 0.2, 0.2)]);
    let mut runtime = Runtime::new(app, draw_diagram, (900.0, 700.0), Env::new(true, 1.0));
    runtime.frame(0.0);
    let caption = runtime
        .elements()
        .into_iter()
        .find(|element| element.key.as_deref() == Some("diagram-caption"))
        .and_then(|element| element.text);
    assert_eq!(caption.as_deref(), Some("The chambers of the heart"));
}
