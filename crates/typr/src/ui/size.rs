//! The app's control sizes: one value that sets a control's height, horizontal padding, text size
//! and radius together.
//!
//! These are the app's own control geometry, not spacing: the numbers are the audited physical
//! frames a pointer control needs (the design guides' 24, 32 and 40 point tiers), so they live here
//! rather than at a call site. Everything else a screen draws uses the spacing and type scale.

use nsh_core::element::{PropEnum, Text};
use nsh_core::prelude::*;

use super::tokens;

/// A control's size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PropEnum)]
pub enum Size {
    /// A dense toolbar control.
    Sm,
    /// The ordinary control.
    #[default]
    Md,
    /// A comfortable control for a sparse form.
    Lg,
}

impl Size {
    /// The height a control is at least, so a larger text size grows the frame.
    pub fn min_height(self) -> Points {
        match self {
            Size::Sm => px(24.0),
            Size::Md => px(32.0),
            Size::Lg => px(40.0),
        }
    }

    /// The horizontal padding inside a control.
    pub fn pad_x(self) -> Points {
        match self {
            Size::Sm => px(8.0),
            Size::Md => px(12.0),
            Size::Lg => px(16.0),
        }
    }

    /// The vertical padding inside a control, so text keeps its room once the frame grows.
    pub fn pad_y(self) -> Points {
        match self {
            Size::Sm => px(2.0),
            Size::Md => px(4.0),
            Size::Lg => px(6.0),
        }
    }

    /// The radius a control of this size wears: small controls take half the theme's.
    pub fn radius(self) -> Points {
        let radius = tokens::radius();
        match self {
            Size::Sm => Points(radius.0 / 2.0),
            Size::Md | Size::Lg => radius,
        }
    }

    /// A label at this size's step of the type scale.
    pub fn text(self, label: impl IntoReactive<SharedString>) -> Text {
        match self {
            Size::Sm => text(label).text_xs(),
            Size::Md => text(label).text_sm(),
            Size::Lg => text(label).text_base(),
        }
    }
}
