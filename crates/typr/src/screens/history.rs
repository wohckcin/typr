//! A card's review history: how many reviews it has had, and every record with its phase, grade,
//! time and due date, newest first. The records are passed in; the screen reads no database.

use nsh_core::prelude::*;

use crate::ui::{status::badge, table::table, tokens};

/// One review record as the screen shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    /// `learn` or `review`.
    pub phase: SharedString,
    /// The grade the user gave.
    pub grade: SharedString,
    /// When the review happened, already formatted.
    pub at: SharedString,
    /// When the card is next due, already formatted.
    pub due: SharedString,
}

/// The number of reviews a card has had: every record except the learning-phase ones.
pub fn review_count(records: &[Record]) -> usize {
    records
        .iter()
        .filter(|r| r.phase.as_ref() != "learn")
        .count()
}

/// The history of one card: the count, then its records newest first in a table, or an empty state.
pub fn history(records: Vec<Record>) -> impl IntoElement {
    let count = review_count(&records);
    let body = if records.is_empty() {
        view()
            .child(
                text("No reviews yet")
                    .text_sm()
                    .text_color(tokens::muted_foreground),
            )
            .into_element()
    } else {
        let rows: Vec<Vec<SharedString>> = records
            .iter()
            .rev()
            .map(|record| {
                vec![
                    record.phase.clone(),
                    record.grade.clone(),
                    record.at.clone(),
                    record.due.clone(),
                ]
            })
            .collect();
        table()
            .headers(
                ["Phase", "Grade", "When", "Due"]
                    .map(SharedString::from)
                    .to_vec(),
            )
            .rows(rows)
            .accessibility_label("Review history")
            .into_element()
    };
    view()
        .flex_1()
        .gap_3()
        .p_4()
        .child(badge().label(format!("{count} reviews")))
        .child(body)
}
