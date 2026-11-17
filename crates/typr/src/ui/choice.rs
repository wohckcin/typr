//! Choosing among things: tabs for sections, a select for one value from a list, a list row for
//! a selectable item.

use nsh_base::button as base_button;
use nsh_base::{SelectItem, select, tab, tabs};
use nsh_core::element::Prop;
use nsh_core::prelude::*;

use super::{motion, tokens};

/// A tab row's props.
#[derive(Clone, PartialEq, Props)]
pub struct TabRowProps {
    /// The sections, in drawing order.
    pub labels: Prop<Vec<SharedString>>,
    /// Which one is selected.
    #[prop(default)]
    pub selected: Prop<usize>,
}

/// What a tab row reports.
#[derive(Event)]
pub enum TabRowEvent {
    /// The section chosen.
    Select(usize),
}

/// A row of tabs showing which section is selected; the caller shows the panel.
#[component]
pub fn TabRow(props: TabRowProps, cx: Scope<TabRowEvent>) -> impl IntoElement {
    let labels = props.labels.get_untracked();
    let mut row = tabs()
        .flex_row()
        .gap_1()
        .p_1()
        .rounded(tokens::radius)
        .bg(tokens::hover);
    for (index, label) in labels.into_iter().enumerate() {
        row = row.child(
            tab()
                .selected(move || props.selected.get() == index)
                .on_press(move |_: &()| cx.emit(TabRowEvent::Select(index)))
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

/// A chooser's props.
#[derive(Clone, PartialEq, Props)]
pub struct ChooseProps {
    /// The options, each with the value it reports.
    pub options: Prop<Vec<SelectItem>>,
    /// The value in force, if one is.
    #[prop(default)]
    pub chosen: Prop<Option<SharedString>>,
}

/// What a chooser reports.
#[derive(Event)]
pub enum ChooseEvent {
    /// The value chosen.
    Change(Option<SharedString>),
}

/// One value from a list; the closed control shows the chosen title.
#[component]
pub fn Choose(props: ChooseProps, cx: Scope<ChooseEvent>) -> impl IntoElement {
    select()
        .items(props.options.get_untracked())
        .value(move || props.chosen.get())
        .on_change(move |next: &Option<SharedString>| cx.emit(ChooseEvent::Change(next.clone())))
        .accessibility_label("Choose")
        .trigger(
            view()
                .h(px(32.0))
                .px_3()
                .justify_center()
                .rounded(tokens::radius)
                .border_1()
                .border_color(tokens::border)
                .child(text(move || props.chosen.get().unwrap_or("Choose".into())).text_sm()),
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
/// A control's props are made by the component that owns the control: a prop made in the region
/// that renders a list is read after that region goes, and the engine logs "a prop read or written
/// after its owner was disposed". `tests/startup.rs` holds the rule.
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
        // Hover is a wash weaker than the selected fill, and a press deepens it; a row does not
        // scale, because its fill is the state.
        .transition_colors()
        .duration(motion::INSTANT)
        .hover(|style| style.bg(move || tokens::hover().opacity(0.6)))
        .active(|style| style.bg(move || tokens::hover().opacity(0.9)))
        .focus_visible(|style| style.outline(px(2.0)).outline_color(tokens::ring))
        .child(text(props.title).text_sm().text_color(tokens::foreground))
}
