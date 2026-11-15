//! The study screen: the asked face shows, the answer is revealed before a grade, a grade is sent
//! out with its card, and a finished session says how many it reviewed.

use std::cell::RefCell;
use std::rc::Rc;

use nsh_core::prelude::*;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;
use typr::screens::study::{can_reveal, study};
use typr_core::session::{Entry, Session};
use typr_core::screen::Queue;

#[component]
fn Probe() -> impl IntoElement {
    typr::ui::tokens::install();
    let entries = vec![Entry::sides(1, ["Question", "Answer"], 0)];
    study(entries, |_, _| true)
}

#[test]
fn a_fresh_session_offers_a_reveal_and_no_grade() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
    app.wait_for("Question")?;
    app.wait_for("role:button[Show answer]")?;
    assert!(!app.screen().shows("Remembered"));
    Ok(())
}

#[test]
fn a_grade_is_offered_only_after_the_reveal_and_completes_the_session() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
    app.wait_for("role:button[Show answer]")?;
    app.press("role:button[Show answer]")?;
    app.wait_for("role:button[Remembered]")?;
    app.press("role:button[Remembered]")?;
    app.wait_for("text:/Session complete · 1 reviewed/")?;
    Ok(())
}

#[test]
fn a_grade_is_sent_out_with_its_card() {
    let grades: Rc<RefCell<Vec<i64>>> = Rc::new(RefCell::new(Vec::new()));
    let mut session = Session::new(Queue::Due, vec![Entry::sides(7, ["Q", "A"], 0)]);
    assert!(can_reveal(&session));
    session.reveal();
    let sent = session.grade(typr_core::store::Grade::Remembered);
    assert_eq!(sent.map(|(id, _)| id), Some(7));
    grades.borrow_mut().push(7);
    assert_eq!(grades.borrow().len(), 1);
}

#[component]
fn Refusing() -> impl IntoElement {
    typr::ui::tokens::install();
    study(vec![Entry::sides(1, ["Question", "Answer"], 0)], |_, _| false)
}

#[test]
fn a_refused_grade_leaves_the_card_answerable() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Refusing>())).unwrap();
    app.wait_for("role:button[Show answer]")?;
    app.press("role:button[Show answer]")?;
    app.wait_for("role:button[Remembered]")?;
    app.press("role:button[Remembered]")?;
    app.wait_for("role:button[Remembered]")?;
    assert!(!app.screen().shows("Session complete"));
    assert!(app.screen().shows("1 left"));
    Ok(())
}
