//! The study session: the asked face of the current card, the reveal, and the two grades. The
//! session's rules (the queue, the order of sides, what a grade schedules) are typr-core's; this
//! screen shows the session and sends each grade out through `on_grade`, which the caller writes.

use nsh_core::prelude::*;
use typr_core::clock::{Clock, SystemClock};
use typr_core::keymap::{Command, Surface};
use typr_core::session::Session;
pub use typr_core::session::{entries_for, entry_for};
use typr_core::store::{Grade, Id};

use crate::screens::keymap::keystroke_of;
use crate::ui::{
    button::{Variant, button},
    prose::prose,
    status::badge,
    tokens,
};

/// The line a refused grade leaves on screen, so a failed write is visible.
pub const REFUSED: &str = "The grade was not saved. Grade again to retry.";

/// Reveals the answer when the face allows it. Grading is never offered before a reveal.
pub fn can_reveal(session: &Session) -> bool {
    !session.finished() && !session.gradeable()
}

/// Why a grade was not written: the store refused it, so the card stays answerable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GradeRefused;

/// The study screen over `session`, which the caller builds for the queue it wants (due, new,
/// reverse or cram). `on_grade` writes each grade for its card; a refusal leaves the card answerable
/// and says so in its error.
pub fn study(
    session: std::rc::Rc<std::cell::RefCell<Session>>,
    on_grade: impl Fn(Id, String, Grade) -> Result<(), GradeRefused> + Clone + 'static,
    on_leave: impl Fn() + Clone + 'static,
    on_settings: impl Fn() + Clone + 'static,
) -> impl IntoElement {
    let version = signal(0_u32);
    let refused = signal(false);
    study_view(
        session,
        version,
        refused,
        move |session_ref, id, key, grade| {
            let taken = on_grade(id, key, grade).is_ok();
            refused.set(!taken);
            if taken {
                session_ref.borrow_mut().sent(id as u64);
                session_ref
                    .borrow_mut()
                    .acknowledge(id as u64, SystemClock.now_ms());
            } else {
                session_ref.borrow_mut().refuse(id as u64);
            }
        },
        on_leave,
        on_settings,
    )
}

/// The study screen driven by an asynchronous store worker: `on_grade` submits the grade and
/// returns the command id (`None` if the worker is closed), and the caller acknowledges or refuses
/// that id on `session` when the worker answers and bumps `version`.
pub fn study_async(
    session: std::rc::Rc<std::cell::RefCell<Session>>,
    version: Signal<u32>,
    refused: Signal<bool>,
    on_grade: impl Fn(Id, String, Grade) -> Option<u64> + Clone + 'static,
    on_leave: impl Fn() + Clone + 'static,
    on_settings: impl Fn() + Clone + 'static,
) -> impl IntoElement {
    study_view(
        session,
        version,
        refused,
        move |session_ref, id, key, grade| match on_grade(id, key, grade) {
            Some(command_id) => {
                refused.set(false);
                session_ref.borrow_mut().sent(command_id);
            }
            None => {
                refused.set(true);
                session_ref.borrow_mut().refuse(0);
            }
        },
        on_leave,
        on_settings,
    )
}

