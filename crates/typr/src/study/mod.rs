//! The study screen: the side in front of the user, the keys that answer it, and the summary
//! that ends a session.

use rune::tok::{self, type_};
use rune::{El, Key, KeyEvent, ScrollSpec, Tx, col, row, text};
use rune_kit::{Button, ButtonVariant, Markdown, MarkdownStyle, root};

use typr_core::keymap::{Command, Surface};
use typr_core::session::{Face, Session};
use typr_core::store::{Grade, Id, StoreError};

/// What the study screen needs from the app.
pub trait Host: 'static {
    fn session(&self) -> &Session;
    fn session_mut(&mut self) -> &mut Session;
    /// Send the grade to the store and answer the id it was sent under.
    fn submit_grade(&mut self, card: Id, grade: Grade) -> Result<u64, StoreError>;
    /// How many cards are due right now, for the summary.
    fn due_now(&self) -> usize;
    /// Leave the session and go back to the library.
    fn leave_session(&mut self);

    /// Reorder the cards still to answer, without touching any schedule.
    fn reshuffle(&mut self);
}

/// Run a command the table bound to the session surface. Answers whether it was used.
pub fn run<S: Host>(s: &mut S, command: Command) -> bool {
    match command {
        Command::Reveal => s.session_mut().reveal(),
        Command::GradeRemembered => {
            answer(s, Grade::Remembered);
            true
        }
        Command::GradeForgot => {
            answer(s, Grade::Forgot);
            true
        }
        Command::Forward => s.session_mut().forward(),
        Command::Back => s.session_mut().backward(),
        Command::Shuffle => {
            s.reshuffle();
            true
        }
        Command::Leave => {
            s.leave_session();
            true
        }
        _ => false,
    }
}

/// The session's keyboard: the table names the command, [`run`] carries it out.
pub fn key<S: Host>(s: &mut S, event: &KeyEvent) -> bool {
    match crate::keymap::command_for(Surface::Session, event) {
        Some(command) => run(s, command),
        None => false,
    }
}

/// Give the card on screen the grade the review offers.
pub fn answer<S: Host>(s: &mut S, grade: Grade) {
    let Some((card, grade)) = s.session_mut().grade(grade) else {
        return;
    };
    match s.submit_grade(card, grade) {
        Ok(id) => s.session_mut().sent(id),
        Err(_) => s.session_mut().unsent(),
    }
}

fn bindings() -> El {
    let t = tok::theme(true);
    row()
        .key("bindings")
        .gap(12.0)
        .children(
            typr_core::keymap::bindings_of(Surface::Session).map(|binding| {
                text(format!("{}  {}", binding.chord, binding.command.label()))
                    .font(type_::LABEL)
                    .color(t.text_secondary)
            }),
        )
        .gap(12.0)
}

fn side_heading<S: Host>(s: &S) -> String {
    let session = s.session();
    let faces = session.current().map_or(0, |entry| entry.faces.len());
    let masked = session
        .current()
        .is_some_and(|entry| entry.faces.iter().any(Face::unmasks));
    match (session.read_only, masked, session.revealed()) {
        (true, _, _) => format!("Answer · {faces} face(s) · you have answered this card"),
        (_, true, _) => "Hidden text".to_string(),
        (_, false, revealed) => format!("Side {revealed} of {faces}"),
    }
}

/// One side of the current card, rendered, with what is still hidden.
pub fn view<S: Host>(s: &S) -> El {
    let t = tok::theme(true);
    let session = s.session();
    let body = session
        .lines()
        .into_iter()
        .map(|(line, _)| Markdown::new(line).render(&MarkdownStyle::neutral(true)));

    let mut card = col()
        .key("session-card")
        .grow(1.0)
        .w_full()
        .gap(16.0)
        .scroll(ScrollSpec {
            stick_bottom: false,
        });
    for el in body {
        card = card.child(el);
    }
    if session.hidden() > 0 {
        card = card.child(
            text(format!("{} face(s) still hidden", session.hidden()))
                .key("hidden")
                .color(t.text_secondary)
                .font(type_::LABEL),
        );
    }

    let mut answers = row().key("answers").gap(8.0).justify_end().child(
        Button::new("Shuffle")
            .variant(ButtonVariant::Ghost)
            .on_press(|s: &mut S| s.reshuffle())
            .build(&rune_kit::Kit::neutral(true))
            .key("shuffle"),
    );
    if session.gradeable() {
        answers = answers
            .child(
                Button::new("Forgot")
                    .variant(ButtonVariant::Ghost)
                    .on_press(|s: &mut S| answer(s, Grade::Forgot))
                    .build(&rune_kit::Kit::neutral(true))
                    .key("forgot"),
            )
            .child(
                Button::new("Remembered")
                    .variant(ButtonVariant::Submit)
                    .on_press(|s: &mut S| answer(s, Grade::Remembered))
                    .build(&rune_kit::Kit::neutral(true))
                    .key("remembered"),
            );
    }

    let body = col()
        .w_full()
        .h_full()
        .fill(t.bg)
        .padding(24.0)
        .gap(16.0)
        .child(
            text(side_heading::<S>(s))
                .font(type_::LABEL)
                .color(t.text_secondary),
        )
        .child(card)
        .child(answers)
        .child(bindings())
        .on_key(|s: &mut S, event: &KeyEvent, _: &mut Tx| key::<S>(s, event));
    root(body, Vec::new())
}

/// What the session came to: the answers given, and what is due now.
pub fn summary<S: Host>(s: &S) -> El {
    let t = tok::theme(true);
    let session = s.session();
    let remembered = session
        .grades
        .iter()
        .filter(|g| **g == Grade::Remembered)
        .count();
    let forgot = session.grades.len() - remembered;
    let body = col()
        .key("summary")
        .w_full()
        .h_full()
        .fill(t.bg)
        .padding(24.0)
        .gap(12.0)
        .child(text("Session finished").font(type_::TITLE).color(t.text))
        .child(
            text(format!("{remembered} remembered · {forgot} forgot"))
                .key("summary-grades")
                .color(t.text_secondary),
        )
        .child(
            text(format!("{} due now", s.due_now()))
                .key("summary-due")
                .color(t.text_secondary),
        )
        .child(
            Button::new("Back to library")
                .variant(ButtonVariant::Neutral)
                .on_press(|s: &mut S| s.leave_session())
                .build(&rune_kit::Kit::neutral(true))
                .key("back-to-library"),
        )
        .on_key(|s: &mut S, event: &KeyEvent, _: &mut Tx| {
            if event.key == Key::Escape {
                s.leave_session();
                true
            } else {
                false
            }
        });
    root(body, Vec::new())
}

#[cfg(test)]
mod tests;
