//! Checkbox and switch: controlled by a signal, drawn in the app's colours and sizes.
//!
//! A checkbox is for an independent choice; a switch is for a setting that takes effect at once.

use nsh_base::{checkbox, switch};
use nsh_core::element::{JsObject, Prop};
use nsh_core::prelude::*;

use super::{motion, tokens};

/// A checkbox's props.
#[derive(Clone, PartialEq, Props)]
#[non_exhaustive]
pub struct CheckProps {
    /// The words beside it.
    pub label: Prop<SharedString>,
    /// Whether it is ticked.
    #[prop(default)]
    pub checked: Prop<bool>,
}

/// The state a checkbox was toggled to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, JsObject)]
#[non_exhaustive]
pub struct CheckChange {
    /// Whether the checkbox is now ticked.
    pub checked: bool,
}

/// What a checkbox reports.
#[derive(Event)]
pub enum CheckEvent {
    /// Ticked or cleared.
    Change(CheckChange),
}

/// A checkbox bound to `checked`; a press writes the new value back.
#[component]
pub fn Check(props: CheckProps, cx: Scope<CheckEvent>) -> impl IntoElement {
    checkbox()
        .checked(move || props.checked.get())
        .on_change(move |next: &bool| cx.emit(CheckEvent::Change(CheckChange { checked: *next })))
        .flex_row()
        .items_center()
        .gap_2()
        .focus_visible(tokens::focus_ring)
        .indicator_style(|s| {
            s.size(px(16.0))
                .items_center()
                .justify_center()
                .rounded(tokens::radius)
                .border_1()
                .border_color(tokens::border)
                .checked(|s| s.bg(tokens::selection).border_color(tokens::selection))
        })
        .child(text(props.label).text_sm().text_color(tokens::foreground))
}

/// A switch's props.
#[derive(Clone, PartialEq, Props)]
#[non_exhaustive]
pub struct ToggleProps {
    /// The words beside it.
    pub label: Prop<SharedString>,
    /// Whether it is on.
    #[prop(default)]
    pub on: Prop<bool>,
}

/// The state a switch was toggled to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, JsObject)]
#[non_exhaustive]
pub struct ToggleChange {
    /// Whether the switch is now on.
    pub on: bool,
}

/// What a switch reports.
#[derive(Event)]
pub enum ToggleEvent {
    /// Turned on or off.
    Change(ToggleChange),
}

/// A switch bound to `on`; the thumb slides on the base motion token.
#[component]
pub fn Toggle(props: ToggleProps, cx: Scope<ToggleEvent>) -> impl IntoElement {
    switch()
        .checked(move || props.on.get())
        .on_change(move |next: &bool| cx.emit(ToggleEvent::Change(ToggleChange { on: *next })))
        .flex_row()
        .items_center()
        .gap_3()
        .focus_visible(tokens::focus_ring)
        .track_style(|s| {
            s.w(px(36.0))
                .h(px(20.0))
                .p(px(2.0))
                .rounded_full()
                .bg(tokens::hover)
                .checked(|s| s.bg(tokens::selection))
        })
        .thumb_style(|s| {
            let s = s.size(px(16.0)).rounded_full().bg(tokens::card);
            // Reduced motion: the thumb jumps to its side; the state change stays.
            let s = if reduce_motion() {
                s
            } else {
                s.transition_transform().duration(motion::BASE)
            };
            s.checked(|s| s.translate_x(px(16.0)))
        })
        .child(text(props.label).text_sm().text_color(tokens::foreground))
}
