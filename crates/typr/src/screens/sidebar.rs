//! The sidebar column: navigation, decks and utilities, drawn as one list of rows.
//!
//! The data comes in as [`Item`]s and is replaced whole on a refresh; what the user did to the
//! column (the row in force, which rows are folded away, a deck being named) lives in [`Sidebar`], so a
//! refresh keeps it.

use std::collections::{HashMap, HashSet};

use nsh_core::prelude::*;
use typr_core::keymap::{Command, Surface, chord_of};
use typr_core::store::{Id, Snapshot};

use crate::ui::{choice::list_row, icon::Icon, input::name_field, size, status::badge, tokens};
use typr_core::inbox::{Inbox, inbox_cards};

/// The prefix of the key reported when a new deck's name is entered: `new-deck:<name>`.
pub const NEW_DECK_PREFIX: &str = "new-deck:";

/// The groups of rows, drawn in this order. A group with no visible row draws nothing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Group {
    /// The way in: the inbox of cards that are waiting, and its two filters.
    #[default]
    Navigation,
    /// The decks, each with the prompts due in its subtree.
    Decks,
    /// Settings and the trash.
    Utilities,
}

impl Group {
    /// Every group, in drawing order.
    pub const ALL: [Group; 3] = [Group::Navigation, Group::Decks, Group::Utilities];

    /// The heading the group draws above its rows; empty for a group that has none.
    pub fn label(self) -> &'static str {
        match self {
            Group::Navigation | Group::Utilities => "",
            Group::Decks => "Decks",
        }
    }
}

/// One row of the column.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct Item {
    /// Identifies the row; a press reports it and collapsing refers to it.
    pub key: SharedString,
    /// The words on the row.
    pub label: SharedString,
    /// The group the row sits in.
    pub group: Group,
    /// The row this one is nested under, if any.
    pub parent: Option<SharedString>,
    /// The count this row states at the last refresh; zero draws no count.
    pub due: usize,
    /// Whether any row is nested under this one, so it offers a fold control.
    pub has_children: bool,
    /// The glyph before the label.
    pub icon: Option<Icon>,
    /// Whether the row is shown but cannot be used yet.
    pub disabled: bool,
}

impl Item {
    /// A row of `group` with `key` and `label`, nested under nothing and holding no work.
    pub fn new(key: impl Into<SharedString>, label: impl Into<SharedString>, group: Group) -> Item {
        Item {
            key: key.into(),
            label: label.into(),
            group,
            parent: None,
            due: 0,
            has_children: false,
            icon: None,
            disabled: false,
        }
    }

    fn with_icon(mut self, icon: Icon) -> Item {
        self.icon = Some(icon);
        self
    }
}

/// The sidebar's state: the data it shows, and what the user has done to it.
#[derive(Clone, Copy)]
pub struct Sidebar {
    items: Signal<Vec<Item>>,
    selected: Signal<Option<SharedString>>,
    collapsed: Signal<Vec<SharedString>>,
    library_open: Signal<bool>,
    notice: Signal<Option<SharedString>>,
    refusal: Signal<Option<SharedString>>,
    naming: Signal<bool>,
}

impl Sidebar {
    /// A sidebar over `items`, with the library open or closed.
    pub fn new(items: Vec<Item>, library_open: bool) -> Self {
        // The Inbox is in force until another row is chosen, so the grid and the sidebar agree about
        // what is shown; a column with no Inbox starts on its first row.
        let first = items
            .iter()
            .find(|item| item.key == "inbox")
            .or_else(|| items.first())
            .map(|item| item.key.clone());
        Self {
            items: signal(items),
            selected: signal(first),
            collapsed: signal(Vec::new()),
            library_open: signal(library_open),
            notice: signal(None),
            refusal: signal(None),
            naming: signal(false),
        }
    }

    /// States why the library could not be opened, beside "Library closed".
    pub fn note(&self, why: impl Into<SharedString>) {
        self.notice.set(Some(why.into()));
    }

    /// States why a command the person gave was refused, until they choose another row.
    pub fn refuse(&self, why: impl Into<SharedString>) {
        self.refusal.set(Some(why.into()));
    }

