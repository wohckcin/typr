//! The tree from `nsh/base`, drawn with the app's tokens: a column of rows that fold, chosen by a
//! press or the arrow keys. The base element owns the keyboard model and what a screen reader
//! hears; this component owns the row's look and the fold mark, and reports both choices.

use nsh_base::{SelectValue, TreeEntry, TreeItem};
use nsh_core::element::Prop;
use nsh_core::prelude::*;

use super::tokens;

/// A tree's props.
#[derive(Clone, PartialEq, Props)]
pub struct TreeProps {
    /// The rows, each with its children already nested.
    pub items: Prop<Vec<TreeItem>>,
    /// The ids that are open.
    #[prop(default)]
    pub expanded: Prop<Vec<SharedString>>,
    /// The id in force, if any.
    #[prop(default)]
    pub selected: Prop<Option<SharedString>>,
    /// What a screen reader announces the tree as.
    #[prop(default)]
    pub accessibility_label: Prop<SharedString>,
}

/// What a tree reports.
#[derive(Event)]
pub enum TreeEvent {
    /// A row chosen: by a press, the arrows, Home or End, or typing its name.
    Selected(SharedString),
    /// The open set after a fold: a press on a folder, or Left/Right, Enter or Space.
    Expanded(Vec<SharedString>),
}

/// A tree of its items, folding them and reporting the row chosen and the set left open.
///
/// A folder folds on a press, and with Left/Right, Enter and Space while the tree holds focus —
/// the base element's own model. No chevron is drawn: a second control for the same fold would
/// toggle twice on one press, and its press would reach the row beneath it.
#[component]
pub fn Tree(props: TreeProps, cx: Scope<TreeEvent>) -> impl IntoElement {
    nsh_base::tree()
        .items(move || props.items.get())
        .selected(move || SelectValue::Controlled(props.selected.get()))
        .expanded(move || Some(props.expanded.get()))
        .accessibility_label(move || Some(props.accessibility_label.get()))
        .render_item(move |entry: &TreeEntry| {
            let indent = px(12.0 * entry.depth as f32);
            view().flex_row().items_center().gap_1().pl(indent).child(
                text(entry.label.clone())
                    .text_sm()
                    .text_color(tokens::foreground),
            )
        })
        .on_selected_change(move |id: &SharedString| cx.emit(TreeEvent::Selected(id.clone())))
        .on_expanded_change(move |ids: &Vec<SharedString>| {
            cx.emit(TreeEvent::Expanded(ids.clone()))
        })
        .flex_1()
}

/// The rows of a tree: each item with its children already nested.
pub type Items = Vec<TreeItem>;

/// One item, with its children already nested.
pub fn item(id: impl Into<SharedString>, label: impl Into<SharedString>) -> TreeItem {
    TreeItem::new(id, label)
}

/// A row's label: the deck's name, and its due count when it has one. The base element gives a row
/// no trailing slot, so the count rides in the label (design Decision 3).
pub fn label_with_due(name: &str, due: usize) -> SharedString {
    match due {
        0 => SharedString::from(name.to_owned()),
        count => SharedString::from(format!("{name} {count}")),
    }
}
