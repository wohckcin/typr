//! The NSH app's root and its shell: the sidebar beside the work area. Pages and screens join the
//! shell as they are ported.
//!
//! The sidebar's Settings row opens the settings screen in the work area; Escape returns to the
//! library. Anything else keeps the work area empty until its screen is wired.

use std::rc::Rc;

use nsh_core::file_dialog::{OpenDialog, SaveDialog};
use nsh_core::keymap::When;
use nsh_core::prelude::*;
use typr_core::clock::{Clock, SystemClock};
use typr_core::keymap::{Command as Key, Surface};
use typr_core::session::{Entry, Session};
use typr_core::store::{Command, Db, Grade, Id, Outcome, Snapshot};

use crate::screens::diagnostics::{SlowCommand, diagnostics};
use crate::screens::editor::{self, Actions, Card, Diagram, Field, Template};
use crate::screens::grid::grid;
use crate::screens::library::{cards_in, tiles_for};
use crate::screens::settings::{LibraryInfo, Values, settings};
use crate::screens::sidebar::{Sidebar, due_counts, items_from, search_chord, sidebar};
use crate::screens::study::{entries_for, study};
use crate::ui::{
    input::{field, field_reachable},
    tokens,
};

/// How long a store command may take before the diagnostics screen lists it, in milliseconds.
const SLOW_COMMAND_MS: u32 = 100;

/// Exports the library to `target` and gives the line the diagnostics screen states.
pub fn export_to(db: &Db, target: &std::path::Path) -> String {
    match db.apply(
        &Command::ExportLibrary {
            target: target.to_path_buf(),
        },
        SystemClock.now_ms(),
    ) {
        Ok(_) => format!("Exported to {}", target.display()),
        Err(error) => crate::logging::describe(&error),
    }
}

/// Imports the Mochi archive at `archive` and gives the line the diagnostics screen states: what
/// arrived, or why the file was refused.
pub fn import_from(db: &Db, archive: &std::path::Path, attachments: &std::path::Path) -> String {
    let command = Command::ImportArchive {
        archive: archive.to_path_buf(),
        attachments: attachments.to_path_buf(),
    };
    match db.apply(&command, SystemClock.now_ms()) {
        Ok(typr_core::store::Outcome::Imported(report)) => format!(
            "Imported {} decks, {} cards, {} reviews and {} pictures from {}",
            report.decks,
            report.cards,
            report.reviews,
            report.images,
            archive
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("the archive")
        ),
        Ok(_) => "Imported".to_owned(),
        Err(error) => crate::logging::describe(&error),
    }
}

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
pub fn shell_with(
    snapshot: Snapshot,
    library_open: bool,
    library: LibraryInfo,
) -> impl IntoElement {
    let due = due_counts(&snapshot, SystemClock.now_ms());
    let state = Sidebar::new(items_from(&snapshot, &due), library_open, search_chord());
    let cap = snapshot.settings.new_per_day;
    // Without a store the screen still shows the library it read: a blank work area would leave a
    // reader guessing whether the app works.
    let snapshot = Rc::new(snapshot);
    let query = signal(String::new());
    let chosen = signal(0_usize);
    let tiles = {
        let snapshot = snapshot.clone();
        let with = state;
        computed(move || {
            let cards = cards_in(&snapshot, with.selected_key().as_deref());
            tiles_for(&cards, &query.get(), SystemClock.now_ms())
        })
    };
    // Escape leaves the screen the work area is showing, back to the whole library. The library
    // table binds no Leave command, so the key is the one every screen uses for it.
    on_key("escape", move |_| state.select(&"all".into()));
    view()
        .flex_1()
        .flex_row()
        .bg(tokens::background)
        .child(sidebar(state))
        .child(view().flex_1().child(dynamic(move || {
            let showing_settings = state.selected_key().as_deref() == Some("settings");
            if showing_settings {
                // The screen's signals belong to this branch, so leaving it disposes them with it.
                return settings(
                    Values::new(cap, true),
                    library.clone(),
                    |_| {},
                    || {},
                    || {},
                )
                .into_element();
            }
            view()
                .flex_1()
                .p_4()
                .gap_2()
                .child(field("Search cards", query))
                .child(grid(tiles, chosen, || {}, |_| {}))
                .into_element()
        })))
}

