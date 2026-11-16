//! The study session: the asked face of the current card, the reveal, and the two grades. The
//! session's rules (the queue, the order of sides, what a grade schedules) are typr-core's; this
//! screen shows the session and sends each grade out through `on_grade`, which the caller writes.

use nsh_core::prelude::*;
use typr_core::clock::{Clock, SystemClock};
use typr_core::keymap::{Command, Surface};
use typr_core::session::{Entry, Session};
use typr_core::store::{CardSummary, Grade, Id};

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

/// The study screen over `session`, which the caller builds for the queue it wants (due, new,
/// reverse or cram). `on_grade` writes each grade for its card and
/// reports whether the store took it; a refused grade leaves the card answerable.
/// The entry one prompt walks: a hidden-text group masked until revealed, a diagram group covered
/// until uncovered, or the sides of a card asked forwards or backwards. `None` for a key the card
/// no longer holds (a prompt retired after an edit).
pub fn entry_for(card: &CardSummary, key: &str, overdue: i64) -> Option<Entry> {
    if let Some(group) = key.strip_prefix("mask:") {
        let group: u32 = group.parse().ok()?;
        let masks: Vec<typr_core::store::MaskRect> = card
            .masks
            .iter()
            .filter(|row| row.group == group)
            .map(|row| row.rect)
            .collect();
        if masks.is_empty() {
            return None;
        }
        return Some(Entry::mask_group(
            card.id,
            key,
            card.image.clone().unwrap_or_default(),
            card.caption.clone(),
            masks,
            overdue,
        ));
    }
    let shape = typr_core::hidden::shape(&card.markdown);
    match key {
        "fwd" => {
            let sides = typr_core::sides::split(&card.markdown);
            (sides.len() >= 2).then(|| Entry::sides(card.id, sides, overdue))
        }
        "rev" => {
            let sides = typr_core::sides::split(&card.markdown);
            (sides.len() >= 2).then(|| Entry::reverse(card.id, sides, overdue))
        }
        other if typr_core::hidden::group_of(other).is_some() => {
            let hidden = typr_core::hidden::masked(&card.markdown, &shape, other);
            let shown = typr_core::hidden::shown(&card.markdown, &shape);
            (hidden != shown).then(|| Entry::masked(card.id, other, hidden, shown, overdue))
        }
        _ => None,
    }
}

/// The entries a session walks, from the prompts the store served and the cards it holds. A prompt
/// whose card was edited away is left out rather than asked with nothing on it.
pub fn entries_for(cards: &[CardSummary], prompts: &[(Id, String, i64)]) -> Vec<Entry> {
    prompts
        .iter()
        .filter_map(|(card, key, overdue)| {
            let card = cards.iter().find(|row| row.id == *card)?;
            entry_for(card, key, *overdue)
        })
        .collect()
}

pub fn study(
    session: std::rc::Rc<std::cell::RefCell<Session>>,
    on_grade: impl Fn(Id, String, Grade) -> bool + Clone + 'static,
    on_leave: impl Fn() + Clone + 'static,
    on_settings: impl Fn() + Clone + 'static,
) -> impl IntoElement {
    let session = stored(session);
    let version = signal(0_u32);
    let refused = signal(false);
    let grade_out = on_grade.clone();
    let reveal = move || {
        session.with(|s| {
            s.borrow_mut().reveal();
        });
        version.update(|v| *v += 1);
    };
    let grade = move |g: Grade| {
        let sent = session.with(|s| s.borrow_mut().grade(g));
        if let Some((id, grade)) = sent {
            let key = session
                .with(|s| s.borrow().prompt_of(id).map(str::to_owned))
                .unwrap_or_default();
            let taken = grade_out(id, key, grade);
            refused.set(!taken);
            session.with(|s| {
                if taken {
                    s.borrow_mut().sent(id as u64);
                    s.borrow_mut().acknowledge(id as u64, SystemClock.now_ms());
                } else {
                    s.borrow_mut().refuse(id as u64);
                }
            });
        }
        version.update(|v| *v += 1);
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
                    .child(badge(format!("Session complete · {reviewed} reviewed")))
                    .child(
                        text(format!(
                            "Remembered {remembered} · Forgot {forgot} · {due} due when you started"
                        ))
                        .text_sm()
                        .text_color(tokens::foreground),
                    )
                    .child(view().flex_row().child(button(
                        Variant::Default,
                        "Back to the library",
                        || false,
                        move || leave.clone()(),
                    )))
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
                column = column.child(prose(move || line.clone().into()));
            }
            column.into_element()
        }))
        .child(dynamic(move || {
            version.get();
            let reveal_ok = session.with(|s| can_reveal(&s.borrow()));
            let grading_ok = session.with(|s| s.borrow().gradeable());
            let mut row = view().flex_row().gap_2();
            if reveal_ok {
                row = row.child(button(Variant::Primary, "Show answer", || false, reveal));
            }
            if grading_ok {
                let forgot = grade_forgot.clone();
                let remembered = grade_remembered.clone();
                row = row
                    .child(button(
                        Variant::Destructive,
                        "Forgot",
                        || false,
                        move || forgot(Grade::Forgot),
                    ))
                    .child(button(
                        Variant::Default,
                        "Remembered",
                        || false,
                        move || remembered(Grade::Remembered),
                    ));
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
