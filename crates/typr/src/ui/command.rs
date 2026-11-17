//! The command list: the base `Command` element, with the app's row look and its own events.

use nsh_base::{CommandEntry, CommandItem, command as base_command};
use nsh_core::element::StyleRefinement;
use nsh_core::prelude::*;

use super::{status::keycap, tokens};

/// What the list is for: the palette runs the highlighted command, the shortcut panel only shows
/// the bindings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// A command palette: Return runs the highlighted item.
    Palette,
    /// The shortcut panel: the rows are the bindings, and nothing runs from here.
    Panel,
}

/// A list of `items` that filters as the user types, reporting the item the user chose.
///
/// The rows, the keys that move between them, the typeahead and the filtering are the base
/// element's; the app decides the items and the row's look.
pub fn command(
    items: Vec<CommandItem>,
    placeholder: &'static str,
    mode: Mode,
    on_run: impl Fn(&SharedString) + Clone + 'static,
    on_cancel: impl Fn() + Clone + 'static,
) -> impl IntoElement {
    let run = on_run.clone();
    let cancel = on_cancel.clone();
    let runnable = mode == Mode::Palette;
    let mut list = base_command()
        .items(items)
        .searchable(true)
        .auto_focus(true)
        .placeholder(placeholder)
        // The base element's field has no colours of its own; without these the placeholder is
        // the window's, which is invisible on the app's dark surface.
        .search_style(|s| s.text_color(tokens::foreground()))
        // The field's own surface: without it the query line is the dialog's background and the
        // placeholder reads as empty space.
        .search_bar_style(|s: StyleRefinement| {
            s.px_3()
                .py_2()
                .mb_2()
                .rounded(tokens::radius())
                .border_1()
                .border_color(tokens::border())
                .bg(tokens::card())
        })
        .accessibility_label(Some(SharedString::from(match mode {
            Mode::Palette => "Commands",
            Mode::Panel => "Keyboard shortcuts",
        })))
        .empty(
            view().child(
                text("No matching command")
                    .text_sm()
                    .text_color(tokens::muted_foreground),
            ),
        )
        .render_item(move |entry: &CommandEntry| {
            let mut row = view()
                .flex_row()
                .items_center()
                .justify_between()
                .gap_3()
                .child(
                    text(entry.label.clone())
                        .text_sm()
                        .text_color(tokens::foreground),
                );
            if let Some(label) = entry.shortcut_label.clone() {
                row = row.child(keycap().chord(label));
            }
            row
        })
        .on_select(move |id: &SharedString| {
            if runnable {
                run(id);
            }
        })
        .on_cancel(cancel)
        .flex_1();
    // The panel and the palette are one list; the panel says which it is, because the rows look
    // alike and only the palette runs anything on Return.
    if mode == Mode::Panel {
        list = list.header(
            view().child(
                text("Keyboard shortcuts")
                    .text_sm()
                    .text_color(tokens::muted_foreground()),
            ),
        );
    }
    list
}