    /// Replaces the rows with a fresh snapshot. The row in force and the folds the user made stay.
    pub fn refresh(&self, items: Vec<Item>) {
        self.items.set(items);
    }

    /// Makes `key` the row in force.
    pub fn select(&self, key: &SharedString) {
        self.selected.set(Some(key.clone()));
        self.refusal.set(None);
    }

    /// Turns the `New Deck` row into a name field.
    pub fn start_naming(&self) {
        if self.library_open.get_untracked() {
            self.naming.set(true);
        }
    }

    /// Puts the `New Deck` row back.
    pub fn stop_naming(&self) {
        if self.naming.get_untracked() {
            self.naming.set(false);
        }
    }

    /// Whether a deck is being named.
    pub fn is_naming(&self) -> bool {
        self.naming.get()
    }

    /// Folds `key`'s subtree away, or opens it again.
    pub fn toggle_collapsed(&self, key: &SharedString) {
        self.collapsed
            .update(|folded| match folded.iter().position(|k| k == key) {
                Some(at) => {
                    folded.remove(at);
                }
                None => folded.push(key.clone()),
            });
    }

    /// The rows of `group` as a tree, each nested under its parent. A count rides in its label,
    /// because the base element gives a row no trailing slot, and a disabled row says so in words.
    pub fn tree_items(&self, group: Group) -> Vec<crate::ui::tree::TreeItem> {
        fn nest(
            items: &[Item],
            group: Group,
            parent: Option<&SharedString>,
            into: &mut Vec<crate::ui::tree::TreeItem>,
        ) {
            for item in items {
                if item.group != group || item.parent.as_ref() != parent {
                    continue;
                }
                let words = if item.disabled {
                    SharedString::from(format!("{} (coming soon)", item.label))
                } else {
                    crate::ui::tree::label_with_due(&item.label, item.due)
                };
                let mut node =
                    crate::ui::tree::TreeItem::new(item.key.clone(), words).disabled(item.disabled);
                let mut children = Vec::new();
                nest(items, group, Some(&item.key), &mut children);
                if !children.is_empty() {
                    node = node.children(children);
                }
                into.push(node);
            }
        }
        let mut top = Vec::new();
        self.items.with(|items| nest(items, group, None, &mut top));
        top
    }

    /// The ids of `group`'s rows that lead with a glyph, and the glyphs, in step.
    pub fn glyphs(&self, group: Group) -> (Vec<SharedString>, Vec<Icon>) {
        self.items.with(|items| {
            items
                .iter()
                .filter(|item| item.group == group)
                .filter_map(|item| item.icon.map(|glyph| (item.key.clone(), glyph)))
                .unzip()
        })
    }

    fn folders(&self, group: Group) -> Vec<SharedString> {
        self.items.with(|items| {
            items
                .iter()
                .filter(|item| item.group == group && item.has_children)
                .map(|item| item.key.clone())
                .collect()
        })
    }

    /// The ids `group`'s tree shows open: every row that has children, less the ones folded.
    pub fn expanded(&self, group: Group) -> Vec<SharedString> {
        self.folders(group)
            .into_iter()
            .filter(|key| !self.is_collapsed(key))
            .collect()
    }

    /// Folds every row of `group` with children that `expanded` does not name. Another group's folds
    /// stay as they are.
    pub fn set_expanded(&self, group: Group, expanded: &[SharedString]) {
        let folders = self.folders(group);
        self.collapsed.update(|folded| {
            folded.retain(|key| !folders.contains(key));
            folded.extend(
                folders
                    .iter()
                    .filter(|key| !expanded.contains(key))
                    .cloned(),
            );
        });
    }

    /// The key of the row in force, if any.
    pub fn selected_key(&self) -> Option<SharedString> {
        self.selected.with(|s| s.clone())
    }

    /// The key of the row in force, or `none`.
    pub fn selected_text(&self) -> String {
        self.selected.with(|s| {
            s.as_ref()
                .map(|k| k.to_string())
                .unwrap_or_else(|| "none".into())
        })
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
                        parent = items
                            .iter()
                            .find(|i| i.key == key)
                            .and_then(|i| i.parent.clone());
                    }
                    true
                })
                .cloned()
                .collect()
        })
    }
}

