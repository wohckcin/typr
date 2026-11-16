//! The history screen: the count leaves out learning records, every record is listed newest first,
//! and an empty card says so.

use nsh_core::prelude::*;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;
use typr::screens::history::{Record, history, review_count};

fn record(phase: &'static str, grade: &'static str, at: &'static str) -> Record {
    Record {
        phase: phase.into(),
        grade: grade.into(),
        at: at.into(),
        due: "tomorrow".into(),
    }
}

#[component]
fn Probe() -> impl IntoElement {
    typr::ui::tokens::install();
    history(vec![
        record("learn", "good", "09:00"),
        record("learn", "good", "09:05"),
        record("learn", "again", "09:10"),
        record("review", "good", "10:00"),
        record("review", "easy", "11:00"),
    ])
}

#[component]
fn Empty() -> impl IntoElement {
    typr::ui::tokens::install();
    history(Vec::new())
}

#[test]
fn the_count_leaves_out_learning_records() {
    let records = vec![
        record("learn", "good", "a"),
        record("learn", "good", "b"),
        record("learn", "again", "c"),
        record("review", "good", "d"),
        record("review", "easy", "e"),
    ];
    assert_eq!(review_count(&records), 2);
}

#[test]
fn every_record_is_listed_newest_first() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
    app.wait_for("text:/review · easy/")?;
    app.wait_for("text:/^2 reviews$/")?;
    let texts: Vec<String> = app.screen().texts().iter().map(|t| t.to_string()).collect();
    let at = |needle: &str| texts.iter().position(|t| t.contains(needle));
    assert!(at("11:00").unwrap() < at("10:00").unwrap(), "{texts:?}");
    assert!(at("09:00").is_some(), "{texts:?}");
    Ok(())
}

#[test]
fn a_card_with_no_reviews_says_so() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Empty>())).unwrap();
    app.wait_for("No reviews yet")?;
    Ok(())
}
