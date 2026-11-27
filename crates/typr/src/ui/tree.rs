//! The tree from `nsh/base`, drawn with the app's tokens: a column of rows that fold, chosen by a
//! press or the arrow keys. The base element owns the keyboard model and what a screen reader
//! hears; this component owns the row's look and the fold mark, and reports both choices.

pub use nsh_base::TreeItem;
use nsh_base::{SelectValue, TreeEntry};
use nsh_core::element::{JsObject, Prop};
use nsh_core::prelude::*;

use super::{
    icon::{Icon, icon},
    tokens,
};

/// A tree's props.
#[derive(Clone, PartialEq, Props)]
#[non_exhaustive]
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
    /// The ids of the rows that lead with a glyph, parallel to `glyphs`.
    #[prop(default)]
    pub icon_ids: Prop<Vec<SharedString>>,
    /// The glyph of each id in `icon_ids`.
    #[prop(default)]
    pub glyphs: Prop<Vec<Icon>>,
}

/// The row a user chose in a tree.
#[derive(Clone, Debug, Default, PartialEq, Eq, JsObject)]
#[non_exhaustive]
pub struct TreeSelect {
    /// The chosen row's id.
    pub id: SharedString,
}

/// The open set after a user folded or unfolded a tree node.
#[derive(Clone, Debug, Default, PartialEq, Eq, JsObject)]
#[non_exhaustive]
pub struct TreeExpand {
    /// The row ids currently expanded.
    pub expanded: Vec<SharedString>,
}

/// What a tree reports.
#[derive(Event)]
pub enum TreeEvent {
    /// A row chosen: by a press, the arrows, Home or End, or typing its name.
    Select(TreeSelect),
    /// The open set after a fold: a press on a folder, or Left/Right, Enter or Space.
    Expand(TreeExpand),
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
            // The list rows' px_2 plus one step per level, so every glyph sits on one spine.
            let indent = px(8.0 + 12.0 * entry.depth as f32);
            let id = entry.id.clone();
            let in_force = move || props.selected.get().as_ref() == Some(&id);
            let glyph = props
                .icon_ids
                .get_untracked()
                .iter()
                .position(|id| *id == entry.id)
                .and_then(|at| props.glyphs.get_untracked().get(at).copied());
            // A disabled row reads dimmed in words as well as colour: the label carries "(soon)".
            let tone = if entry.disabled {
                tokens::muted_foreground
            } else {
                tokens::foreground
            };
            let mut row = view()
                .test_id(&format!("tree-row:{}", entry.id))
                .flex_row()
                .items_center()
                .gap_2()
                .w_full()
                .pl(indent)
                .pr_2()
                .py_1()
                .rounded(tokens::radius)
                .bg(move || tokens::row_fill(in_force()));
            if let Some(glyph) = glyph {
                row = row.child(view().text_color(tone).child(icon(glyph)));
            }
            row.child(text(entry.label.clone()).text_sm().text_color(tone))
        })
        .on_selected_change(move |id: &SharedString| {
            cx.emit(TreeEvent::Select(TreeSelect { id: id.clone() }))
        })
        .on_expanded_change(move |ids: &Vec<SharedString>| {
            cx.emit(TreeEvent::Expand(TreeExpand {
                expanded: ids.clone(),
            }))
        })
        .flex_1()
}

/// A row's label: the deck's name, and its due count when it has one. The base element gives a row
/// no trailing slot, so the count rides in the label (design Decision 3).
pub fn label_with_due(name: &str, due: usize) -> SharedString {
    match due {
        0 => SharedString::from(name.to_owned()),
        count => SharedString::from(format!("{name} {count}")),
    }
}
