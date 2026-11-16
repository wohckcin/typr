//! Checkbox and switch: controlled by a signal, drawn in the app's colours and sizes.
//!
//! A checkbox is for an independent choice; a switch is for a setting that takes effect at once.

use nsh_base::{checkbox, switch};
use nsh_core::prelude::*;

use super::{motion, tokens};

/// A checkbox bound to `checked`; a press writes the new value back.
pub fn check(label: impl IntoReactive<SharedString>, checked: Signal<bool>) -> impl IntoElement {
    checkbox()
        .checked(move || checked.get())
        .on_change(move |next: &bool| checked.set(*next))
        .flex_row()
        .items_center()
        .gap_2()
        .focus_visible(|s| s.outline(px(2.0)).outline_color(move || tokens::ring()))
        .indicator_style(|s| {
            s.size(px(16.0))
                .items_center()
                .justify_center()
                .rounded(tokens::radius)
                .border_1()
                .border_color(move || tokens::border())
                .checked(|s| s.bg(move || tokens::selection()).border_color(move || tokens::selection()))
        })
        .child(text(label).text_sm().text_color(move || tokens::foreground()))
}

/// A switch bound to `on`; the thumb slides on the base motion token.
pub fn toggle(label: impl IntoReactive<SharedString>, on: Signal<bool>) -> impl IntoElement {
    switch()
        .checked(move || on.get())
        .on_change(move |next: &bool| on.set(*next))
        .flex_row()
        .items_center()
        .gap_3()
        .focus_visible(|s| s.outline(px(2.0)).outline_color(move || tokens::ring()))
        .track_style(|s| {
            s.w(px(36.0))
                .h(px(20.0))
                .p(px(2.0))
                .rounded_full()
                .bg(move || tokens::hover())
                .checked(|s| s.bg(move || tokens::selection()))
        })
        .thumb_style(|s| {
            s.size(px(16.0))
                .rounded_full()
                .bg(move || tokens::card())
                .transition_transform()
                .duration(motion::BASE)
                .checked(|s| s.translate_x(px(16.0)))
        })
        .child(text(label).text_sm().text_color(move || tokens::foreground()))
}
