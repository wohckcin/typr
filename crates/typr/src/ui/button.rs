//! The app's button: one look per variant, one press feedback, one focus ring, one disabled look.
//!
//! Screens place [`button`] with its props and hear its press; they never style a `nsh_base`
//! button.

use nsh_base::button as base_button;
use nsh_core::element::Prop;
use nsh_core::prelude::*;

use super::{
    motion::{self, PRESS_SCALE},
    size::Size,
    tokens,
};

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

/// The interaction state a fill is drawn for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Rest,
    Hover,
    Press,
}

/// A fill by role, before the theme resolves it: what the control paints in a state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fill {
    /// The selection colour, at a percent of its strength.
    Selection(u8),
    /// The card surface.
    Card,
    /// The hover wash, at a percent of its strength.
    Hover(u8),
    /// Nothing at rest.
    Clear,
    /// The destructive colour, at a percent of its strength.
    Destructive(u8),
}

/// A fill's colour in the theme.
fn resolve(fill: Fill) -> AdaptiveColor {
    fn at(color: AdaptiveColor, percent: u8) -> AdaptiveColor {
        color.opacity(f32::from(percent) / 100.0)
    }
    match fill {
        Fill::Selection(percent) => at(tokens::selection(), percent),
        Fill::Card => tokens::card(),
        Fill::Hover(percent) => at(tokens::hover(), percent),
        Fill::Clear => at(tokens::background(), 0),
        Fill::Destructive(percent) => at(tokens::destructive(), percent),
    }
}

impl Variant {
    /// The fill and the text colour the variant draws with at rest.
    fn colors(self) -> (AdaptiveColor, AdaptiveColor) {
        let text = match self {
            Variant::Primary | Variant::Destructive => tokens::selection_foreground(),
            Variant::Default | Variant::Ghost => tokens::foreground(),
        };
        (resolve(self.fill(State::Rest)), text)
    }

    /// What the control paints in `state`: a wash over the rest fill, deeper when pressed.
    fn fill(self, state: State) -> Fill {
        match (self, state) {
            (Variant::Primary, State::Rest) => Fill::Selection(100),
            (Variant::Primary, State::Hover) => Fill::Selection(85),
            (Variant::Primary, State::Press) => Fill::Selection(75),
            (Variant::Destructive, State::Rest) => Fill::Destructive(100),
            (Variant::Destructive, State::Hover) => Fill::Destructive(85),
            (Variant::Destructive, State::Press) => Fill::Destructive(75),
            (Variant::Default, State::Rest) => Fill::Card,
            (Variant::Default, State::Hover) => Fill::Hover(100),
            (Variant::Default, State::Press) => Fill::Hover(90),
            (Variant::Ghost, State::Rest) => Fill::Clear,
            (Variant::Ghost, State::Hover) => Fill::Hover(60),
            (Variant::Ghost, State::Press) => Fill::Hover(90),
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
    let variant = props.variant.get_untracked();
    let (fill, text_color) = variant.colors();
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
        .transition_colors()
        .transition_transform()
        .duration(motion::INSTANT)
        .hover(move |style| style.bg(move || resolve(variant.fill(State::Hover))))
        .active(move |style| {
            if reduce_motion() {
                style.bg(move || resolve(variant.fill(State::Press)))
            } else {
                style
                    .bg(move || resolve(variant.fill(State::Press)))
                    .scale(PRESS_SCALE)
            }
        })
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
        .transition_transform()
        .duration(motion::FAST)
        .hover(|style| style.bg(tokens::hover().opacity(0.6)))
        .active(move |style| {
            if reduce_motion() {
                style.bg(tokens::hover().opacity(0.9))
            } else {
                style.bg(tokens::hover().opacity(0.9)).scale(PRESS_SCALE)
            }
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_variant_paints_a_different_fill_in_each_state() {
        for variant in [
            Variant::Primary,
            Variant::Default,
            Variant::Ghost,
            Variant::Destructive,
        ] {
            let rest = variant.fill(State::Rest);
            let hover = variant.fill(State::Hover);
            let press = variant.fill(State::Press);
            assert_ne!(rest, hover, "{variant:?} changes when hovered");
            assert_ne!(hover, press, "{variant:?} deepens when pressed");
            assert_ne!(rest, press, "{variant:?} differs from rest when pressed");
        }
    }

    #[test]
    fn the_press_fill_is_mapped_without_motion() {
        // Reduced motion skips the scale only, so the fills stay apart whatever the setting says.
        for variant in [Variant::Primary, Variant::Default, Variant::Ghost] {
            assert_ne!(
                variant.fill(State::Hover),
                variant.fill(State::Press),
                "{variant:?} keeps hover and press apart"
            );
        }
    }
}
