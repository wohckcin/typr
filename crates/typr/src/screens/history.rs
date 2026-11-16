//! A card's review history: how many reviews it has had, and every record with its phase, grade,
//! time and due date, newest first. The records are passed in; the screen reads no database.

use nsh_core::prelude::*;

use crate::ui::{status::badge, surface, tokens};

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

/// The history of one card: the count, then its records newest first, or an empty state.
pub fn history(records: Vec<Record>) -> impl IntoElement {
    let count = review_count(&records);
    let mut list = view().gap_2();
    if records.is_empty() {
        list = list.child(
            text("No reviews yet")
                .text_sm()
                .text_color(tokens::muted_foreground),
        );
    }
    for record in records.iter().rev() {
        list = list.child(
            surface::surface().child(
                view()
                    .gap_1()
                    .child(
                        text(format!("{} · {}", record.phase, record.grade))
                            .text_sm()
                            .text_color(tokens::foreground),
                    )
                    .child(
                        text(format!("{} · due {}", record.at, record.due))
                            .text_xs()
                            .text_color(tokens::muted_foreground),
                    ),
            ),
        );
    }
    view()
        .flex_1()
        .gap_3()
        .p_4()
        .child(badge().label(format!("{count} reviews")))
        .child(list)
}