/// The column: each group with rows, then, when the library is closed, the state saying so. `pick`
/// hears every press, so the caller owns what a row opens; a named deck reports `new-deck:<name>`.
pub fn sidebar(state: Sidebar, pick: impl Fn(&SharedString) + Clone + 'static) -> impl IntoElement {
    let column = view()
        .w(tokens::sidebar_width())
        .p_3()
        .gap_3()
        .border_r_1()
        .border_color(tokens::border)
        // One dynamic for every group: one owner for the rows, so a row's controls are not rebuilt
        // under a nested owner that the next render disposes. It takes the height the footer leaves.
        .child(view().flex_1().child(groups_view(state, pick)));
    // An open library is the normal case and says nothing; only a closed one needs the footer.
    if state.library_open.get_untracked() {
        column
    } else {
        column.child(state_row(state))
    }
}

/// A tree of `group`'s rows: the base element folds them, moves the selection with the arrow keys
/// and tells a screen reader how deep each row sits.
fn group_tree(
    state: Sidebar,
    group: Group,
    pick: impl Fn(&SharedString) + Clone + 'static,
) -> impl IntoElement {
    let chosen = state;
    crate::ui::tree::tree()
        .items(move || state.tree_items(group))
        .expanded(move || state.expanded(group))
        .selected(move || state.selected_key())
        .icon_ids(move || state.glyphs(group).0)
        .glyphs(move || state.glyphs(group).1)
        .accessibility_label(if group == Group::Decks {
            "Decks"
        } else {
            "Navigation"
        })
        .on_select(move |ev: &crate::ui::tree::TreeSelect| {
            chosen.select(&ev.id);
            pick(&ev.id);
        })
        .on_expand(move |ev: &crate::ui::tree::TreeExpand| state.set_expanded(group, &ev.expanded))
}

/// Every group with its rows.
fn groups_view(state: Sidebar, pick: impl Fn(&SharedString) + Clone + 'static) -> impl IntoElement {
    dynamic(move || {
        let mut groups = view().gap_3();
        for group in Group::ALL {
            let rows: Vec<Item> = state
                .visible()
                .into_iter()
                .filter(|item| item.group == group)
                .collect();
            if rows.is_empty() {
                continue;
            }
            let mut section = view().gap_1();
            if !group.label().is_empty() {
                section = section.child(
                    text(group.label())
                        .text_xs()
                        .text_color(tokens::muted_foreground),
                );
            }
            match group {
                Group::Navigation => {
                    section = section.child(group_tree(state, group, pick.clone()))
                }
                Group::Decks => {
                    section = section
                        .child(group_tree(state, group, pick.clone()))
                        .child(new_deck_row(state, pick.clone()));
                }
                Group::Utilities => {
                    if let Some(why) = state.refusal.get() {
                        section =
                            section.child(text(why).text_sm().text_color(tokens::destructive));
                    }
                    for item in rows {
                        section = section.child(row_view(state, item, pick.clone()));
                    }
                }
            }
            groups = groups.child(section);
        }
        groups.into_element()
    })
}

/// `New Deck`, or the field that names it. Enter with a name reports `new-deck:<name>`; a blank
/// name puts the row back.
///
/// Not a `dynamic` of its own: it is read inside the groups' one, so toggling the field rebuilds the
/// rows under that owner instead of a nested one the next render disposes.
fn new_deck_row(state: Sidebar, pick: impl Fn(&SharedString) + Clone + 'static) -> AnyElement {
    if state.is_naming() {
        name_field("Deck name", move |name: String| {
            state.stop_naming();
            if typr_core::edits::deck_name(&name).is_some() {
                pick(&SharedString::from(format!("{NEW_DECK_PREFIX}{name}")));
            }
        })
        .into_element()
    } else {
        list_row()
            .title("New Deck")
            .icon(Icon::Plus)
            .selected(|| false)
            .outlined(false)
            .trailing(|| None::<SharedString>)
            .disabled(move || !state.library_open.get())
            .on_press(move || state.start_naming())
            .into_element()
    }
}

