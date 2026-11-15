//! The sidebar column: navigation, decks, views, tags and utilities, drawn as one list of rows.
//!
//! The data comes in as [`Item`]s and is replaced whole on a refresh; what the user did to the
//! column (the row in force, which decks are folded away) lives in [`Sidebar`], so a refresh keeps
//! it.

use nsh_core::prelude::*;

use crate::ui::{
    button::{Variant, button},
    choice::list_row,
    status::badge,
    tokens,
};

/// The groups of rows, drawn in this order. A group with no visible row draws nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    /// The way out of a deck: the whole library.
    Navigation,
    /// The decks, each with the prompts due in its subtree.
    Decks,
    /// The saved views.
    Views,
    /// The tags the library holds.
    Tags,
    /// Import, settings and the trash.
    Utilities,
}

impl Group {
    /// Every group, in drawing order.
    pub const ALL: [Group; 5] = [Group::Navigation, Group::Decks, Group::Views, Group::Tags, Group::Utilities];

    /// The heading the group draws above its rows.
    pub fn label(self) -> &'static str {
        match self {
            Group::Navigation => "Navigation",
            Group::Decks => "Decks",
            Group::Views => "Views",
            Group::Tags => "Tags",
            Group::Utilities => "Utilities",
        }
    }
}

/// One row of the column.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    /// Identifies the row; a press reports it and collapsing refers to it.
    pub key: SharedString,
    /// The words on the row.
    pub label: SharedString,
    /// The group the row sits in.
    pub group: Group,
    /// The row this one is nested under, if any.
    pub parent: Option<SharedString>,
    /// The prompts due in this row's subtree at the last refresh; zero draws no count.
    pub due: usize,
    /// Whether any row is nested under this one, so it offers a fold control.
    pub has_children: bool,
}

/// The sidebar's state: the data it shows, and what the user has done to it.
#[derive(Clone, Copy)]
pub struct Sidebar {
    items: Signal<Vec<Item>>,
    selected: Signal<Option<SharedString>>,
    collapsed: Signal<Vec<SharedString>>,
    library_open: Signal<bool>,
    search_chord: Signal<SharedString>,
}

impl Sidebar {
    /// A sidebar over `items`, with the library open or closed and the chord that focuses search.
    pub fn new(items: Vec<Item>, library_open: bool, search_chord: impl Into<SharedString>) -> Self {
        Self {
            items: signal(items),
            selected: signal(None),
            collapsed: signal(Vec::new()),
            library_open: signal(library_open),
            search_chord: signal(search_chord.into()),
        }
    }

    /// Replaces the rows with a fresh snapshot. The row in force and the folds the user made stay.
    pub fn refresh(&self, items: Vec<Item>) {
        self.items.set(items);
    }

    /// Makes `key` the row in force.
    pub fn select(&self, key: &SharedString) {
        self.selected.set(Some(key.clone()));
    }

    /// Folds `key`'s subtree away, or opens it again.
    pub fn toggle_collapsed(&self, key: &SharedString) {
        self.collapsed.update(|folded| match folded.iter().position(|k| k == key) {
            Some(at) => {
                folded.remove(at);
            }
            None => folded.push(key.clone()),
        });
    }

    /// The key of the row in force, or `none`.
    pub fn selected_text(&self) -> String {
        self.selected.with(|s| s.as_ref().map(|k| k.to_string()).unwrap_or_else(|| "none".into()))
    }

    fn is_collapsed(&self, key: &SharedString) -> bool {
        self.collapsed.with(|folded| folded.contains(key))
    }

    /// The rows the column draws now: those whose ancestors are all open.
    pub fn visible(&self) -> Vec<Item> {
        self.items.with(|items| {
            items
                .iter()
                .filter(|item| {
                    let mut parent = item.parent.clone();
                    while let Some(key) = parent {
                        if self.is_collapsed(&key) {
                            return false;
                        }
                        parent = items.iter().find(|i| i.key == key).and_then(|i| i.parent.clone());
                    }
                    true
                })
                .cloned()
                .collect()
        })
    }
}

/// The column: the search chord, the library's state, then each group with rows.
pub fn sidebar(state: Sidebar) -> impl IntoElement {
    let mut column = view()
        .w(tokens::sidebar_width())
        .p_3()
        .gap_3()
        .border_r_1()
        .border_color(move || tokens::border())
        .child(search_row(state))
        .child(library_state(state));
    for group in Group::ALL {
        column = column.child(group_view(state, group));
    }
    column
}

fn search_row(state: Sidebar) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .justify_between()
        .child(text("Search").text_sm().text_color(move || tokens::foreground()))
        .child(dynamic(move || badge(state.search_chord.get()).into_element()))
}

fn library_state(state: Sidebar) -> impl IntoElement {
    text(move || if state.library_open.get() { "Library open" } else { "Library closed" })
        .text_sm()
        .text_color(move || {
            if state.library_open.get() { tokens::muted_foreground() } else { tokens::destructive() }
        })
}

fn group_view(state: Sidebar, group: Group) -> impl IntoElement {
    dynamic(move || {
        let rows: Vec<Item> = state.visible().into_iter().filter(|item| item.group == group).collect();
        if rows.is_empty() {
            return view().into_element();
        }
        let mut column = view()
            .gap_1()
            .child(text(group.label()).text_xs().text_color(move || tokens::muted_foreground()));
        for item in rows {
            column = column.child(row_view(state, item));
        }
        column.into_element()
    })
}

fn row_view(state: Sidebar, item: Item) -> impl IntoElement {
    let key = item.key.clone();
    let in_force = key.clone();
    let press = key.clone();
    let fold = key.clone();
    let folded = key.clone();
    let mut row = view().flex_row().items_center().gap_1().child(view().flex_1().child(list_row(
        item.label.clone(),
        move || state.selected.with(|s| s.as_ref() == Some(&in_force)),
        move || state.select(&press),
    )));
    if item.due > 0 {
        row = row.child(badge(item.due.to_string()));
    }
    if item.has_children {
        row = row.child(button(
            Variant::Ghost,
            move || -> SharedString { if state.is_collapsed(&folded) { "▸".into() } else { "▾".into() } },
            || false,
            move || state.toggle_collapsed(&fold),
        ));
    }
    row
}
