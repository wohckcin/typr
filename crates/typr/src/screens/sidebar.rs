//! The sidebar column: navigation, decks, views, tags and utilities, drawn as one list of rows.
//!
//! The data comes in as [`Item`]s and is replaced whole on a refresh; what the user did to the
//! column (the row in force, which decks are folded away) lives in [`Sidebar`], so a refresh keeps
//! it.

use std::collections::{HashMap, HashSet};

use nsh_core::prelude::*;
use typr_core::keymap::{Command, Surface, chord_of};
use typr_core::store::{Id, Snapshot};

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

/// The rows the column shows for a library: the whole library, every live deck with its due count
/// (`due` comes from [`typr_core`]'s subtree counts, so a parent counts its children), the saved
/// views, and Settings.
pub fn items_from(snapshot: &Snapshot, due: &HashMap<Id, u32>) -> Vec<Item> {
    let live: Vec<_> = snapshot.decks.iter().filter(|deck| !deck.archived).collect();
    let live_ids: HashSet<Id> = live.iter().map(|deck| deck.id).collect();
    let parents: HashSet<Id> = live.iter().filter_map(|deck| deck.parent).filter(|p| live_ids.contains(p)).collect();
    let mut items = vec![Item {
        key: "all".into(),
        label: "All cards".into(),
        group: Group::Navigation,
        parent: None,
        due: 0,
        has_children: false,
    }];
    for deck in &live {
        items.push(Item {
            key: format!("deck:{}", deck.id).into(),
            label: deck.name.clone().into(),
            group: Group::Decks,
            parent: deck.parent.filter(|p| live_ids.contains(p)).map(|p| format!("deck:{p}").into()),
            due: due.get(&deck.id).copied().unwrap_or(0) as usize,
            has_children: parents.contains(&deck.id),
        });
    }
    for view in &snapshot.views {
        items.push(Item {
            key: format!("view:{}", view.id).into(),
            label: view.name.clone().into(),
            group: Group::Views,
            parent: None,
            due: 0,
            has_children: false,
        });
    }
    items.push(Item {
        key: "settings".into(),
        label: "Settings".into(),
        group: Group::Utilities,
        parent: None,
        due: 0,
        has_children: false,
    });
    items
}

/// The chord that focuses search, read from the keymap table rather than written here; empty when
/// the library surface binds none.
pub fn search_chord() -> String {
    chord_of(Surface::Library, Command::FocusSearch).map(|chord| chord.to_string()).unwrap_or_default()
}

/// How many prompts are due in each deck's subtree at `now_ms`: a due card counts for its own deck
/// and every deck above it. Archived decks, archived cards and trashed cards count for nothing.
pub fn due_counts(snapshot: &Snapshot, now_ms: i64) -> HashMap<Id, u32> {
    let parents: HashMap<Id, Option<Id>> = snapshot.decks.iter().map(|deck| (deck.id, deck.parent)).collect();
    let archived: HashSet<Id> = snapshot.decks.iter().filter(|deck| deck.archived).map(|deck| deck.id).collect();
    let mut counts: HashMap<Id, u32> = HashMap::new();
    for card in &snapshot.cards {
        let Some(due_at) = card.due_at else {
            continue;
        };
        if due_at > now_ms || card.archived || card.trashed_at.is_some() {
            continue;
        }
        let mut current = Some(card.deck);
        // A parent chain that loops would be walked forever; the hop limit keeps a bad snapshot
        // from hanging the frame.
        let mut hops = 0;
        while let Some(id) = current {
            if hops > 64 || archived.contains(&id) {
                break;
            }
            let Some(parent) = parents.get(&id).copied() else {
                break;
            };
            *counts.entry(id).or_insert(0) += 1;
            current = parent;
            hops += 1;
        }
    }
    counts
}
