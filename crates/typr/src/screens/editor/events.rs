//! Shared editor value types and events reported out of the editor component.

use nsh_core::element::JsObject;
use nsh_core::prelude::*;
use typr_core::store::{MaskRect, MaskRow};

use super::fields::TemplateField;

/// A rectangle as fractions of the picture.
#[derive(Clone, Copy, Debug, Default, PartialEq, JsObject)]
#[non_exhaustive]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub(super) fn of(rect: MaskRect) -> Rect {
        Rect {
            x: rect.x,
            y: rect.y,
            w: rect.w,
            h: rect.h,
        }
    }

    pub(super) fn rect(self) -> MaskRect {
        MaskRect::new(self.x, self.y, self.w, self.h)
    }
}

/// One cover on a diagram's picture: the group it belongs to, and its rectangle as fractions of
/// the picture.
#[derive(Clone, Copy, Debug, Default, PartialEq, JsObject)]
#[non_exhaustive]
pub struct Cover {
    pub group: u32,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Cover {
    /// The cover `group` draws over `rect`, both as fractions of the picture.
    pub fn new(group: u32, rect: MaskRect) -> Cover {
        Cover {
            group,
            x: rect.x,
            y: rect.y,
            w: rect.w,
            h: rect.h,
        }
    }

    /// A store row as the pane draws it.
    pub fn of(row: &MaskRow) -> Cover {
        Cover {
            group: row.group,
            x: row.rect.x,
            y: row.rect.y,
            w: row.rect.w,
            h: row.rect.h,
        }
    }

    /// The rectangle the store holds.
    pub(super) fn rect(&self) -> MaskRect {
        Rect {
            x: self.x,
            y: self.y,
            w: self.w,
            h: self.h,
        }
        .rect()
    }
}

/// A diagram card's picture, its caption, and the rectangles drawn on it.
#[derive(Clone, Debug, Default, PartialEq, JsObject)]
#[non_exhaustive]
pub struct Diagram {
    pub image: String,
    pub caption: String,
    pub rows: Vec<Cover>,
}

impl Diagram {
    /// A diagram showing `image`, with an empty caption and no masks yet.
    pub fn new(image: impl Into<String>) -> Self {
        Self {
            image: image.into(),
            ..Self::default()
        }
    }
}

/// A template in force on the card: its text, the fields it fills from, and the deck's name.
#[derive(Clone, Debug, Default, PartialEq, JsObject)]
#[non_exhaustive]
pub struct Template {
    pub text: String,
    pub fields: Vec<TemplateField>,
    pub deck: Option<String>,
}

impl Template {
    /// A template whose text is `text`, with no fields filled and no deck named.
    pub fn new(text: impl Into<String>) -> Template {
        Template {
            text: text.into(),
            ..Template::default()
        }
    }
}

/// The card's text, as the editor reports it.
#[derive(Clone, Debug, Default, PartialEq, JsObject)]
#[non_exhaustive]
pub struct CardText {
    /// The whole text.
    pub text: String,
}

/// The fields of a templated card, as the user has them.
#[derive(Clone, Debug, Default, PartialEq, JsObject)]
#[non_exhaustive]
pub struct FieldValues {
    pub fields: Vec<TemplateField>,
}

/// One rectangle dragged onto the picture, and the group it joins.
#[derive(Clone, Copy, Debug, Default, PartialEq, JsObject)]
#[non_exhaustive]
pub struct MaskAdded {
    /// The cover the drag drew.
    pub mask: Cover,
}

/// One group's rectangles, to be replaced by the one that holds them all.
#[derive(Clone, Debug, Default, PartialEq, JsObject)]
#[non_exhaustive]
pub struct GroupMerge {
    /// The group the rectangles belong to.
    pub group: u32,
    /// Every rectangle of that group.
    pub rects: Vec<Rect>,
}

/// One group of covers, to be let go.
#[derive(Clone, Copy, Debug, Default, PartialEq, JsObject)]
#[non_exhaustive]
pub struct MaskGroup {
    /// The group's number.
    pub group: u32,
}

/// The picture a card becomes a diagram of.
#[derive(Clone, Debug, Default, PartialEq, JsObject)]
#[non_exhaustive]
pub struct DiagramStart {
    /// The picture's name, already copied beside the library.
    pub name: String,
}

/// What the editor reports: the card's text, and every edit to what it draws. The caller writes
/// them down; the editor holds nothing of the store.
#[derive(Event)]
pub enum EditorEvent {
    /// The text, to write down as the card.
    Save(CardText),
    /// A templated card's fields, to write down so its text is rendered from them.
    SaveFields(FieldValues),
    /// The text as it is typed, to keep as the draft.
    Draft(CardText),
    /// Leaving, with nothing unsaved.
    Leave,
    /// Leaving without the draft: the in-progress text is thrown away.
    Discard,
    /// A rectangle dragged on the picture.
    AddMask(MaskAdded),
    /// One group's rectangles to replace with the one that holds them all.
    MergeMasks(GroupMerge),
    /// Every rectangle of a group let go.
    RemoveMaskGroup(MaskGroup),
    /// The picture and every rectangle let go; the card is text again.
    ClearDiagram,
    /// The card becomes a diagram of the picture copied under `name`.
    StartDiagram(DiagramStart),
}
