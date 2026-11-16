//! Overlays: a menu of commands, a popover of controls, a tooltip for an icon, a dialog for a
//! decision, and a toast for a result that needs no reply.

use nsh_base::{dialog, menu, menu_item, popover};
use nsh_core::{element::StyleRefinement, prelude::*};

use super::{motion, tokens};

/// The popup surface every anchored overlay shares.
fn popup() -> nsh_core::element::StyleRefinement {
    StyleRefinement::default()
        .min_w(px(192.0))
        .p_1()
        .rounded(tokens::radius)
        .border_1()
        .border_color(tokens::border)
        .bg(tokens::popover())
}

/// A menu opened from `trigger`, with commands as its items.
pub fn command_menu(trigger: impl IntoElement, commands: Commands) -> impl IntoElement {
    let mut m = menu()
        .offset(px(4.0))
        .popup_style(|_| popup())
        .trigger(trigger);
    for (label, action) in commands {
        m = m.child(
            menu_item()
                .on_select(action)
                .px_3()
                .py_2()
                .rounded(tokens::radius)
                .hover(|s| s.bg(tokens::hover))
                .child(text(label).text_sm().text_color(tokens::foreground)),
        );
    }
    m
}

/// Controls shown beside their trigger without interrupting the task.
/// The rows a menu shows: a label and what it does.
pub type Commands = Vec<(&'static str, Box<dyn Fn()>)>;

pub fn anchored(trigger: impl IntoElement, content: impl IntoElement) -> impl IntoElement {
    popover().offset(px(6.0)).trigger(trigger).child(
        view()
            .p_3()
            .child(content)
            .bg(tokens::popover())
            .rounded(tokens::radius),
    )
}

/// A short explanation of an icon-only control, shown on hover.
pub fn hint(anchor: impl IntoElement, label: &'static str) -> impl IntoElement {
    view().tooltip(text(label).text_xs()).child(anchor)
}

/// A focused decision: shown while `open`, cancelled by Escape or a backdrop press.
pub fn decision<V: IntoElement + 'static>(
    open: Signal<bool>,
    content: impl Fn() -> V + 'static,
) -> impl IntoElement {
    when(
        move || open.get(),
        move || {
            dialog()
                .on_cancel(move || open.set(false))
                .flex_1()
                .bg(tokens::background)
                .p_4()
                .child(content())
        },
    )
}

/// A result that needs no reply: a line of text while `message` is set.
pub fn toast(message: Signal<Option<SharedString>>) -> impl IntoElement {
    when(
        move || message.with(Option::is_some),
        move || {
            view()
                .p_3()
                .rounded(tokens::radius)
                .bg(tokens::popover())
                .border_1()
                .border_color(tokens::border)
                .child(
                    text(move || message.get().unwrap_or_default())
                        .text_sm()
                        .text_color(tokens::foreground),
                )
        },
    )
}

/// How long a toast stays before it is cleared, from the motion tokens.
pub const TOAST_MS: f32 = motion::SLOW;
