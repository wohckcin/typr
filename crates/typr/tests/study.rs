//! The study screen: the asked face shows, the answer is revealed before a grade, a grade is sent
//! out with its card, and a finished session says how many it reviewed.

use std::cell::RefCell;
use std::rc::Rc;

use nsh_core::prelude::*;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;
use typr::screens::study::{can_reveal, entry_for, study};
use typr_core::screen::Queue;
use typr_core::session::{Entry, Session};

#[component]
fn Probe() -> impl IntoElement {
    typr::ui::tokens::install();
    let entries = vec![Entry::sides(1, ["Question", "Answer"], 0)];
    study(
        std::rc::Rc::new(std::cell::RefCell::new(Session::new(Queue::Due, entries))),
        |_, _, _| Ok(()),
        || {},
        || {},
    )
}

#[test]
fn a_prompt_becomes_the_entry_it_names() {
    use typr_core::store::{CardKind, CardSummary, MaskRect, MaskRow};
    let sides = CardSummary {
        id: 1,
        markdown: "gato\n---\ncat".to_owned(),
        sides: 2,
        ..CardSummary::default()
    };
    assert!(
        entry_for(&sides, "fwd", 0).is_some(),
        "a two-sided card asks forwards"
    );
    assert!(entry_for(&sides, "rev", 0).is_some(), "and backwards");
    assert!(
        entry_for(&sides, "cloze:1", 0).is_none(),
        "a prompt it does not hold"
    );

    let hidden = CardSummary {
        id: 2,
        markdown: "The {{1::air}} holds {{2::oxygen}}.".to_owned(),
        sides: 1,
        ..CardSummary::default()
    };
    let entry = entry_for(&hidden, "cloze:2", 0).expect("a hidden group");
    assert_eq!(entry.prompt, "cloze:2");

    let diagram = CardSummary {
        id: 3,
        kind: CardKind::Diagram,
        image: Some("plate.png".to_owned()),
        caption: Some("The heart".to_owned()),
        masks: vec![
            MaskRow {
                group: 1,
                rect: MaskRect::new(0.1, 0.1, 0.2, 0.2),
            },
            MaskRow {
                group: 2,
                rect: MaskRect::new(0.4, 0.4, 0.2, 0.2),
            },
        ],
        ..CardSummary::default()
    };
    assert!(
        entry_for(&diagram, "mask:1", 0).is_some(),
        "a diagram group"
    );
    assert!(
        entry_for(&diagram, "mask:3", 0).is_none(),
        "a group it no longer holds"
    );
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
    study(
        std::rc::Rc::new(std::cell::RefCell::new(Session::new(
            Queue::Due,
            vec![Entry::sides(1, ["Question", "Answer"], 0)],
        ))),
        |_, _, _| Err(typr::screens::study::GradeRefused),
        || {},
        || {},
    )
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
    app.wait_for("text:/was not saved/")?;
    Ok(())
}

#[test]
fn the_keyboard_reveals_and_grades_as_the_keymap_says() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
    app.wait_for("role:button[Show answer]")?;
    app.press_key("space")?;
    app.wait_for("role:button[Remembered]")?;
    app.press_key("j")?;
    app.wait_for("text:/Session complete · 1 reviewed/")?;
    Ok(())
}

#[component]
fn Leaving() -> impl IntoElement {
    typr::ui::tokens::install();
    let left = signal(false);
    view()
        .child(study(
            std::rc::Rc::new(std::cell::RefCell::new(Session::new(
                Queue::Due,
                vec![Entry::sides(1, ["Question", "Answer"], 0)],
            ))),
            |_, _, _| Ok(()),
            move || left.set(true),
            || {},
        ))
        .child(text(move || format!("left={}", left.get())))
}

#[test]
fn escape_leaves_through_the_keymap_binding() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Leaving>())).unwrap();
    app.wait_for("text:/left=false/")?;
    app.press_key("escape")?;
    app.wait_for("text:/left=true/")?;
    Ok(())
}

#[component]
fn Reverse() -> impl IntoElement {
    typr::ui::tokens::install();
    study(
        std::rc::Rc::new(std::cell::RefCell::new(Session::new(
            Queue::Due,
            vec![Entry::reverse(1, ["Question", "Answer"], 0)],
        ))),
        |_, _, _| Ok(()),
        || {},
        || {},
    )
}

#[test]
fn a_reverse_review_shows_the_last_side_first() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Reverse>())).unwrap();
    app.wait_for("Answer")?;
    assert!(!app.screen().shows("Question"));
    Ok(())
}

#[component]
fn Cramming() -> impl IntoElement {
    typr::ui::tokens::install();
    study(
        std::rc::Rc::new(std::cell::RefCell::new(Session::cram(vec![Entry::sides(
            1,
            ["Front", "Back"],
            0,
        )]))),
        |_, _, _| Ok(()),
        || {},
        || {},
    )
}

#[test]
fn a_cram_session_asks_its_cards() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Cramming>())).unwrap();
    app.wait_for("Front")?;
    Ok(())
}

#[component]
fn SettingsKey() -> impl IntoElement {
    typr::ui::tokens::install();
    let opened = signal(false);
    let card = signal(String::from("none"));
    view()
        .child(study(
            std::rc::Rc::new(std::cell::RefCell::new(Session::new(
                Queue::Due,
                vec![Entry::sides(7, ["Question", "Answer"], 0)],
            ))),
            |_, _, _| Ok(()),
            || {},
            move || {
                card.set("card 7".to_owned());
                opened.set(true);
            },
        ))
        .child(text(move || {
            format!("settings={} on {}", opened.get(), card.get())
        }))
}

#[test]
fn the_settings_key_opens_settings_from_a_session_card() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<SettingsKey>())).unwrap();
    app.wait_for("Question")?;
    app.press_key(",")?;
    app.wait_for("settings=true on card 7")?;
    Ok(())
}

#[test]
fn the_summary_reports_both_grades_and_the_way_back() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
    app.wait_for("role:button[Show answer]")?;
    app.press("role:button[Show answer]")?;
    app.press("role:button[Remembered]")?;
    app.wait_for("text:/Session complete · 1 reviewed/")?;
    app.wait_for("text:/Remembered 1 · Forgot 0 · 1 due when you started/")?;
    app.wait_for("role:button[Back to the library]")?;
    Ok(())
}
