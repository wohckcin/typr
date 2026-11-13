//! The study screen: the side in front of the user, the keys that answer it, and the summary
//! that ends a session.

use rune::tok::{self, type_};
use rune::{El, Key, KeyEvent, ScrollSpec, Tx, col, row, text};
use rune_kit::{Button, ButtonVariant, Markdown, MarkdownStyle, root};

use crate::session::Session;
use crate::store::{Grade, Id};

/// What the study screen needs from the app.
pub trait Host: 'static {
    fn session(&self) -> &Session;
    fn session_mut(&mut self) -> &mut Session;
    /// Send the grade to the store and answer the id it was sent under.
    fn submit_grade(&mut self, card: Id, grade: Grade) -> u64;
    /// How many cards are due right now, for the summary.
    fn due_now(&self) -> usize;
    /// Leave the session and go back to the library.
    fn leave_session(&mut self);
}

/// The answers a session offers, and the keys that give them.
pub const BINDINGS: &[(&str, &str)] = &[
    ("Space", "Reveal"),
    ("J", "Remembered"),
    ("F", "Forgot"),
    ("Right", "Forward"),
    ("Left", "Back"),
    ("Esc", "Library"),
];

fn answer<S: Host>(s: &mut S, grade: Grade) {
    if let Some((card, grade)) = s.session_mut().grade(grade) {
        let id = s.submit_grade(card, grade);
        s.session_mut().sent(id);
    }
}

/// The session's keyboard. Answers whether the key was used.
pub fn key<S: Host>(s: &mut S, event: &KeyEvent) -> bool {
    if event.mods.cmd || event.mods.ctrl {
        return false;
    }
    match &event.key {
        Key::Text(t) if t == " " => s.session_mut().reveal(),
        Key::Enter => s.session_mut().reveal(),
        Key::Escape => {
            s.leave_session();
            true
        }
        Key::Right => s.session_mut().forward(),
        Key::Left => s.session_mut().backward(),
        Key::Text(t) => match t.to_ascii_lowercase().as_str() {
            "j" => {
                answer(s, Grade::Remembered);
                true
            }
            "f" => {
                answer(s, Grade::Forgot);
                true
            }
            _ => false,
        },
        _ => false,
    }
}

fn bindings<S: Host>() -> El {
    let t = tok::theme(true);
    row()
        .key("bindings")
        .gap(12.0)
        .children(BINDINGS.iter().map(|(key, action)| {
            text(format!("{key}  {action}")).font(type_::LABEL).color(t.text_secondary)
        }))
        .gap(12.0)
}

fn side_heading<S: Host>(s: &S) -> String {
    let session = s.session();
    let sides = session.current().map_or(0, |entry| entry.sides.len());
    match (session.read_only, session.revealed()) {
        (true, _) => format!("Answer · {sides} sides · you have answered this card"),
        (_, revealed) => format!("Side {revealed} of {sides}"),
    }
}

/// One side of the current card, rendered, with what is still hidden.
pub fn view<S: Host>(s: &S) -> El {
    let t = tok::theme(true);
    let session = s.session();
    let entry = session.current();
    let shown: Vec<String> = entry
        .map(|entry| entry.sides.iter().take(session.revealed()).cloned().collect())
        .unwrap_or_default();
    let body = shown
        .iter()
        .map(|side| Markdown::new(side.clone()).render(&MarkdownStyle::neutral(true)));

    let mut card = col().key("session-card").grow(1.0).w_full().gap(16.0).scroll(ScrollSpec { stick_bottom: false });
    for el in body {
        card = card.child(el);
    }
    if session.hidden() > 0 {
        card = card.child(
            text(format!("{} side(s) still hidden", session.hidden()))
                .key("hidden")
                .color(t.text_secondary)
                .font(type_::LABEL),
        );
    }

    let mut answers = row().key("answers").gap(8.0).justify_end();
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
        .child(text(side_heading::<S>(s)).font(type_::LABEL).color(t.text_secondary))
        .child(card)
        .child(answers)
        .child(bindings::<S>())
        .on_key(|s: &mut S, event: &KeyEvent, _: &mut Tx| key::<S>(s, event));
    root(body, Vec::new())
}

/// What the session came to: the answers given, and what is due now.
pub fn summary<S: Host>(s: &S) -> El {
    let t = tok::theme(true);
    let session = s.session();
    let remembered = session.grades.iter().filter(|g| **g == Grade::Remembered).count();
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
            text(format!("{} due now", s.due_now())).key("summary-due").color(t.text_secondary),
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
