//! The study session: the asked face of the current card, the reveal, and the two grades. The
//! session's rules (the queue, the order of sides, what a grade schedules) are typr-core's; this
//! screen shows the session and sends each grade out through `on_grade`, which the caller writes.

use nsh_core::prelude::*;
use typr_core::clock::{Clock, SystemClock};
use typr_core::screen::Queue;
use typr_core::session::{Entry, Session};
use typr_core::store::{Grade, Id};

use crate::ui::{
    button::{Variant, button},
    prose::prose,
    status::badge,
    tokens,
};

/// Reveals the answer when the face allows it. Grading is never offered before a reveal.
pub fn can_reveal(session: &Session) -> bool {
    !session.finished() && !session.gradeable()
}

/// The study screen over the due queue of `entries`. `on_grade` writes each grade for its card and
/// reports whether the store took it; a refused grade leaves the card answerable.
pub fn study(entries: Vec<Entry>, on_grade: impl Fn(Id, Grade) -> bool + Clone + 'static) -> impl IntoElement {
    let session = stored(Session::new(Queue::Due, entries));
    let version = signal(0_u32);
    let grade_out = on_grade.clone();
    let reveal = move || {
        session.with(|s| {
            s.reveal();
        });
        version.update(|v| *v += 1);
    };
    let grade = move |g: Grade| {
        let sent = session.with(|s| s.grade(g));
        if let Some((id, grade)) = sent {
            let taken = grade_out(id, grade);
            session.with(|s| {
                if taken {
                    s.sent(id as u64);
                    s.acknowledge(id as u64, SystemClock.now_ms());
                } else {
                    s.refuse(id as u64);
                }
            });
        }
        version.update(|v| *v += 1);
    };
    let grade_forgot = grade.clone();
    let grade_remembered = grade;
    view()
        .flex_1()
        .gap_4()
        .p_4()
        .child(dynamic(move || {
            version.get();
            let finished = session.with(|s| s.finished());
            if finished {
                let reviewed = session.with(|s| s.grades.len());
                return badge(format!("Session complete · {reviewed} reviewed")).into_element();
            }
            let lines: Vec<String> = session.with(|s| s.lines().iter().map(|(t, _)| t.to_string()).collect());
            let mut column = view().gap_3();
            for line in lines {
                column = column.child(prose(move || line.clone().into()));
            }
            column.into_element()
        }))
        .child(dynamic(move || {
            version.get();
            let reveal_ok = session.with(|s| can_reveal(s));
            let grading_ok = session.with(|s| s.gradeable());
            let mut row = view().flex_row().gap_2();
            if reveal_ok {
                row = row.child(button(Variant::Primary, "Show answer", || false, reveal));
            }
            if grading_ok {
                let forgot = grade_forgot.clone();
                let remembered = grade_remembered.clone();
                row = row
                    .child(button(Variant::Destructive, "Forgot", || false, move || forgot(Grade::Forgot)))
                    .child(button(Variant::Default, "Remembered", || false, move || remembered(Grade::Remembered)));
            }
            row.into_element()
        }))
        .child(dynamic(move || {
            version.get();
            let left = session.with(|s| s.remaining());
            text(format!("{left} left")).text_xs().text_color(move || tokens::muted_foreground()).into_element()
        }))
}
