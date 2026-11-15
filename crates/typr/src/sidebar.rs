//! The sidebar column: navigation, decks and utilities, drawn as one list of rows.
//!
//! The measurements come from the reference (Mochi at 1485 points, dark): a 225-point column
//! with no divider, rows 211 points wide at a 7-point inset, 25 points tall, a 5-point radius,
//! and a label that starts where the icon's slot ends plus a 14-point gap.

use std::rc::Rc;

use rune::tok::{self, type_};
use rune::{El, col, text};

/// The width of the whole column, in points.
pub const WIDTH: f32 = 225.0;

/// The height of one row.
pub const ROW_HEIGHT: f32 = 25.0;

/// The horizontal inset between the column's edge and a row's fill.
pub const INSET: f32 = 7.0;

/// The width of a row's icon slot.
pub const ICON: f32 = 20.0;

/// The gap between the icon slot and the label.
pub const GAP_AFTER_ICON: f32 = 14.0;

/// The gap between one group of rows and the next.
pub const GROUP_GAP: f32 = 20.0;

/// Where a label starts inside a row, measured from the row's own left edge.
pub const LABEL_X: f32 = ICON + GAP_AFTER_ICON;

/// The groups of rows, drawn in this order. A group with nothing in it draws nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    /// The way out of a deck: the whole library and saved views.
    Navigation,
    /// The decks, each with its count.
    Decks,
    /// The tags the library holds, which filter the list.
    Tags,
    /// Import, settings and the trash.
    Utilities,
}

impl Group {
    /// Every group, in drawing order.
    pub const ALL: [Group; 4] = [
        Group::Navigation,
        Group::Decks,
        Group::Tags,
        Group::Utilities,
    ];

    /// The element key of the group's container.
    pub fn key(self) -> &'static str {
        match self {
            Group::Navigation => "sidebar-navigation",
            Group::Decks => "sidebar-decks",
            Group::Tags => "tags",
            Group::Utilities => "sidebar-utilities",
        }
    }

    /// The heading the group draws above its rows.
    pub fn label(self) -> &'static str {
        match self {
            Group::Navigation => "Navigation",
            Group::Decks => "Decks",
            Group::Tags => "Tags",
            Group::Utilities => "Utilities",
        }
    }
}

/// What a row is, which decides the letter its icon draws.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Kind {
    /// The whole library.
    All,
    /// A deck.
    #[default]
    Deck,
    /// A saved view.
    View,
    /// A tag.
    Tag,
    /// Importing a file.
    Import,
    /// The settings screen.
    Settings,
    /// The trash.
    Trash,
}

/// What one row shows.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Row {
    /// The element key, which is also what a test presses.
    pub key: String,
    /// The words on the row.
    pub label: String,
    /// What the row is, which its icon draws.
    pub kind: Kind,
    /// Whether the row is the one in force.
    pub selected: bool,
    /// A row whose change has not landed yet: it draws, and pressing it does nothing.
    pub disabled: bool,
}

impl Row {
    /// A row that is neither selected nor disabled.
    pub fn new(key: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            kind: Kind::default(),
            selected: false,
            disabled: false,
        }
    }

    /// What the row is.
    pub fn kind(mut self, kind: Kind) -> Self {
        self.kind = kind;
        self
    }

    /// The row in force.
    pub fn selected(mut self) -> Self {
        self.selected = true;
        self
    }

    /// A row waiting on a change that has not landed.
    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }
}

/// A group's heading: small, uppercase, quiet, in a 27-point band.
pub fn heading(group: Group) -> El {
    let t = tok::theme(true);
    col()
        .key(format!("{}-title", group.key()))
        .w_full()
        .h(27.0)
        .px(14.0)
        .center_y()
        .child(
            text(group.label().to_uppercase())
                .font(type_::MONO_CAPTION)
                .color(t.text_secondary),
        )
}

/// A group of rows under its heading, or nothing at all when it holds no row: an empty group
/// leaves no heading and no gap behind.
pub fn group(group: Group, rows: Vec<El>) -> Option<El> {
    if rows.is_empty() {
        return None;
    }
    let mut section = col().key(group.key()).w_full().child(heading(group));
    for item in rows {
        section = section.child(item);
    }
    Some(section)
}

/// One row: a filled, rounded body, an icon slot, and a label at [`LABEL_X`].
///
/// The row is 25 points tall and as wide as it is offered, so the column insets its children
/// rather than the row insetting itself. Hover draws an overlay, not another row.
pub fn row<S: 'static>(item: &Row, on_press: impl Fn(&mut S) + 'static) -> El {
    let t = tok::theme(true);
    let selected = item.selected;
    let disabled = item.disabled;
    let press = Rc::new(on_press);
    rune::row()
        .key(item.key.clone())
        .w_full()
        .h(ROW_HEIGHT)
        .radius(5.0)
        .center_y()
        .gap(GAP_AFTER_ICON)
        .fill(if selected {
            tok::accent(0, true)
        } else {
            t.control_off.alpha(0.0)
        })
        .hover(if selected {
            tok::accent(0, true)
        } else {
            t.text.alpha(0.06)
        })
        .child(icon(item.kind, &item.label))
        .child(
            text(item.label.clone())
                .key(format!("{}-label", item.key))
                .font(type_::LABEL)
                .color(if selected {
                    rune::Color::WHITE
                } else if disabled {
                    t.text_secondary
                } else {
                    t.text
                }),
        )
        .on_press(move |s: &mut S| {
            if !disabled {
                press(s);
            }
        })
}

/// The colour an icon of this name is tinted with: one of the six accents, chosen by the name
/// rather than by position, so a deck keeps its colour when another deck is added above it.
pub fn tint(name: &str) -> rune::Color {
    tok::accent(hash(name), true)
}

/// A 20-point icon box: a monogram letter on a tinted ground.
pub fn icon(kind: Kind, name: &str) -> El {
    let tint = tint(name);
    col()
        .key(format!("icon-{name}"))
        .w(ICON)
        .h(ICON)
        .radius(5.0)
        .fill(tint.alpha(0.18))
        .center()
        .child(
            text(monogram(kind, name))
                .font(type_::MONO_CAPTION)
                .color(tint),
        )
}

/// The one letter an icon draws: the first thing in the row's words that is a letter or a
/// digit, uppercased, except for a tag, which draws its own mark.
fn monogram(kind: Kind, name: &str) -> String {
    if kind == Kind::Tag || name.trim_start().starts_with('#') {
        return "#".to_string();
    }
    name.chars()
        .find(|c| c.is_alphanumeric())
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_else(|| "?".to_string())
}

/// FNV-1a over the name's bytes, folded into the palette's six slots.
fn hash(name: &str) -> usize {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in name.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    usize::try_from(hash % 6).unwrap_or(0)
}

#[cfg(test)]
mod tests;
