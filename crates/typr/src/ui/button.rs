//! The app's button: one look per variant, one press feedback, one focus ring, one disabled look.
//!
//! Screens call [`button`] with a variant and a label; they never style a `nsh_base` button.

use nsh_base::button as base_button;
use nsh_core::prelude::*;

use super::{motion, tokens};

/// What a button is for. The variant sets its colours; the size sets its height and padding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Variant {
    /// The one commitment in a decision area.
    Primary,
    /// An ordinary visible action.
    Default,
    /// A quiet toolbar or inline action.
    Ghost,
    /// An action that deletes or cannot be undone.
    Destructive,
}

/// A button, filled with the variant's colours and sized from the scale.
pub fn button(variant: Variant, label: impl IntoReactive<SharedString>, on_press: impl Fn() + 'static) -> impl IntoElement {
    let (fill, text_color) = match variant {
        Variant::Primary => (tokens::selection(), tokens::selection_foreground()),
        Variant::Default => (tokens::card(), tokens::foreground()),
        Variant::Ghost => (tokens::background().opacity(0.0), tokens::foreground()),
        Variant::Destructive => (tokens::destructive(), tokens::selection_foreground()),
    };
    base_button()
        .on_press(on_press)
        .h(px(32.0))
        .px_4()
        .items_center()
        .justify_center()
        .rounded(tokens::radius)
        .bg(move || fill)
        .border_1()
        .border_color(move || tokens::border())
        .active(|s| s.opacity(0.85))
        .focus_visible(|s| s.outline(px(2.0)).outline_color(move || tokens::ring()))
        .disabled_style(|s| s.opacity(0.5))
        .child(text(label).text_sm().text_color(move || text_color))
}

/// Presses take this long to settle back, from the motion tokens.
pub const PRESS_MS: f32 = motion::FAST;