/// The shell over a live library: the grid opens a card's editor, and the editor's actions are
/// store commands. `db` is shared, so every action sees the same library the snapshot came from.
pub fn shell_over(
    db: Rc<Db>,
    snapshot: Snapshot,
    library_open: bool,
    library: LibraryInfo,
) -> impl IntoElement {
    let snapshot_signal = signal(snapshot);
    let route = signal(Route::Library);
    let due = due_counts(&snapshot_signal.get_untracked(), SystemClock.now_ms());
    let state = Sidebar::new(
        items_from(&snapshot_signal.get_untracked(), &due),
        library_open,
        search_chord(),
    );
    let cap = snapshot_signal.with(|s| s.settings.new_per_day);
    let attachments = library.attachments.clone();
    // What the diagnostics screen states: a stoppage at any write since launch, and the commands
    // that took long enough to feel.
    let stopped = signal(!library_open);
    // The library's search: the sidebar states the chord, and this field takes it.
    let query = signal(String::new());
    let search_handle = element_ref();
    // The library's keyboard: the card Enter acts on, and the session that may be running.
    let selection = signal(0_usize);
    let studying = signal(false);
    let session_slot: Rc<std::cell::RefCell<Option<Session>>> =
        Rc::new(std::cell::RefCell::new(None));
    let reopen = {
        let db = db.clone();
        Rc::new(move || {
            if let Ok(next) = db.snapshot() {
                snapshot_signal.set(next);
            }
        })
    };
    // The cards the grid shows now, in the order it shows them: the keyboard walks this list.
    let visible = {
        let db = db.clone();
        Rc::new(move || {
            let _ = &db;
            let snapshot = snapshot_signal.get_untracked();
            let cards = cards_in(&snapshot, state.selected_key().as_deref());
            tiles_for(&cards, &query.get_untracked(), SystemClock.now_ms())
                .into_iter()
                .map(|tile| tile.id)
                .collect::<Vec<Id>>()
        })
    };
    let open_selected = {
        let visible = visible.clone();
        Rc::new(move || {
            let ids = visible();
            if let Some(id) = ids.get(selection.get_untracked()) {
                route.set(Route::Editor(*id));
            }
        })
    };
    // A session: the store serves the queue, the screen walks it, and each grade goes back through
    // the store quietly, so the session is not rebuilt under the user as it is graded.
    let start_session = {
        let db = db.clone();
        let session_slot = session_slot.clone();
        Rc::new(move |queue: typr_core::screen::Queue| {
            let now = SystemClock.now_ms();
            let offset = crate::day_offset::local_offset_ms();
            let seed = now as u64;
            let command = match queue {
                typr_core::screen::Queue::Due => Command::DuePrompts {
                    now_ms: now,
                    offset_ms: offset,
                    seed,
                    deck: None,
                },
                typr_core::screen::Queue::New => Command::NewPrompts {
                    now_ms: now,
                    offset_ms: offset,
                    seed,
                    deck: None,
                },
            };
            let pairs: Vec<(Id, String, i64)> = match db.apply(&command, now) {
                Ok(Outcome::Due(prompts)) => prompts
                    .into_iter()
                    .map(|p| (p.card, p.key, (now - p.due_at).max(0)))
                    .collect(),
                Ok(Outcome::New(prompts)) => {
                    prompts.into_iter().map(|p| (p.card, p.key, 0)).collect()
                }
                Ok(_) => Vec::new(),
                Err(error) => {
                    log::error!(
                        "the queue was refused: {}",
                        crate::logging::describe(&error)
                    );
                    Vec::new()
                }
            };
            let cards = snapshot_signal.get_untracked().cards;
            let entries: Vec<Entry> = entries_for(&cards, &pairs);
            if entries.is_empty() {
                return;
            }
            *session_slot.borrow_mut() = Some(Session::new(queue, entries));
            studying.set(true);
        })
    };
    let slow = signal(Vec::<SlowCommand>::new());
    let result = signal(None::<SharedString>);

    // One writer for the store: a command, then the snapshot a view reads. `quiet` skips the
    // re-publish for draft writes, which come once a keystroke and must not rebuild the editor.
    let write = {
        let db = db.clone();
        Rc::new(move |command: Command, quiet: bool, name: &'static str| {
            let started = std::time::Instant::now();
            match db.apply(&command, SystemClock.now_ms()) {
                Ok(_) => {
                    let millis = u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX);
                    if millis >= SLOW_COMMAND_MS {
                        slow.update(|list| {
                            list.push(SlowCommand {
                                name: name.into(),
                                millis,
                            })
                        });
                    }
                    if !quiet && let Ok(next) = db.snapshot() {
                        snapshot_signal.set(next);
                    }
                }
                Err(error) => {
                    stopped.set(true);
                    log::error!(
                        "store refused a command: {}",
                        crate::logging::describe(&error)
                    );
                }
            }
        })
    };

    // The transfer entry points: a save dialog picks where the library goes, an open dialog picks
    // the archive; the screen states what came of either.
    let export = {
        let db = db.clone();
        move || {
            let db = db.clone();
            spawn(async move {
                match pick_save_path(SaveDialog::new().file_name("typr-library.sqlite3")).await {
                    Ok(Some(path)) => result.set(Some(export_to(&db, &path).into())),
                    Ok(None) => {}
                    Err(why) => result.set(Some(why.into())),
                }
            });
        }
    };
    let import = {
        let db = db.clone();
        let attachments = attachments.clone();
        move || {
            let db = db.clone();
            let attachments = attachments.clone();
            spawn(async move {
                match pick_paths(OpenDialog::files()).await {
                    Ok(Some(paths)) => {
                        let Some(archive) = paths.into_iter().next() else {
                            return;
                        };
                        result.set(Some(import_from(&db, &archive, &attachments).into()));
                    }
                    Ok(None) => {}
                    Err(why) => result.set(Some(why.into())),
                }
            });
        }
    };

    let open = { move |id: Id| route.set(Route::Editor(id)) };

    if let Some(key) = crate::screens::keymap::keystroke_of(
        typr_core::keymap::Surface::Library,
        typr_core::keymap::Command::FocusSearch,
    ) {
        let handle = search_handle;
        on_key(&key, move |_| handle.focus());
    }

    // The library's keys, every one taken from the keymap table.
    // A command may be bound to more than one chord (Down and `j`), and every one acts.
    for key in crate::screens::keymap::keystrokes_of(Surface::Library, Key::SelectionNext) {
        let visible = visible.clone();
        on_key(&key, move |_| {
            let last = visible().len().saturating_sub(1);
            selection.update(|at| *at = (*at + 1).min(last));
        });
    }
    for key in crate::screens::keymap::keystrokes_of(Surface::Library, Key::SelectionPrevious) {
        let visible = visible.clone();
        on_key(&key, move |_| {
            let last = visible().len().saturating_sub(1);
            selection.update(|at| *at = at.saturating_sub(1).min(last));
        });
    }
    for key in crate::screens::keymap::keystrokes_of(Surface::Library, Key::EditCard) {
        let open = open_selected.clone();
        on_key(&key, move |_| open());
    }
    for key in crate::screens::keymap::keystrokes_of(Surface::Library, Key::StartDueSession) {
        let start = start_session.clone();
        on_key(&key, move |_| start(typr_core::screen::Queue::Due));
    }
    for key in crate::screens::keymap::keystrokes_of(Surface::Library, Key::StartNewSession) {
        let start = start_session.clone();
        on_key(&key, move |_| start(typr_core::screen::Queue::New));
    }
    for key in crate::screens::keymap::keystrokes_of(Surface::Library, Key::NewCard) {
        let db = db.clone();
        let reopen = reopen.clone();
        on_key(&key, move |_| {
            let deck = state
                .selected_key()
                .as_deref()
                .and_then(|key| key.strip_prefix("deck:"))
                .and_then(|id| id.parse::<Id>().ok())
                .or_else(|| {
                    snapshot_signal
                        .get_untracked()
                        .decks
                        .first()
                        .map(|deck| deck.id)
                });
            let Some(deck) = deck else { return };
            match db.apply(
                &Command::CreateCard {
                    deck,
                    markdown: String::new(),
                },
                SystemClock.now_ms(),
            ) {
                Ok(Outcome::Card(id)) => {
                    reopen();
                    route.set(Route::Editor(id));
                }
                Ok(_) => {}
                Err(error) => log::error!("new card refused: {}", crate::logging::describe(&error)),
            }
        });
    }
    for key in crate::screens::keymap::keystrokes_of(Surface::Library, Key::DeleteCard) {
        let db = db.clone();
        let visible = visible.clone();
        let reopen = reopen.clone();
        on_key(&key, move |_| {
            let ids = visible();
            let Some(id) = ids.get(selection.get_untracked()).copied() else {
                return;
            };
            match db.apply(&Command::TrashCard { id }, SystemClock.now_ms()) {
                Ok(_) => {
                    selection.update(|at| *at = at.saturating_sub(1));
                    reopen();
                }
                Err(error) => log::error!("trash refused: {}", crate::logging::describe(&error)),
            }
        });
    }

    // Escape returns to the whole library from the grid; while a card is open the editor binds it,
    // and this one lets the key through so both can hear it.
    let route_for_key = route;
    on_key_with(
        "escape",
        KeyOptions {
            pass_through: true,
            when: When::Always,
            ..KeyOptions::default()
        },
        move |_| {
            if route_for_key.get() == Route::Library {
                state.select(&"all".into());
            }
        },
    );

    view()
        .flex_1()
        .flex_row()
        .bg(tokens::background)
        .child(sidebar(state))
        .child(view().flex_1().child(dynamic(move || {
            // A running session takes the whole work area until it is left.
            if studying.get() {
                let taken = session_slot.borrow_mut().take();
                let Some(session) = taken else {
                    return view().into_element();
                };
                let grade_db = db.clone();
                let grade_stopped = stopped;
                let leaving = studying;
                let reopen = reopen.clone();
                return study(
                    session,
                    move |card: Id, key: String, grade: Grade| match grade_db.apply(
                        &Command::Grade {
                            card,
                            key,
                            grade,
                            duration_ms: None,
                        },
                        SystemClock.now_ms(),
                    ) {
                        Ok(_) => true,
                        Err(error) => {
                            grade_stopped.set(true);
                            log::error!("grade refused: {}", crate::logging::describe(&error));
                            false
                        }
                    },
                    move || {
                        leaving.set(false);
                        reopen();
                    },
                    move || state.select(&"settings".into()),
                )
                .into_element();
            }
            if state.selected_key().as_deref() == Some("settings") {
                return settings(
                    Values::new(cap, true),
                    library.clone(),
                    |_| {},
                    || {},
                    || {},
                )
                .into_element();
            }
            if state.selected_key().as_deref() == Some("diagnostics") {
                return diagnostics(stopped, slow.get(), result, import.clone(), export.clone())
                    .into_element();
            }
            match route.get() {
                Route::Editor(id) => {
                    let found = snapshot_signal
                        .with(|snapshot| snapshot.cards.iter().find(|card| card.id == id).cloned());
                    let Some(card) = found else {
                        route.set(Route::Library);
                        return view().into_element();
                    };
                    let template = card.template.as_ref().map(|text| Template {
                        text: text.clone(),
                        fields: card
                            .fields
                            .iter()
                            .map(|(name, value)| Field {
                                name: name.clone(),
                                value: value.clone(),
                                computed: false,
                            })
                            .collect(),
                        deck: None,
                    });
                    let diagram =
                        (card.kind == typr_core::store::CardKind::Diagram).then(|| Diagram {
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
                            write_save(Command::SaveCard { id, markdown }, false, "SaveCard");
                            write_save(
                                Command::SetDraft {
                                    card: id,
                                    text: None,
                                },
                                true,
                                "SetDraft",
                            );
                        })
                        .draft(move |text| {
                            write_draft(
                                Command::SetDraft {
                                    card: id,
                                    text: Some(text.to_owned()),
                                },
                                true,
                                "SetDraft",
                            )
                        })
                        .leave(move || leaving.set(Route::Library))
                        .discard({
                            let write = write.clone();
                            move || {
                                // Discarding drops the stored draft, so the next open is the card.
                                write(
                                    Command::SetDraft {
                                        card: id,
                                        text: None,
                                    },
                                    true,
                                    "SetDraft",
                                );
                                leaving.set(Route::Library);
                            }
                        })
                        .attach({
                            let root = attachments.clone();
                            move |path| editor::attach_image(&path, &root)
                        })
                        .masks(
                            move |group, rect| {
                                write_mask(
                                    Command::AddMask {
                                        card: id,
                                        group,
                                        rect,
                                    },
                                    false,
                                    "AddMask",
                                )
                            },
                            move |group, rects| {
                                write_merge(
                                    Command::MergeMasks {
                                        card: id,
                                        group,
                                        rects,
                                    },
                                    false,
                                    "MergeMasks",
                                )
                            },
                            move |rect| {
                                write_remove(
                                    Command::RemoveMask { card: id, rect },
                                    false,
                                    "RemoveMask",
                                )
                            },
                        )
                        .kinds(
                            {
                                let write = write.clone();
                                move |group| {
                                    write(
                                        Command::RemoveMaskGroup { card: id, group },
                                        false,
                                        "RemoveMaskGroup",
                                    )
                                }
                            },
                            {
                                let write = write.clone();
                                move || {
                                    write(Command::ClearDiagram { card: id }, false, "ClearDiagram")
                                }
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
                                        "SetDiagram",
                                    )
                                }
                            },
                        );
                    editor::editor_card(
                        Card {
                            saved: card.markdown.clone(),
                            draft,
                            template,
                            diagram,
                        },
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
                        let text = query.get();
                        tiles_for(&cards, &text, now)
                    });
                    view()
                        .flex_1()
                        .p_4()
                        .gap_3()
                        .child(field_reachable("Search cards", query, &search_handle))
                        .child(grid(tiles, selection, || {}, open))
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
                    log::error!(
                        "library snapshot failed: {}",
                        crate::logging::describe(&error)
                    );
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
