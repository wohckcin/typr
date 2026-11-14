use rune::{Cx, El, Env, Runtime};

use super::*;
use crate::store::Phase;

fn record(phase: Phase, grade: Grade, at: i64) -> ReviewRow {
    ReviewRow {
        card: 1,
        key: "fwd".to_string(),
        phase,
        at,
        grade,
        interval_ms: 0,
        due_at: None,
        duration_ms: None,
    }
}

fn draw(s: &History, _: &mut Cx<History>) -> El {
    view::<History>(s)
}

/// What the element keyed `key` shows, read from the frame the runtime just drew.
fn shown(rt: &Runtime<History>, key: &str) -> Option<String> {
    rt.elements()
        .into_iter()
        .find(|element| element.key.as_deref() == Some(key))
        .and_then(|element| element.text)
}

#[test]
fn the_panel_shows_the_count_the_store_gave_it() {
    let mut history = History::default();
    history.set(
        1,
        vec![
            record(Phase::Learn, Grade::Forgot, 1_000),
            record(Phase::Learn, Grade::Remembered, 2_000),
            record(Phase::Learn, Grade::Forgot, 3_000),
            record(Phase::Review, Grade::Remembered, 4_000),
            record(Phase::Review, Grade::Forgot, 5_000),
        ],
        2,
    );
    assert_eq!(history.count, 2, "the store's count, not a recount here");
    assert_eq!(history.rows.len(), 5, "every record is listed");
}

#[test]
fn every_record_is_drawn_with_its_phase_grade_and_due_date() {
    let mut history = History::default();
    history.set(
        1,
        vec![
            record(Phase::Review, Grade::Remembered, 0),
            record(Phase::Relearn, Grade::Forgot, 60_000),
        ],
        2,
    );
    let mut rt = Runtime::new(history, draw, (960.0, 640.0), Env::new(true, 1.0));
    rt.frame(0.0);

    assert_eq!(shown(&rt, "history-count").as_deref(), Some("2 reviews"));
    assert_eq!(shown(&rt, "history-records").as_deref(), Some("2 records"));
    assert!(rt.find("history-0").is_some(), "the first record is drawn");
    assert!(
        rt.find("history-60000").is_some(),
        "the relearning record is drawn"
    );
}
