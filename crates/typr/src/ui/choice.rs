//! Choosing among things: tabs for sections, a select for one value from a list, a list row for
//! a selectable item.

use nsh_base::{SelectItem, select, tab, tabs};
use nsh_core::prelude::*;

use super::tokens;

/// A row of tabs showing which section is selected; the caller shows the panel.
pub fn tab_row(labels: &'static [&'static str], selected: Signal<usize>) -> impl IntoElement {
    let mut row = tabs().flex_row().gap_1().p_1().rounded(tokens::radius).bg(move || tokens::hover());
    for (index, label) in labels.iter().copied().enumerate() {
        let is_selected = move || selected.get() == index;
        row = row.child(
            tab()
                .selected(is_selected)
                .on_press(move |_: &()| selected.set(index))
                .px_3()
                .py_1()
                .rounded(tokens::radius)
                .selected_style(|s| s.bg(move || tokens::card()))
                .child(text(label).text_sm().text_color(move || tokens::foreground())),
        );
    }
    row
}

/// One value from a list of `(value, title)` pairs; the closed control shows the chosen title.
pub fn choose(options: &'static [(&'static str, &'static str)], chosen: Signal<Option<SharedString>>) -> impl IntoElement {
    let items: Vec<SelectItem> = options.iter().map(|(v, t)| SelectItem::new(*v, *t)).collect();
    select()
        .items(items)
        .value(move || chosen.get())
        .on_change(move |next: &Option<SharedString>| chosen.set(next.clone()))
        .accessibility_label("Choose")
        .trigger(
            view()
                .h(px(32.0))
                .px_3()
                .justify_center()
                .rounded(tokens::radius)
                .border_1()
                .border_color(move || tokens::border())
                .child(text(move || chosen.get().unwrap_or("Choose".into())).text_sm()),
        )
}

/// A selectable row: its own surface shows the selection, not a marker at the edge.
///
/// The row is a pressable view rather than a base button: a base button made inside the sidebar's
/// reactive list is disposed with the list's previous render, and its `disabled` prop is then read
/// from a dead owner (the engine logs "a prop read or written after its owner was disposed").
pub fn list_row(title: impl IntoReactive<SharedString>, selected: impl Fn() -> bool + 'static, on_press: impl Fn() + 'static) -> impl IntoElement {
    view()
        .w_full()
        .bg(move || tokens::row_fill(selected()))
        .on_press(on_press)
        .child(text(title).text_sm().text_color(move || tokens::foreground()))
}
