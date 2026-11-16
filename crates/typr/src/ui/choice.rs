//! Choosing among things: tabs for sections, a select for one value from a list, a list row for
//! a selectable item.

use nsh_base::button as base_button;
use nsh_base::{SelectItem, select, tab, tabs};
use nsh_core::element::Prop;
use nsh_core::prelude::*;

use super::tokens;

/// A row of tabs showing which section is selected; the caller shows the panel.
pub fn tab_row(labels: &'static [&'static str], selected: Signal<usize>) -> impl IntoElement {
    let mut row = tabs()
        .flex_row()
        .gap_1()
        .p_1()
        .rounded(tokens::radius)
        .bg(tokens::hover);
    for (index, label) in labels.iter().copied().enumerate() {
        let is_selected = move || selected.get() == index;
        row = row.child(
            tab()
                .selected(is_selected)
                .on_press(move |_: &()| selected.set(index))
                .px_3()
                .py_1()
                .rounded(tokens::radius)
                .selected_style(|s| s.bg(tokens::card))
                .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::ring))
                .child(text(label).text_sm().text_color(tokens::foreground)),
        );
    }
    row
}

/// One value from a list of `(value, title)` pairs; the closed control shows the chosen title.
pub fn choose(
    options: &'static [(&'static str, &'static str)],
    chosen: Signal<Option<SharedString>>,
) -> impl IntoElement {
    let items: Vec<SelectItem> = options
        .iter()
        .map(|(v, t)| SelectItem::new(*v, *t))
        .collect();
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
                .border_color(tokens::border)
                .child(text(move || chosen.get().unwrap_or("Choose".into())).text_sm()),
        )
}

/// A row's props.
#[derive(Clone, PartialEq, Props)]
pub struct ListRowProps {
    /// The words on the row.
    pub title: Prop<SharedString>,
    /// Whether it is the row in force.
    #[prop(default)]
    pub selected: Prop<bool>,
}

/// What a row reports.
#[derive(Event)]
pub enum ListRowEvent {
    /// Pressed.
    Press,
}

/// A selectable row: its own surface shows the selection, not a marker at the edge.
///
/// The row is a pressable view rather than a base button: a base button made inside the sidebar's
/// reactive list is disposed with the list's previous render, and its `disabled` prop is then read
/// from a dead owner (the engine logs "a prop read or written after its owner was disposed").
#[component]
pub fn ListRow(props: ListRowProps, cx: Scope<ListRowEvent>) -> impl IntoElement {
    // A row is a control like any other: it says what it is, answers hover and press, and shows the
    // keyboard's ring while the focus is on it.
    base_button()
        .on_press(move || cx.emit(ListRowEvent::Press))
        .selected(move || props.selected.get())
        .w_full()
        .px_2()
        .py_1()
        .rounded(tokens::radius)
        .bg(move || tokens::row_fill(props.selected.get()))
        // Hover is a wash weaker than the selected fill, and pressed is the fill a selection uses.
        .hover(|style| style.bg(move || tokens::hover().opacity(0.6)))
        .active(|style| style.bg(tokens::selection))
        .focus_visible(|style| style.outline(px(2.0)).outline_color(tokens::ring))
        .child(text(props.title).text_sm().text_color(tokens::foreground))
}
