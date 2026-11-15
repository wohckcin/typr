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
    /// Import, settings and the trash.
    Utilities,
}

impl Group {
    /// Every group, in drawing order.
    pub const ALL: [Group; 3] = [Group::Navigation, Group::Decks, Group::Utilities];

    /// The element key of the group's container.
    pub fn key(self) -> &'static str {
        match self {
            Group::Navigation => "sidebar-navigation",
            Group::Decks => "sidebar-decks",
            Group::Utilities => "sidebar-utilities",
        }
    }

    /// The heading the group draws above its rows.
    pub fn label(self) -> &'static str {
        match self {
            Group::Navigation => "Navigation",
            Group::Decks => "Decks",
            Group::Utilities => "Utilities",
        }
    }
}

/// What one row shows.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Row {
    /// The element key, which is also what a test presses.
    pub key: String,
    /// The words on the row.
    pub label: String,
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
            selected: false,
            disabled: false,
        }
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
        .child(col().w(ICON).h(ICON))
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

#[cfg(test)]
mod tests;
