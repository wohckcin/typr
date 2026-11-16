//! The NSH screens. Each file is one screen, built only from `crate::ui` components and the tokens.

pub mod diagnostics;
pub mod editor;
pub mod embed;
pub mod grid;
pub mod history;
pub mod keymap;
pub mod library;
pub mod settings;
pub mod sidebar;
pub mod study;

/// Every screen module this crate ports, by file name. The appearance check compares its own list
/// with this one, so a screen added here without a light/dark check fails by name.
pub const NAMES: &[&str] = &[
    "diagnostics",
    "editor",
    "embed",
    "grid",
    "history",
    "keymap",
    "library",
    "settings",
    "sidebar",
    "study",
];
