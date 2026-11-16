//! The two questions the editor asks: whether to save before leaving, and whether to clear a
//! diagram.

use nsh_core::prelude::*;

use crate::ui::{
    button::{Variant, button},
    overlay::decision,
    tokens,
};

/// What leaving a card with unsaved edits does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Leaving {
    /// Nothing to lose: leave.
    Leave,
    /// Ask first.
    Ask,
}

/// The user's answer when asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    Save,
    Discard,
}

/// Whether leaving needs the user's answer first.
pub fn leaving_choice(dirty: bool) -> Leaving {
    if dirty { Leaving::Ask } else { Leaving::Leave }
}

/// The question before a diagram is cleared: every rectangle is thrown away and the card becomes
/// empty text.
pub fn clear_question(
    open: Signal<bool>,
    on_clear: impl Fn() + Clone + 'static,
) -> impl IntoElement {
    let keep = open;
    let clear = on_clear;
    let asked = keep;
    let confirmed = clear;
    decision(open, move || {
        let asked = asked;
        let confirmed = confirmed.clone();
        view()
            .gap_3()
            .child(
                text("Clear this diagram?")
                    .text_sm()
                    .text_color(tokens::foreground),
            )
            .child(
                text("Its picture and every rectangle go; it is edited as text afterwards.")
                    .text_xs()
                    .text_color(tokens::muted_foreground),
            )
            .child(
                view()
                    .flex_row()
                    .gap_2()
                    .child(
                        button()
                            .variant(Variant::Destructive)
                            .label("Clear")
                            .disabled(|| false)
                            .on_press(move || {
                                confirmed.clone()();
                                asked.set(false);
                            }),
                    )
                    .child(
                        button()
                            .variant(Variant::Default)
                            .label("Stay")
                            .disabled(|| false)
                            .on_press(move || asked.set(false)),
                    ),
            )
    })
}

/// The question a dirty editor asks: save, discard, or stay. `open` false hides it; Stay closes it.
pub fn leave_decision(
    open: Signal<bool>,
    on_answer: impl Fn(Answer) + Clone + 'static,
) -> impl IntoElement {
    let save = on_answer.clone();
    let discard = on_answer;
    decision(open, move || {
        let stay = open;
        let save = save.clone();
        let discard = discard.clone();
        view()
            .gap_3()
            .child(
                text("Save this card before leaving?")
                    .text_sm()
                    .text_color(tokens::foreground),
            )
            .child(
                view()
                    .flex_row()
                    .gap_2()
                    .child(
                        button()
                            .variant(Variant::Primary)
                            .label("Save and leave")
                            .disabled(|| false)
                            .on_press(move || {
                                save(Answer::Save);
                                stay.set(false);
                            }),
                    )
                    .child(
                        button()
                            .variant(Variant::Default)
                            .label("Discard")
                            .disabled(|| false)
                            .on_press(move || {
                                discard(Answer::Discard);
                                stay.set(false);
                            }),
                    )
                    .child(
                        button()
                            .variant(Variant::Ghost)
                            .label("Stay")
                            .disabled(|| false)
                            .on_press(move || stay.set(false)),
                    ),
            )
    })
}
