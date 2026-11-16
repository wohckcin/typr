//! The NSH app's root and its shell: the sidebar beside the work area. Pages and screens join the
//! shell as they are ported.
//!
//! The sidebar's Settings row opens the settings screen in the work area; Escape returns to the
//! library. Anything else keeps the work area empty until its screen is wired.

use nsh_core::prelude::*;
use typr_core::clock::{Clock, SystemClock};
use typr_core::store::Snapshot;

use crate::screens::settings::{LibraryInfo, Values, settings};
use crate::screens::sidebar::{Sidebar, due_counts, items_from, search_chord, sidebar};
use crate::ui::{surface, tokens};

/// The shell over `snapshot`: `library_open` says whether the store opened, which the sidebar
/// states. Tests drive this with a fixture; the app drives it with the real library.
pub fn shell(snapshot: Snapshot, library_open: bool) -> impl IntoElement {
    shell_with(snapshot, library_open, LibraryInfo::default())
}

/// The shell with the library's own file known, so the settings screen can report its paths.
pub fn shell_with(snapshot: Snapshot, library_open: bool, library: LibraryInfo) -> impl IntoElement {
    let due = due_counts(&snapshot, SystemClock.now_ms());
    let state = Sidebar::new(items_from(&snapshot, &due), library_open, search_chord());
    let cap = snapshot.settings.new_per_day;
    // Escape leaves the screen the work area is showing, back to the whole library. The library
    // table binds no Leave command, so the key is the one every screen uses for it.
    on_key("escape", move |_| state.select(&"all".into()));
    view()
        .flex_1()
        .flex_row()
        .bg(move || tokens::background())
        .child(sidebar(state))
        .child(view().flex_1().child(dynamic(move || {
            let showing_settings = state.selected_key().as_deref() == Some("settings");
            if showing_settings {
                // The screen's signals belong to this branch, so leaving it disposes them with it.
                return settings(Values::new(cap, true), library.clone(), |_| {}, || {}, || {}).into_element();
            }
            view().p_4().child(surface::separator()).into_element()
        })))
}

/// The app's root: the real library when it opens, and an empty, closed one when it does not, so
/// the sidebar says so rather than hiding it.
#[component]
pub fn App() -> impl IntoElement {
    crate::ui::tokens::install();
    match crate::paths::open_from_environment() {
        Ok(db) => {
            let library = LibraryInfo::of(&db);
            match db.snapshot() {
                Ok(snapshot) => shell_with(snapshot, true, library).into_element(),
                Err(error) => {
                log::error!("library snapshot failed: {}", crate::logging::describe(&error));
                shell_with(Snapshot::default(), false, library).into_element()
            }
            }
        }
        Err(error) => {
            log::error!("library did not open: {}", crate::logging::describe(&error));
            shell(Snapshot::default(), false).into_element()
        }
    }
}
