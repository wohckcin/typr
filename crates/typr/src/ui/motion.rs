//! How long a change takes, in milliseconds, so no component writes a duration of its own.
//!
//! The values are the Design Guides' starting set: a hover wash is nearly immediate, a press is a
//! control's own beat, something moving into place takes about a quarter second, and a reveal is
//! unhurried. Leaving is faster than entering, which is why the components pick a shorter token
//! for the way out.

/// A hover wash, a colour change.
pub const INSTANT: f32 = 120.0;

/// A press, a control's state.
pub const FAST: f32 = 180.0;

/// Moving into place: a selection sliding, an icon swapping.
pub const BASE: f32 = 240.0;

/// A reveal: a dialog or a menu opening, a pop.
pub const SLOW: f32 = 360.0;

/// An unhurried entrance: a list's rows arriving as a wave.
pub const SLOWER: f32 = 500.0;

/// How far a label rises when it changes, in points.
pub const RISE: f32 = 4.0;

/// How far a popup slides toward its trigger, in points.
pub const SLIDE: f32 = 8.0;

/// How far content travels when it enters a page, in points.
pub const ENTER: f32 = 16.0;

/// The scale a control presses to.
pub const PRESS_SCALE: f32 = 0.97;

/// The scale an icon button presses to.
pub const PRESS_SCALE_ICON: f32 = 0.95;

/// The scale a surface starts at when it enters.
pub const ENTER_SCALE: f32 = 0.97;
