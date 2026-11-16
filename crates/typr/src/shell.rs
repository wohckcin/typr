//! The NSH app's root and its shell: the sidebar beside the work area. Pages and screens join the
//! shell as they are ported.
//!
//! The sidebar's Settings row opens the settings screen in the work area; Escape returns to the
//! library. Anything else keeps the work area empty until its screen is wired.

use std::rc::Rc;

use nsh_core::keymap::When;
use nsh_core::prelude::*;
use typr_core::clock::{Clock, SystemClock};
use typr_core::store::{Command, Db, Id, Snapshot};

use crate::screens::editor::{self, Actions, Card, Diagram, Field, Template};
use crate::screens::grid::grid;
use crate::screens::library::{cards_in, tiles_for};
use crate::screens::settings::{LibraryInfo, Values, settings};
use crate::screens::sidebar::{Sidebar, due_counts, items_from, search_chord, sidebar};
use crate::ui::{surface, tokens};

/// Where the work area is: the library's grid, a card's editor, or nothing of its own (settings).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Route {
    #[default]
    Library,
    Editor(Id),
}

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

/// The shell over a live library: the grid opens a card's editor, and the editor's actions are
/// store commands. `db` is shared, so every action sees the same library the snapshot came from.
pub fn shell_over(db: Rc<Db>, snapshot: Snapshot, library_open: bool, library: LibraryInfo) -> impl IntoElement {
    let snapshot_signal = signal(snapshot);
    let route = signal(Route::Library);
    let due = due_counts(&snapshot_signal.get_untracked(), SystemClock.now_ms());
    let state = Sidebar::new(items_from(&snapshot_signal.get_untracked(), &due), library_open, search_chord());
    let cap = snapshot_signal.with(|s| s.settings.new_per_day);
    let attachments = library.attachments.clone();

    // One writer for the store: a command, then the snapshot a view reads. `quiet` skips the
    // re-publish for draft writes, which come once a keystroke and must not rebuild the editor.
    let write = {
        let db = db.clone();
        let snapshot_signal = snapshot_signal;
        Rc::new(move |command: Command, quiet: bool| {
            match db.apply(&command, SystemClock.now_ms()) {
                Ok(_) => {
                    if !quiet {
                        if let Ok(next) = db.snapshot() {
                            snapshot_signal.set(next);
                        }
                    }
                }
                Err(error) => log::error!("store refused a command: {}", crate::logging::describe(&error)),
            }
        })
    };

    let open = {
        let route = route;
        move |id: Id| route.set(Route::Editor(id))
    };

    // Escape returns to the whole library from the grid; while a card is open the editor binds it,
    // and this one lets the key through so both can hear it.
    let route_for_key = route;
    on_key_with("escape", KeyOptions { pass_through: true, when: When::Always, ..KeyOptions::default() }, move |_| {
        if route_for_key.get() == Route::Library {
            state.select(&"all".into());
        }
    });

    view()
        .flex_1()
        .flex_row()
        .bg(move || tokens::background())
        .child(sidebar(state))
        .child(view().flex_1().child(dynamic(move || {
            if state.selected_key().as_deref() == Some("settings") {
                return settings(Values::new(cap, true), library.clone(), |_| {}, || {}, || {}).into_element();
            }
            match route.get() {
                Route::Editor(id) => {
                    let found = snapshot_signal.with(|snapshot| snapshot.cards.iter().find(|card| card.id == id).cloned());
                    let Some(card) = found else {
                        route.set(Route::Library);
                        return view().into_element();
                    };
                    let template = card.template.as_ref().map(|text| Template {
                        text: text.clone(),
                        fields: card
                            .fields
                            .iter()
                            .map(|(name, value)| Field { name: name.clone(), value: value.clone(), computed: false })
                            .collect(),
                        deck: None,
                    });
                    let diagram = (card.kind == typr_core::store::CardKind::Diagram).then(|| Diagram {
                        image: card.image.clone().unwrap_or_default(),
                        caption: card.caption.clone().unwrap_or_default(),
                        rows: card.masks.clone(),
                    });
                    let draft = db.draft(id).ok().flatten();
                    let write_save = write.clone();
                    let write_draft = write.clone();
                    let write_mask = write.clone();
                    let write_merge = write.clone();
                    let write_remove = write.clone();
                    let leaving = route;
                    let actions = Actions::new()
                        .save(move |markdown| {
                            write_save(Command::SaveCard { id, markdown }, false);
                            write_save(Command::SetDraft { card: id, text: None }, true);
                        })
                        .draft(move |text| {
                            write_draft(Command::SetDraft { card: id, text: Some(text.to_owned()) }, true)
                        })
                        .leave(move || leaving.set(Route::Library))
                        .discard({
                            let write = write.clone();
                            move || {
                                // Discarding drops the stored draft, so the next open is the card.
                                write(Command::SetDraft { card: id, text: None }, true);
                                leaving.set(Route::Library);
                            }
                        })
                        .attach({
                            let root = attachments.clone();
                            move |path| editor::attach_image(&path, &root)
                        })
                        .masks(
                            move |group, rect| write_mask(Command::AddMask { card: id, group, rect }, false),
                            move |group, rects| write_merge(Command::MergeMasks { card: id, group, rects }, false),
                            move |rect| write_remove(Command::RemoveMask { card: id, rect }, false),
                        )
                        .kinds(
                            {
                                let write = write.clone();
                                move |group| {
                                    write(Command::RemoveMaskGroup { card: id, group }, false)
                                }
                            },
                            {
                                let write = write.clone();
                                move || write(Command::ClearDiagram { card: id }, false)
                            },
                            {
                                let write = write.clone();
                                move |image| {
                                    write(
                                        Command::SetDiagram {
                                            card: id,
                                            image,
                                            caption: String::new(),
                                        },
                                        false,
                                    )
                                }
                            },
                        );
                    editor::editor_card(
                        Card { saved: card.markdown.clone(), draft, template, diagram },
                        attachments.clone(),
                        actions,
                    )
                    .into_element()
                }
                Route::Library => {
                    // The work area's grid: the sidebar's selection chooses the cards, and a tile
                    // opens one.
                    let for_tiles = snapshot_signal;
                    let chosen = state;
                    let now = SystemClock.now_ms();
                    let tiles = computed(move || {
                        let snapshot = for_tiles.get();
                        let cards = cards_in(&snapshot, chosen.selected_key().as_deref());
                        tiles_for(&cards, "", now)
                    });
                    view()
                        .flex_1()
                        .p_4()
                        .gap_3()
                        .child(grid(tiles, || {}, open.clone()))
                        .into_element()
                }
            }
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
            let db = Rc::new(db);
            match db.snapshot() {
                Ok(snapshot) => shell_over(db.clone(), snapshot, true, library).into_element(),
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
