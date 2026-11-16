//! The app's button: one look per variant, one press feedback, one focus ring, one disabled look.
//!
//! Screens place [`button`] with its props and hear its press; they never style a `nsh_base`
//! button.

use nsh_base::button as base_button;
use nsh_core::element::Prop;
use nsh_core::prelude::*;

use super::{motion, size::Size, tokens};

/// What a button is for. The variant sets its colours.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PropEnum)]
pub enum Variant {
    /// The one commitment in a decision area.
    Primary,
    /// An ordinary visible action.
    #[default]
    Default,
    /// A quiet toolbar or inline action.
    Ghost,
    /// An action that deletes or cannot be undone.
    Destructive,
}

impl Variant {
    /// The fill and the text colour the variant draws with.
    fn colors(self) -> (AdaptiveColor, AdaptiveColor) {
        match self {
            Variant::Primary => (tokens::selection(), tokens::selection_foreground()),
            Variant::Default => (tokens::card(), tokens::foreground()),
            Variant::Ghost => (tokens::background().opacity(0.0), tokens::foreground()),
            Variant::Destructive => (tokens::destructive(), tokens::selection_foreground()),
        }
    }
}

/// A button's props.
#[derive(Clone, PartialEq, Props)]
pub struct ButtonProps {
    /// What it is for: the variant sets its colours.
    #[prop(default)]
    pub variant: Prop<Variant>,
    /// The words on it.
    pub label: Prop<SharedString>,
    /// The name a screen reader says, when the visible mark is not the name: an icon-only control
    /// says itself while it draws a glyph.
    #[prop(default)]
    pub accessible: Prop<Option<SharedString>>,
    /// Whether a press does nothing.
    #[prop(default)]
    pub disabled: Prop<bool>,
    /// How large it is: height, padding, text and radius together.
    #[prop(default)]
    pub size: Prop<Size>,
}

/// What a button reports.
#[derive(Event)]
pub enum ButtonEvent {
    /// Pressed.
    Press,
}

/// A button, filled with the variant's colours and sized from the scale.
#[component]
pub fn Button(props: ButtonProps, cx: Scope<ButtonEvent>) -> impl IntoElement {
    let (fill, text_color) = props.variant.get_untracked().colors();
    let sized = props.size.get_untracked();
    base_button()
        .disabled(move || props.disabled.get())
        .on_press(move || cx.emit(ButtonEvent::Press))
        .accessibility_label(move || props.accessible.get())
        .min_h(sized.min_height())
        .px(sized.pad_x())
        .py(sized.pad_y())
        .items_center()
        .justify_center()
        .rounded(sized.radius())
        .bg(move || fill)
        .border_1()
        .border_color(tokens::border)
        .active(|style| style.opacity(0.85))
        .focus_visible(|style| style.outline(px(2.0)).outline_color(tokens::ring))
        .disabled_style(|style| style.opacity(0.5))
        .child(sized.text(props.label).text_color(move || text_color))
}

/// A press target drawn as its own children, for a card that is more than a label: a grid tile
/// opens its card, and its name is what a screen reader says instead of its whole content.
#[derive(Clone, PartialEq, Props)]
pub struct TileProps {
    /// What a screen reader says for it.
    pub label: Prop<SharedString>,
    /// What it draws.
    pub children: Children,
    /// How large it is: its radius follows the size.
    #[prop(default)]
    pub size: Prop<Size>,
}

/// What a tile reports.
#[derive(Event)]
pub enum TileEvent {
    /// Pressed.
    Press,
}

/// A card's tile: its own drawing, its own name for a screen reader.
#[component]
pub fn Tile(props: TileProps, cx: Scope<TileEvent>) -> impl IntoElement {
    let sized = props.size.get_untracked();
    base_button()
        .on_press(move || cx.emit(TileEvent::Press))
        .accessibility_label(move || Some(props.label.get()))
        .rounded(sized.radius())
        .active(|style| style.opacity(0.9))
        .focus_visible(|style| style.outline(px(2.0)).outline_color(tokens::ring))
        .child(props.children)
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
    button()
        .variant(variant)
        .label(move || -> SharedString {
            if busy() {
                "Working…".into()
            } else {
                idle.into()
            }
        })
        .disabled(busy)
        .on_press(on_press)
}
