//! Small status pieces: a badge for a short state, an empty state that names the next action.

use nsh_core::element::Prop;
use nsh_core::prelude::*;

use super::tokens;

/// A badge's props.
#[derive(Clone, PartialEq, Props)]
pub struct BadgeProps {
    /// The short state or count.
    pub label: Prop<SharedString>,
}

/// A short state or count, in the muted colours.
#[component]
pub fn Badge(props: BadgeProps) -> impl IntoElement {
    view()
        .px_2()
        .rounded(tokens::radius)
        .border_1()
        .border_color(tokens::border)
        .child(
            text(props.label)
                .text_xs()
                .text_color(tokens::muted_foreground),
        )
}

/// An empty state's props.
#[derive(Clone, PartialEq, Props)]
pub struct EmptyProps {
    /// What is missing.
    pub message: Prop<SharedString>,
    /// The action that fills it.
    pub children: Children,
}

/// What a screen shows when it has nothing: what is missing, and the action that fills it.
#[component]
pub fn Empty(props: EmptyProps) -> impl IntoElement {
    view()
        .p_6()
        .gap_3()
        .items_center()
        .child(text(props.message).text_color(tokens::muted_foreground))
        .child(props.children)
}

/// A keycap's props.
#[derive(Clone, PartialEq, Props)]
pub struct KeycapProps {
    /// The chord it names.
    pub chord: Prop<SharedString>,
}

/// A key's name, drawn as a small keycap.
#[component]
pub fn Keycap(props: KeycapProps) -> impl IntoElement {
    view()
        .px_2()
        .rounded(tokens::radius)
        .border_1()
        .border_color(tokens::border)
        .child(
            text(props.chord)
                .text_xs()
                .text_color(tokens::muted_foreground),
        )
}

/// A pulsing placeholder of a known shape while content loads.
pub fn skeleton() -> impl IntoElement {
    view()
        .h(px(16.0))
        .w(px(160.0))
        .rounded(tokens::radius)
        .bg(tokens::hover)
}

/// Reserved for an indeterminate wait: a control that is working, not a decoration.
pub fn spinner_label(label: &'static str) -> impl IntoElement {
    text(label).text_sm().text_color(tokens::muted_foreground)
}
