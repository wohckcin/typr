//! The app's single-line text field: one border, one focus ring, the same padding as its label.

use nsh_base::{Input, input};
use nsh_core::element::{JsObject, Prop};
use nsh_core::prelude::*;

use super::{size::Size, tokens};

/// A text field's props.
#[derive(Clone, PartialEq, Props)]
#[non_exhaustive]
pub struct FieldProps {
    /// The words shown while it is empty.
    pub placeholder: Prop<SharedString>,
    /// The text in it.
    #[prop(default)]
    pub value: Prop<SharedString>,
    /// The words naming it, drawn above the field.
    #[prop(default)]
    pub label: Prop<Option<SharedString>>,
    /// How large it is: height, padding, text and radius together.
    #[prop(default)]
    pub size: Prop<Size>,
}

/// The text a user typed into a field.
#[derive(Clone, Debug, Default, PartialEq, Eq, JsObject)]
#[non_exhaustive]
pub struct FieldInput {
    /// The whole text after the edit.
    pub text: String,
}

/// What a field reports.
#[derive(Event)]
pub enum FieldEvent {
    /// The whole text, after an edit.
    Change(FieldInput),
}

/// A text field bound to `value`; typing reports the whole text once per edit.
#[component]
pub fn Field(props: FieldProps, cx: Scope<FieldEvent>) -> impl IntoElement {
    let sized = props.size.get_untracked();
    let control = styled(
        input()
            .value(move || props.value.get().to_string())
            .on_change(move |text: &str| {
                cx.emit(FieldEvent::Change(FieldInput {
                    text: text.to_owned(),
                }))
            })
            .placeholder(move || props.placeholder.get().to_string()),
        sized,
    );
    view()
        .gap_1()
        .child(dynamic(move || match props.label.get() {
            Some(label) => sized
                .text(label)
                .text_color(tokens::muted_foreground)
                .into_element(),
            None => view().into_element(),
        }))
        .child(control)
}

/// A text field bound to `value` that a handle can focus, so a chord can put the caret in it.
pub fn field_reachable(
    placeholder: String,
    value: Signal<String>,
    handle: &ElementRef,
) -> impl IntoElement {
    styled(
        input()
            .value(move || value.get())
            .on_change(move |text: &str| value.set(text.to_owned()))
            .placeholder(move || placeholder.clone())
            .element_ref(handle),
        Size::Md,
    )
}

/// The look every text field shares: colours, size, one border and the focus ring.
fn styled(control: Input, size: Size) -> Input {
    control
        .text_color(move || color(tokens::foreground()))
        .placeholder_color(move || color(tokens::muted_foreground()))
        .caret_color(move || color(tokens::foreground()))
        .selection_color(move || color(tokens::selection()))
        .min_h(size.min_height())
        .px(size.pad_x())
        .py(size.pad_y())
        .text_sm()
        .rounded(size.radius())
        .border_1()
        .border_color(tokens::border)
        .focus(|style| style.border_color(tokens::ring))
        .focus_visible(tokens::focus_ring)
}

/// A field that takes the caret when it appears and reports the text on Enter: for naming a thing
/// where it will sit. Nothing is reported while typing.
pub fn name_field(
    placeholder: &'static str,
    on_submit: impl Fn(String) + 'static,
) -> impl IntoElement {
    styled(
        input()
            .placeholder(placeholder)
            .auto_focus(true)
            .on_submit(move |text: &str| on_submit(text.to_owned())),
        Size::Sm,
    )
}
