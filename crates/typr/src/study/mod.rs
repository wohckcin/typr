//! The study screen: the side in front of the user, the keys that answer it, and the summary
//! that ends a session.

use std::rc::Rc;

use rune::tok::{self, type_};
use rune::{El, Key, KeyEvent, ScrollSpec, TextEdit, Tx, col, row, text, text_input};
use rune_kit::{Button, ButtonVariant, Markdown, MarkdownStyle, root};

use typr_core::keymap::{Command, Surface};
use typr_core::session::{Face, Mode, Session};
use typr_core::store::{Grade, Id, StoreError};

/// What the study screen needs from the app.
pub trait Host: 'static {
    fn session(&self) -> &Session;
    fn session_mut(&mut self) -> &mut Session;
    /// Send the grade to the store and answer the id it was sent under. The key names the prompt
    /// being answered: a card may hold a forward prompt, a reverse one, or a group apiece.
    ///
    /// A cram run sends nothing, because it writes nothing: the grade is taken by the session and
    /// `0` comes back, which is the id of no command and acknowledges nothing.
    fn submit_grade(&mut self, card: Id, key: &str, grade: Grade) -> Result<u64, StoreError>;
    /// How many cards are due right now, for the summary.
    fn due_now(&self) -> usize;
    /// Leave the session and go back to the library.
    fn leave_session(&mut self);

    /// Reorder the cards still to answer, without touching any schedule.
    fn reshuffle(&mut self);

    /// Whether the window has room for a keycap beside a control's label.
    fn keycaps(&self) -> bool {
        false
    }

    /// The picture a diagram card asks about, resolved from its name under the attachments folder.
    fn picture(&self, _src: &str) -> Option<rune::Image> {
        None
    }

    /// The text of the card `name` names, for an embed in the reviewed text to draw. A host
    /// without a library draws the name as one the library does not hold.
    fn embed(&self, _name: &str) -> Option<String> {
        None
    }

    /// The size the picture pane had when it was last laid out, in points. Covers are placed
    /// against it, so it is only known once the frame has been laid out.
    fn picture_pane(&self) -> (f32, f32) {
        (0.0, 0.0)
    }

    /// Record the size the picture pane was laid out at.
    fn set_picture_pane(&mut self, _width: f32, _height: f32) {}

    /// The name the summary offers for the cram run's filter.
    fn cram_name(&self) -> &TextEdit;

    /// That name, for the input that edits it.
    fn cram_name_field(&mut self) -> &mut TextEdit;

    /// Whether the run's filter is already saved, or on its way to being saved.
    fn cram_saved(&self) -> bool;

    /// Save the run's filter as a view, once.
    fn save_cram_view(&mut self);
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

/// A control for a command, drawing its keycap when the window has room for one.
fn with_key<S: Host>(s: &S, command: Command, label: &str, variant: ButtonVariant) -> Button {
    let mut button = Button::new(label).variant(variant);
    if let Some(chord) = crate::keymap::chord(Surface::Session, command)
        && s.keycaps()
    {
        button = button.shortcut(chord.to_string());
    }
    button
}

