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
pub fn button(
    variant: Variant,
    label: impl IntoReactive<SharedString>,
    disabled: impl Fn() -> bool + 'static,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let (fill, text_color) = match variant {
        Variant::Primary => (tokens::selection(), tokens::selection_foreground()),
        Variant::Default => (tokens::card(), tokens::foreground()),
        Variant::Ghost => (tokens::background().opacity(0.0), tokens::foreground()),
        Variant::Destructive => (tokens::destructive(), tokens::selection_foreground()),
    };
    base_button()
        .disabled(disabled)
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

/// A button for work in flight: while `busy`, it is disabled and its label says so.
///
/// The label reads "Working…" in place of `idle`, so the button keeps its size and the user sees
/// why a second press does nothing.
pub fn busy_button(
    variant: Variant,
    idle: &'static str,
    busy: impl Fn() -> bool + Copy + 'static,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let label = move || -> SharedString { if busy() { "Working…".into() } else { idle.into() } };
    button(variant, label, busy, on_press)
}
