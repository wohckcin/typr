//! The app's styled layer: the one place a control's look comes from.
//!
//! `nsh_base` supplies behaviour and no appearance, so every screen draws with a component from
//! here ([`button`], [`input`], ...) rather than with a base control. Colours, radii and durations
//! are named in [`tokens`] and [`motion`]; a screen that writes a value of its own is the defect
//! `tests/tokens.rs` looks for.

pub mod button;
pub mod choice;
pub mod overlay;
pub mod prose;
pub mod input;
pub mod status;
pub mod surface;
pub mod toggle;
pub mod motion;
pub mod tokens;