/// Give the card on screen the grade the review offers.
pub fn answer<S: Host>(s: &mut S, grade: Grade) {
    let Some((card, grade)) = s.session_mut().grade(grade) else {
        return;
    };
    // The prompt is the one on screen when the grade was chosen; a session that moved on before the
    // send still grades the prompt the user actually answered.
    let key = s.session().prompt_of(card).unwrap_or("fwd").to_string();
    match s.submit_grade(card, &key, grade) {
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
    let heading = match (session.read_only, masked, session.revealed()) {
        (true, _, _) => format!("Answer · {faces} face(s) · you have answered this card"),
        (_, true, _) => "Hidden text".to_string(),
        (_, false, revealed) => format!("Side {revealed} of {faces}"),
    };
    // Which way the card is being asked is part of the question, not a decoration: "Cell division"
    // as a prompt means something different from the same card asked forward.
    match session.direction() {
        "reverse" => format!("Reverse · {heading}"),
        _ => heading,
    }
}

/// One side of the current card, rendered, with what is still hidden.
/// The picture of a diagram face, with the group being asked covered until the answer is revealed.
///
/// The covers are placed against the size the pane had in the last frame, because an image's drawn
/// size is only known once it has been laid out. The first frame therefore draws the picture without
/// covers and the second draws them, which is the same one-frame settle the grid uses for its
/// column count.
fn picture_face<S: Host>(s: &S, face: &Face) -> Option<El> {
    let t = tok::theme(true);
    let (image, caption, masks) = face.picture()?;
    let unmasked = s.session().revealed() > 0 || !face.unmasks();
    let (pane_w, pane_h) = s.picture_pane();

    // The picture box is measured on its own, because a cover is a fraction of the picture and the
    // caption below it is not part of that fraction.
    let mut picture_box = col()
        .key("picture-box")
        .w_full()
        .on_layout(|s: &mut S, rect| s.set_picture_pane(rect[2], rect[3]));
    let mut pane = col().key("diagram-face").w_full().gap(6.0);

    match s.picture(image) {
        Some(picture) => {
            let mut drawn = rune::el().role("image").w_full().padding(0.0);
            if let Some([w, _]) = picture.size() {
                drawn = drawn.max_w(w as f32);
            }
            picture_box = picture_box.child(drawn.pixels(picture.pixels()));
        }
        None => {
            // A picture that cannot be read still gets its pane, so the cover has somewhere to sit
            // and the card says what it wanted to show.
            picture_box = picture_box.child(
                col()
                    .w_full()
                    .h(160.0)
                    .radius(4.0)
                    .fill(t.text_secondary.alpha(0.08))
                    .center()
                    .child(
                        text(format!("The picture {image} is missing"))
                            .key("diagram-missing")
                            .color(t.text_secondary)
                            .font(type_::LABEL),
                    ),
            );
        }
    }

    if !unmasked {
        // One cover per rectangle of the group being asked, in the pane's own points. A rectangle
        // outside the pane is clamped rather than drawn over the edge.
        for (index, mask) in masks.iter().enumerate() {
            let left = (mask.x * pane_w).clamp(0.0, pane_w);
            let top = (mask.y * pane_h).clamp(0.0, pane_h);
            let width = (mask.w * pane_w).min(pane_w - left);
            let height = (mask.h * pane_h).min(pane_h - top);
            picture_box = picture_box.child(
                col()
                    .key(format!("cover-{index}"))
                    .absolute(left, top)
                    .size(width, height)
                    .radius(2.0)
                    .fill(t.card)
                    .border(1.0, t.text_secondary.alpha(0.35)),
            );
        }
    }

    pane = pane.child(picture_box);
    if let Some(caption) = caption {
        pane = pane.child(
            text(caption.to_owned())
                .key("diagram-caption")
                .color(t.text_secondary)
                .font(type_::LABEL),
        );
    }
    Some(pane)
}

pub fn view<S: Host>(s: &S) -> El {
    let t = tok::theme(true);
    let session = s.session();
    let body = session.lines().into_iter().map(|(line, _)| {
        let embeds = Rc::new(crate::embed::Embeds::of(line, |name| s.embed(name)));
        let drawn = crate::embed::expanded(line, &embeds);
        Markdown::new(drawn)
            .code_frame(move |card, frame| crate::embed::frame(card, frame, &embeds))
            .render(&MarkdownStyle::neutral(true))
    });

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
    if let Some(picture) = session.asking().and_then(|face| picture_face(s, face)) {
        card = card.child(picture);
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
        with_key(s, Command::Shuffle, "Shuffle", ButtonVariant::Ghost)
            .on_press(|s: &mut S| s.reshuffle())
            .build(&rune_kit::Kit::neutral(true))
            .key("shuffle"),
    );
    if session.gradeable() {
        answers = answers
            .child(
                with_key(s, Command::GradeForgot, "Forgot", ButtonVariant::Ghost)
                    .on_press(|s: &mut S| answer(s, Grade::Forgot))
                    .build(&rune_kit::Kit::neutral(true))
                    .key("forgot"),
            )
            .child(
                with_key(
                    s,
                    Command::GradeRemembered,
                    "Remembered",
                    ButtonVariant::Submit,
                )
                .on_press(|s: &mut S| answer(s, Grade::Remembered))
                .build(&rune_kit::Kit::neutral(true))
                .key("remembered"),
            );
    }

    let mut body = col()
        .w_full()
        .h_full()
        .fill(t.bg)
        .padding(24.0)
        .gap(16.0)
        .child(
            text(side_heading::<S>(s))
                .font(type_::LABEL)
                .color(t.text_secondary),
        );
    // A cram run looks like any other session, so the screen has to say that its answers go
    // nowhere: the reader would otherwise expect a due date to move.
    if session.mode() == Mode::Cram {
        body = body.child(
            text("Cram · answers here are not scheduled".to_string())
                .key("cram-note")
                .font(type_::LABEL)
                .color(t.text_secondary),
        );
    }
    let body = body
        .child(card)
        .child(answers)
        .child(bindings())
        .on_key(|s: &mut S, event: &KeyEvent, _: &mut Tx| key::<S>(s, event));
    root(body, Vec::new())
}

/// What the session came to: the answers given, and what is due now.
///
/// A cram run ends differently: it scheduled nothing, so there is no due date to report, and its
/// filter is worth keeping, so the summary offers to save it as a view.
pub fn summary<S: Host>(s: &S) -> El {
    let t = tok::theme(true);
    let session = s.session();
    let cram = session.mode() == Mode::Cram;
    let remembered = session
        .grades
        .iter()
        .filter(|g| **g == Grade::Remembered)
        .count();
    let forgot = session.grades.len() - remembered;
    let mut body = col()
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
        );
    if cram {
        body = body.child(
            text(format!(
                "{} answer(s) · nothing was scheduled",
                session.grades.len()
            ))
            .key("summary-scheduled")
            .color(t.text_secondary),
        );
        body = body.child(
            row()
                .key("save-cram")
                .w_full()
                .gap(8.0)
                .child(
                    text_input(s.cram_name(), type_::LABEL)
                        .key("cram-name")
                        .single_line()
                        .grow(1.0)
                        .placeholder("Name this view", t.text_secondary)
                        .on_edit(|s: &mut S| s.cram_name_field())
                        .on_submit(|s: &mut S, _| s.save_cram_view()),
                )
                .child(
                    Button::new(if s.cram_saved() {
                        "Saved"
                    } else {
                        "Save as view"
                    })
                    .variant(ButtonVariant::Ghost)
                    .on_press(|s: &mut S| s.save_cram_view())
                    .build(&rune_kit::Kit::neutral(true))
                    .key("save-cram-view"),
                ),
        );
    } else {
        body = body.child(
            text(format!("{} due now", s.due_now()))
                .key("summary-due")
                .color(t.text_secondary),
        );
    }
    body = body
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