/// The foot of a closed library's column: a dot and the words, and why when it is known. The dot is
/// a second cue beside the tone; it has no name of its own, so a screen reader hears the state once.
fn state_row(state: Sidebar) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap_2()
        .pt_3()
        .border_t_1()
        .border_color(tokens::border)
        .child(
            view()
                .test_id("library-dot")
                .w(size::dot())
                .h(size::dot())
                .rounded_full()
                .bg(tokens::destructive),
        )
        .child(library_state(state))
}

fn library_state(state: Sidebar) -> impl IntoElement {
    text(move || match state.notice.get() {
        Some(why) => SharedString::from(format!("Library closed: {why}")),
        None => SharedString::from("Library closed"),
    })
    .text_sm()
    .text_color(tokens::destructive)
}

/// A utility row: a glyph and a label, one press target. Settings and Trash are screens and take the
/// selection; the caller decides what any other row does with the press.
fn row_view(
    state: Sidebar,
    item: Item,
    pick: impl Fn(&SharedString) + Clone + 'static,
) -> impl IntoElement {
    let key = item.key.clone();
    let in_force = key.clone();
    let press = key.clone();
    let mut row = view().flex_row().items_center().gap_1().child(
        view().flex_1().child(
            list_row()
                .title(item.label.clone())
                .icon(item.icon)
                .outlined(false)
                .trailing(|| None::<SharedString>)
                .disabled(item.disabled)
                .selected(move || state.selected.with(|s| s.as_ref() == Some(&in_force)))
                .on_press(move || pick(&press)),
        ),
    );
    if item.due > 0 {
        row = row.child(badge().label(item.due.to_string()));
    }
    row
}

/// The rows the column shows for a library at `now_ms`: the Inbox with the cards that are due or
/// new, and its two filters as rows of their own, every live deck with its due count (`due`
/// comes from [`typr_core`]'s subtree counts, so a parent counts its children), and Settings and
/// Trash.
pub fn items_from(snapshot: &Snapshot, due: &HashMap<Id, u32>, now_ms: i64) -> Vec<Item> {
    let live: Vec<_> = snapshot
        .decks
        .iter()
        .filter(|deck| !deck.archived)
        .collect();
    let live_ids: HashSet<Id> = live.iter().map(|deck| deck.id).collect();
    let parents: HashSet<Id> = live
        .iter()
        .filter_map(|deck| deck.parent)
        .filter(|p| live_ids.contains(p))
        .collect();
    let count = |which| inbox_cards(snapshot, now_ms, which).len();
    // Flat rows: a press on the Inbox opens it and never folds its filters away.
    let inbox_row = |key: &str, label: &str, glyph, which| {
        let mut item =
            Item::new(key.to_owned(), label.to_owned(), Group::Navigation).with_icon(glyph);
        item.due = count(which);
        item
    };
    let mut items = vec![
        inbox_row("inbox", "Inbox", Icon::Inbox, Inbox::All),
        inbox_row("inbox:due", "Due today", Icon::CircleCheck, Inbox::Due),
        inbox_row("inbox:new", "New cards", Icon::CircleDashed, Inbox::New),
    ];
    for deck in &live {
        let mut item = Item::new(format!("deck:{}", deck.id), deck.name.clone(), Group::Decks)
            .with_icon(if deck.name == "Notes" {
                Icon::Archive
            } else {
                Icon::Book
            });
        item.parent = deck
            .parent
            .filter(|p| live_ids.contains(p))
            .map(|p| format!("deck:{p}").into());
        item.due = due.get(&deck.id).copied().unwrap_or(0) as usize;
        item.has_children = parents.contains(&deck.id);
        items.push(item);
    }
    items.push(Item::new("settings", "Settings", Group::Utilities).with_icon(Icon::Gear));
    items.push(Item::new("trash", "Trash", Group::Utilities).with_icon(Icon::Trash));
    items
}

/// The chord that focuses search, read from the keymap table rather than written here; empty when
/// the library surface binds none.
pub fn search_chord() -> String {
    chord_of(Surface::Library, Command::FocusSearch)
        .map(|chord| chord.to_string())
        .unwrap_or_default()
}

pub use typr_core::inbox::due_counts;
