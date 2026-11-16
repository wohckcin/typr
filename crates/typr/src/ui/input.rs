//! The app's single-line text field: one border, one focus ring, the same padding as its label.

use nsh_base::input;
use nsh_core::element::Prop;
use nsh_core::prelude::*;

use super::tokens;

/// A text field's props.
#[derive(Clone, PartialEq, Props)]
pub struct FieldProps {
    /// The words shown while it is empty.
    pub placeholder: Prop<SharedString>,
    /// The text in it.
    #[prop(default)]
    pub value: Prop<SharedString>,
}

/// What a field reports.
#[derive(Event)]
pub enum FieldEvent {
    /// The whole text, after an edit.
    Change(String),
}

/// A text field bound to `value`; typing reports the whole text once per edit.
#[component]
pub fn Field(props: FieldProps, cx: Scope<FieldEvent>) -> impl IntoElement {
    input()
        .value(move || props.value.get().to_string())
        .on_change(move |text: &str| cx.emit(FieldEvent::Change(text.to_owned())))
        .placeholder(move || props.placeholder.get().to_string())
        .text_color(move || color(tokens::foreground()))
        .placeholder_color(move || color(tokens::muted_foreground()))
        .caret_color(move || color(tokens::foreground()))
        .selection_color(move || color(tokens::selection()))
        .h(px(32.0))
        .px_3()
        .text_sm()
        .rounded(tokens::radius)
        .border_1()
        .border_color(tokens::border)
        .focus(|style| style.border_color(tokens::ring))
        .focus_visible(|style| style.outline(px(2.0)).outline_color(tokens::ring))
}

/// A text field bound to `value` that a handle can focus, so a chord can put the caret in it.
pub fn field_reachable(
    placeholder: &'static str,
    value: Signal<String>,
    handle: &ElementRef,
) -> impl IntoElement {
    input()
        .value(move || value.get())
        .on_change(move |text: &str| value.set(text.to_owned()))
        .placeholder(placeholder)
        .element_ref(handle)
        .text_color(move || color(tokens::foreground()))
        .placeholder_color(move || color(tokens::muted_foreground()))
        .caret_color(move || color(tokens::foreground()))
        .selection_color(move || color(tokens::selection()))
        .h(px(32.0))
        .px_3()
        .text_sm()
        .rounded(tokens::radius)
        .border_1()
        .border_color(tokens::border)
        .focus(|s| s.border_color(tokens::ring))
        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::ring))
}

/// A text field that runs `on_submit` when Return is pressed in it, for a field that commits a
/// command (the palette) rather than editing a value that saves elsewhere.
pub fn submit_field(
    placeholder: &'static str,
    value: Signal<String>,
    on_submit: impl Fn(&str) + 'static,
) -> impl IntoElement {
    input()
        .value(move || value.get())
        .on_change(move |text: &str| value.set(text.to_owned()))
        .submit_behavior(SubmitBehavior::Submit)
        .on_submit(on_submit)
        .placeholder(placeholder)
        .text_color(move || color(tokens::foreground()))
        .placeholder_color(move || color(tokens::muted_foreground()))
        .caret_color(move || color(tokens::foreground()))
        .selection_color(move || color(tokens::selection()))
        .h(px(32.0))
        .px_3()
        .text_sm()
        .rounded(tokens::radius)
        .border_1()
        .border_color(tokens::border)
        .focus(|s| s.border_color(tokens::ring))
        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::ring))
}
