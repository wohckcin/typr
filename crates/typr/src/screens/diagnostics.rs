//! The diagnostics screen: the failure line, the recorded slow commands, and the import and export
//! entry points. The records are passed in; file dialogs are the caller's, and a result line shows
//! through the app's toast.

use nsh_core::prelude::*;

use crate::ui::{button::{Variant, button}, overlay::toast, status::badge, surface, tokens};

/// A command the app recorded as slow.
#[derive(Clone, Debug, PartialEq)]
pub struct SlowCommand {
    /// The command's name.
    pub name: SharedString,
    /// How long it took, in milliseconds.
    pub millis: u32,
}

/// The line a stopped library worker sets. It names the stoppage, once, and is absent otherwise.
pub fn stoppage_message(stopped: bool) -> Option<&'static str> {
    stopped.then_some("The library work stopped; the app keeps running and later actions will report failure.")
}

/// The diagnostics screen. `stopped` is the failure line's state; `slow` the recorded commands;
/// `on_import` and `on_export` run the caller's dialogs; `result` is the line shown after one.
pub fn diagnostics(
    stopped: Signal<bool>,
    slow: Vec<SlowCommand>,
    result: Signal<Option<SharedString>>,
    on_import: impl Fn() + 'static,
    on_export: impl Fn() + 'static,
) -> impl IntoElement {
    let mut slow_list = view().gap_2();
    if slow.is_empty() {
        slow_list = slow_list.child(text("No slow commands recorded").text_sm().text_color(move || tokens::muted_foreground()));
    }
    for command in slow {
        slow_list = slow_list.child(surface::card(
            text(format!("{} · {} ms", command.name, command.millis)).text_sm().text_color(move || tokens::foreground()),
        ));
    }
    view()
        .flex_1()
        .gap_4()
        .p_4()
        .child(dynamic(move || match stoppage_message(stopped.get()) {
            Some(message) => text(message).text_sm().text_color(move || tokens::destructive()).into_element(),
            None => view().into_element(),
        }))
        .child(badge("Slow commands"))
        .child(slow_list)
        .child(
            view()
                .flex_row()
                .gap_2()
                .child(button(Variant::Default, "Import", || false, on_import))
                .child(button(Variant::Default, "Export", || false, on_export)),
        )
        .child(toast(result))
}