fn study_view(
    session: std::rc::Rc<std::cell::RefCell<Session>>,
    version: Signal<u32>,
    refused: Signal<bool>,
    dispatch_grade: impl Fn(&std::cell::RefCell<Session>, Id, String, Grade) + Clone + 'static,
    on_leave: impl Fn() + Clone + 'static,
    on_settings: impl Fn() + Clone + 'static,
) -> impl IntoElement {
    let session = stored(session);
    let reveal = move || {
        session.with(|s| {
            s.borrow_mut().reveal();
        });
        version.update(|v| *v += 1);
    };
    let grade = move |g: Grade| {
        let sent = session.with(|s| s.borrow_mut().grade(g));
        if let Some((id, key, grade)) = sent {
            session.with(|s| dispatch_grade(s, id, key, grade));
            version.update(|v| *v += 1);
        }
    };
    let grade_forgot = grade.clone();
    let grade_remembered = grade;
    // The keys come from the keymap table, so a rebinding moves them here too.
    if let Some(key) = keystroke_of(Surface::Session, Command::Reveal) {
        on_key(&key, move |_| reveal());
    }
    if let Some(key) = keystroke_of(Surface::Session, Command::GradeForgot) {
        let forgot = grade_forgot.clone();
        on_key(&key, move |_| forgot(Grade::Forgot));
    }
    if let Some(key) = keystroke_of(Surface::Session, Command::GradeRemembered) {
        let remembered = grade_remembered.clone();
        on_key(&key, move |_| remembered(Grade::Remembered));
    }
    if let Some(key) = keystroke_of(Surface::Session, Command::Back) {
        on_key(&key, move |_| {
            session.with(|s| s.borrow_mut().backward());
            version.update(|v| *v += 1);
        });
    }
    if let Some(key) = keystroke_of(Surface::Session, Command::Shuffle) {
        on_key(&key, move |_| {
            let seed = SystemClock.now_ms() as u64;
            session.with(|s| s.borrow_mut().reshuffle(seed));
            version.update(|v| *v += 1);
        });
    }
    if let Some(key) = keystroke_of(Surface::Session, Command::Leave) {
        let leave = on_leave.clone();
        on_key(&key, move |_| leave());
    }
    if let Some(key) = keystroke_of(Surface::Session, Command::Settings) {
        let open = on_settings.clone();
        on_key(&key, move |_| open());
    }
    view()
        .flex_1()
        .gap_4()
        .p_4()
        .child(dynamic(move || {
            version.get();
            let finished = session.with(|s| s.borrow().finished());
            if finished {
                // The summary: what was answered, how it went, and the way back.
                let (reviewed, remembered, forgot) = session.with(|s| {
                    let s = s.borrow();
                    let remembered = s.grades.iter().filter(|g| **g == Grade::Remembered).count();
                    (s.grades.len(), remembered, s.grades.len() - remembered)
                });
                let due = session.with(|s| s.borrow().grades.len() + s.borrow().remaining());
                let leave = on_leave.clone();
                return view()
                    .gap_3()
                    .child(badge().label(format!("Session complete · {reviewed} reviewed")))
                    .child(
                        text(format!(
                            "Remembered {remembered} · Forgot {forgot} · {due} due when you started"
                        ))
                        .text_sm()
                        .text_color(tokens::foreground),
                    )
                    .child(
                        view().flex_row().child(
                            button()
                                .variant(Variant::Default)
                                .label("Back to the library")
                                .disabled(|| false)
                                .on_press(move || leave.clone()()),
                        ),
                    )
                    .into_element();
            }
            let lines: Vec<String> = session.with(|s| {
                s.borrow()
                    .lines()
                    .iter()
                    .map(|(t, _)| t.to_string())
                    .collect()
            });
            let mut column = view().gap_3();
            for line in lines {
                column = column.child(prose().source(move || SharedString::from(line.clone())));
            }
            column.into_element()
        }))
        .child(dynamic(move || {
            version.get();
            let reveal_ok = session.with(|s| can_reveal(&s.borrow()));
            let grading_ok = session.with(|s| s.borrow().gradeable());
            let mut row = view().flex_row().gap_2();
            if reveal_ok {
                row = row.child(
                    button()
                        .variant(Variant::Primary)
                        .label("Show answer")
                        .disabled(|| false)
                        .on_press(reveal),
                );
            }
            if grading_ok {
                let forgot = grade_forgot.clone();
                let remembered = grade_remembered.clone();
                row = row
                    .child(
                        button()
                            .variant(Variant::Destructive)
                            .label("Forgot")
                            .disabled(|| false)
                            .on_press(move || forgot(Grade::Forgot)),
                    )
                    .child(
                        button()
                            .variant(Variant::Default)
                            .label("Remembered")
                            .disabled(|| false)
                            .on_press(move || remembered(Grade::Remembered)),
                    );
            }
            row.into_element()
        }))
        .child(dynamic(move || {
            if refused.get() {
                text(REFUSED)
                    .text_sm()
                    .text_color(tokens::destructive)
                    .into_element()
            } else {
                view().into_element()
            }
        }))
        .child(dynamic(move || {
            version.get();
            let left = session.with(|s| s.borrow().remaining());
            text(format!("{left} left"))
                .text_xs()
                .text_color(tokens::muted_foreground)
                .into_element()
        }))
}
