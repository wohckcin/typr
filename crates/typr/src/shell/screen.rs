//! Active work-area screen state shared across the shell controllers.

use std::cell::RefCell;
use std::rc::Rc;

use nsh_core::prelude::SharedString;
use typr_core::session::Session;
use typr_core::store::Id;

/// What the work area shows. One value owns it, so a key can only act where its screen is showing
/// and a screen can only be entered by setting it.
#[derive(Clone)]
pub(super) enum Screen {
    /// The grid, filtered by the sidebar's row in force.
    Library,
    /// One card's editor.
    Editor(Id),
    /// The library's paths and defaults.
    Settings,
    /// What the store and the slow commands have done since launch.
    Diagnostics,
    /// The cards that were deleted and can still be restored.
    Trash,
    /// A session, shared rather than taken, so a rebuild keeps the session it is showing.
    Study(Rc<RefCell<Session>>),
}

impl PartialEq for Screen {
    /// Two screens are the same when they show the same thing; a session is the same session while
    /// its handle matches, so a rebuild of the work area is not a new screen.
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Screen::Library, Screen::Library)
            | (Screen::Settings, Screen::Settings)
            | (Screen::Diagnostics, Screen::Diagnostics)
            | (Screen::Trash, Screen::Trash) => true,
            (Screen::Editor(one), Screen::Editor(two)) => one == two,
            (Screen::Study(one), Screen::Study(two)) => Rc::ptr_eq(one, two),
            _ => false,
        }
    }
}

/// The screen a sidebar row opens: Settings and Trash are screens, every other row filters the grid.
pub(super) fn screen_for(key: &SharedString) -> Screen {
    match key.as_ref() {
        "settings" => Screen::Settings,
        "trash" => Screen::Trash,
        _ => Screen::Library,
    }
}
