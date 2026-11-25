//! Library grid selection, keyboard stepping, trashing, and Escape handling.

use nsh_core::keymap::When;
use nsh_core::prelude::*;
use typr_core::clock::{Clock, SystemClock};
use typr_core::edits::selection_after_trash;
use typr_core::store::{Command, Id, Snapshot};

use crate::screens::library::{cards_in, tiles_for};
use crate::screens::sidebar::Sidebar;

use super::screen::Screen;
use super::store_bridge::StoreBridge;

/// Owns the library screen's visible selection and search focus.
#[derive(Clone)]
pub(super) struct LibraryNav {
    snapshot_signal: Signal<Snapshot>,
    state: Sidebar,
    query: Signal<String>,
    selection: Signal<Option<Id>>,
    screen: Signal<Screen>,
    search_handle: ElementRef,
    bridge: StoreBridge,
}

impl LibraryNav {
    pub(super) fn new(
        snapshot_signal: Signal<Snapshot>,
        state: Sidebar,
        query: Signal<String>,
        selection: Signal<Option<Id>>,
        screen: Signal<Screen>,
        search_handle: ElementRef,
        bridge: StoreBridge,
    ) -> Self {
        let nav = Self {
            snapshot_signal,
            state,
            query,
            selection,
            screen,
            search_handle,
            bridge,
        };
        nav.install_selection_pruner();
        nav.bind_escape();
        nav
    }

    /// The cards the grid shows now, in the order it shows them: the keyboard walks this list.
    pub(super) fn visible(&self) -> Vec<Id> {
        let snapshot = self.snapshot_signal.get_untracked();
        let now = SystemClock.now_ms();
        let cards = cards_in(&snapshot, self.state.selected_key().as_deref(), now);
        tiles_for(&cards, &self.query.get_untracked(), now)
            .into_iter()
            .map(|tile| tile.id)
            .collect()
    }

    /// Moves the selection by `delta` steps in the visible list.
    pub(super) fn step(&self, delta: isize) {
        let ids = self.visible();
        let Some(last) = ids.len().checked_sub(1) else {
            self.selection.set(None);
            return;
        };
        let at = self
            .selection
            .get_untracked()
            .and_then(|id| ids.iter().position(|each| *each == id));
        let next = match at {
            Some(at) => (at as isize + delta).clamp(0, last as isize) as usize,
            None if delta > 0 => 0,
            None => last,
        };
        self.selection.set(Some(ids[next]));
    }

    /// Moves the selection to the first or last visible card.
    pub(super) fn select_end(&self, first: bool) {
        let ids = self.visible();
        let chosen = if first { ids.first() } else { ids.last() };
        self.selection.set(chosen.copied());
    }

    /// Trashes the selected card and moves selection to its surviving neighbor.
    pub(super) fn delete_card(&self) {
        let Some(id) = self.selection.get_untracked() else {
            return;
        };
        let before = self.visible();
        if !before.contains(&id) {
            return;
        }
        let nav = self.clone();
        self.bridge.submit(
            Command::TrashCard { id },
            true,
            Some(Box::new(move |result| {
                if result.is_ok() {
                    nav.selection
                        .set(selection_after_trash(&before, id, &nav.visible()));
                }
            })),
        );
    }

    /// Focuses the library search input.
    pub(super) fn focus_search(&self) {
        self.search_handle.focus();
    }

    /// The selected card leaves the list when a filter stops matching it: nothing stays highlighted.
    fn install_selection_pruner(&self) {
        let nav = self.clone();
        effect(move || {
            let Some(id) = nav.selection.get() else {
                return;
            };
            nav.snapshot_signal.get();
            nav.query.get();
            nav.state.selected_key();
            if !nav.visible().contains(&id) {
                nav.selection.set(None);
            }
        });
    }

    /// Escape returns to the whole library from the grid; while a card is open the editor binds it,
    /// and this one lets the key through so both can hear it.
    fn bind_escape(&self) {
        let screen = self.screen;
        let state = self.state;
        on_key_with(
            "escape",
            KeyOptions {
                pass_through: true,
                when: When::Always,
                ..KeyOptions::default()
            },
            move |_| match screen.get() {
                // Escape cancels a deck being named before it does anything else.
                Screen::Library if state.is_naming() => state.stop_naming(),
                // Escape narrows the grid here, and the editor and the session bind their own Leave.
                Screen::Library => state.select(&"inbox".into()),
                Screen::Settings | Screen::Diagnostics | Screen::Trash => {
                    state.select(&"inbox".into());
                    screen.set(Screen::Library);
                }
                Screen::Editor(_) | Screen::Study(_) => {}
            },
        );
    }
}
