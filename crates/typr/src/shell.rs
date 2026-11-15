//! The NSH app's root and its shell: the sidebar beside the work area. Pages and screens join the
//! shell as they are ported.

use nsh_core::prelude::*;
use typr_core::clock::{Clock, SystemClock};
use typr_core::store::Snapshot;

use crate::screens::sidebar::{Sidebar, due_counts, items_from, search_chord, sidebar};
use crate::ui::{surface, tokens};

/// The shell over `snapshot`: `library_open` says whether the store opened, which the sidebar
/// states. Tests drive this with a fixture; the app drives it with the real library.
pub fn shell(snapshot: Snapshot, library_open: bool) -> impl IntoElement {
    let due = due_counts(&snapshot, SystemClock.now_ms());
    let state = Sidebar::new(items_from(&snapshot, &due), library_open, search_chord());
    view()
        .flex_1()
        .flex_row()
        .bg(move || tokens::background())
        .child(sidebar(state))
        .child(view().flex_1().p_4().child(surface::separator()))
}

/// The app's root: the real library when it opens, and an empty, closed one when it does not, so
/// the sidebar says so rather than hiding it.
#[component]
pub fn App() -> impl IntoElement {
    crate::ui::tokens::install();
    match crate::paths::open_from_environment() {
        Ok(db) => match db.snapshot() {
            Ok(snapshot) => shell(snapshot, true).into_element(),
            Err(_) => shell(Snapshot::default(), false).into_element(),
        },
        Err(_) => shell(Snapshot::default(), false).into_element(),
    }
}
