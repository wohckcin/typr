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
use crate::screens::editor::{self, Cover, Diagram, Field, Template};
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

/// What the work area shows. One value owns it, so a key can only act where its screen is showing
/// and a screen can only be entered by setting it.
#[derive(Clone)]
enum Screen {
    /// The grid, filtered by the sidebar's row in force.
    Library,
    /// One card's editor.
    Editor(Id),
    /// The library's paths and defaults.
    Settings,
    /// What the store and the slow commands have done since launch.
    Diagnostics,
    /// A session, shared rather than taken, so a rebuild keeps the session it is showing.
    Study(Rc<std::cell::RefCell<Session>>),
}

impl PartialEq for Screen {
    /// Two screens are the same when they show the same thing; a session is the same session while
    /// its handle matches, so a rebuild of the work area is not a new screen.
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Screen::Library, Screen::Library)
            | (Screen::Settings, Screen::Settings)
            | (Screen::Diagnostics, Screen::Diagnostics) => true,
            (Screen::Editor(one), Screen::Editor(two)) => one == two,
            (Screen::Study(one), Screen::Study(two)) => Rc::ptr_eq(one, two),
            _ => false,
        }
    }
}

/// The screen a sidebar row opens: the utilities are screens, every other row filters the grid.
fn screen_for(key: &SharedString) -> Screen {
    match key.as_ref() {
        "settings" => Screen::Settings,
        "diagnostics" => Screen::Diagnostics,
        _ => Screen::Library,
    }
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
    let screen = signal(Screen::Library);
    let pick = {
        move |key: &SharedString| {
            state.select(key);
            screen.set(screen_for(key));
        }
    };
    let cap = snapshot.settings.new_per_day;
    // Without a store the screen still shows the library it read: a blank work area would leave a
    // reader guessing whether the app works.
    let snapshot = Rc::new(snapshot);
    let query = signal(String::new());
    let chosen = signal(None::<Id>);
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
    on_key("escape", move |_| {
        state.select(&"all".into());
        screen.set(Screen::Library);
    });
    view()
        .flex_1()
        .flex_row()
        .bg(tokens::background)
        .child(sidebar(state, pick))
        .child(view().flex_1().child(dynamic(move || {
            if let Screen::Settings = screen.get() {
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
                .child(
                    field()
                        .placeholder("Search cards")
                        .value(move || SharedString::from(query.get()))
                        .on_change(move |text: &String| query.set(text.to_string())),
                )
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
    let screen = signal(Screen::Library);
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
    // The library's keyboard: the card in force, by identity, so a filter or a reorder cannot move the
    // highlight to a different card at the same position.
    let selection = signal(None::<Id>);
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
        Rc::new(move || {
            let snapshot = snapshot_signal.get_untracked();
            let cards = cards_in(&snapshot, state.selected_key().as_deref());
            tiles_for(&cards, &query.get_untracked(), SystemClock.now_ms())
                .into_iter()
                .map(|tile| tile.id)
                .collect::<Vec<Id>>()
        })
    };
    let open_selected = {
        Rc::new(move || {
            if let Some(id) = selection.get_untracked() {
                screen.set(Screen::Editor(id));
            }
        })
    };
    // Moving the selection: one step from where the selected card sits in the list as it is shown,
    // the first card when nothing is selected, and the ends for Home and End.
    let step = {
        let visible = visible.clone();
        Rc::new(move |delta: isize| {
            let ids = visible();
            let Some(last) = ids.len().checked_sub(1) else {
                selection.set(None);
                return;
            };
            let at = selection
                .get_untracked()
                .and_then(|id| ids.iter().position(|each| *each == id));
            let next = match at {
                Some(at) => (at as isize + delta).clamp(0, last as isize) as usize,
                None if delta > 0 => 0,
                None => last,
            };
            selection.set(Some(ids[next]));
        })
    };
    let select_end = {
        let visible = visible.clone();
        Rc::new(move |first: bool| {
            let ids = visible();
            let chosen = if first { ids.first() } else { ids.last() };
            selection.set(chosen.copied());
        })
    };
    // The selected card leaves the list when a filter stops matching it: nothing stays highlighted.
    {
        let visible = visible.clone();
        effect(move || {
            let chosen = selection.get();
            let Some(id) = chosen else {
                return;
            };
            snapshot_signal.get();
            query.get();
            state.selected_key();
            if !visible().contains(&id) {
                selection.set(None);
            }
        });
    }
    // A session: the store serves the queue, the screen walks it, and each grade goes back through
    // the store quietly, so the session is not rebuilt under the user as it is graded.
    let start_session = {
        let db = db.clone();
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
            screen.set(Screen::Study(Rc::new(std::cell::RefCell::new(
                Session::new(queue, entries),
            ))));
        })
    };
    let slow = signal(Vec::<SlowCommand>::new());
    let result = signal(None::<SharedString>);
    let tiles = {
        computed(move || {
            let snapshot = snapshot_signal.get();
            let cards = cards_in(&snapshot, state.selected_key().as_deref());
            tiles_for(&cards, &query.get(), SystemClock.now_ms())
        })
    };

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

    let open = { move |id: Id| screen.set(Screen::Editor(id)) };
    let pick = {
        move |key: &SharedString| {
            state.select(key);
            screen.set(screen_for(key));
        }
    };

    // The library's keys act only on the library, so `c` inside the editor makes nothing and `s`
    // inside a session starts nothing.
    let in_library = move || matches!(screen.get(), Screen::Library);

    if let Some(key) = crate::screens::keymap::keystroke_of(
        typr_core::keymap::Surface::Library,
        typr_core::keymap::Command::FocusSearch,
    ) {
        let handle = search_handle;
        let here = in_library;
        on_key(&key, move |_| {
            if here() {
                handle.focus();
            }
        });
    }

    // The library's keys, every one taken from the keymap table.
    // A command may be bound to more than one chord (Down and `j`), and every one acts.
    for key in crate::screens::keymap::keystrokes_of(Surface::Library, Key::SelectionNext) {
        let here = in_library;
        let step = step.clone();
        on_key(&key, move |_| {
            if here() {
                step(1);
            }
        });
    }
    for key in crate::screens::keymap::keystrokes_of(Surface::Library, Key::SelectionPrevious) {
        let here = in_library;
        let step = step.clone();
        on_key(&key, move |_| {
            if here() {
                step(-1);
            }
        });
    }
    for key in crate::screens::keymap::keystrokes_of(Surface::Library, Key::SelectionFirst) {
        let here = in_library;
        let select_end = select_end.clone();
        on_key(&key, move |_| {
            if here() {
                select_end(true);
            }
        });
    }
    for key in crate::screens::keymap::keystrokes_of(Surface::Library, Key::SelectionLast) {
        let here = in_library;
        let select_end = select_end.clone();
        on_key(&key, move |_| {
            if here() {
                select_end(false);
            }
        });
    }
    for key in crate::screens::keymap::keystrokes_of(Surface::Library, Key::EditCard) {
        let here = in_library;
        let open = open_selected.clone();
        on_key(&key, move |_| {
            if here() {
                open();
            }
        });
    }
    for key in crate::screens::keymap::keystrokes_of(Surface::Library, Key::StartDueSession) {
        let here = in_library;
        let start = start_session.clone();
        on_key(&key, move |_| {
            if here() {
                start(typr_core::screen::Queue::Due);
            }
        });
    }
    for key in crate::screens::keymap::keystrokes_of(Surface::Library, Key::StartNewSession) {
        let here = in_library;
        let start = start_session.clone();
        on_key(&key, move |_| {
            if here() {
                start(typr_core::screen::Queue::New);
            }
        });
    }
    for key in crate::screens::keymap::keystrokes_of(Surface::Library, Key::NewCard) {
        let here = in_library;
        let db = db.clone();
        let reopen = reopen.clone();
        on_key(&key, move |_| {
            if !here() {
                return;
            }
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
                    screen.set(Screen::Editor(id));
                }
                Ok(_) => {}
                Err(error) => log::error!("new card refused: {}", crate::logging::describe(&error)),
            }
        });
    }
    for key in crate::screens::keymap::keystrokes_of(Surface::Library, Key::DeleteCard) {
        let here = in_library;
        let db = db.clone();
        let visible = visible.clone();
        let reopen = reopen.clone();
        on_key(&key, move |_| {
            if !here() {
                return;
            }
            let ids = visible();
            let Some(id) = selection.get_untracked() else {
                return;
            };
            let Some(at) = ids.iter().position(|each| *each == id) else {
                return;
            };
            match db.apply(&Command::TrashCard { id }, SystemClock.now_ms()) {
                Ok(_) => {
                    reopen();
                    // The card that took its place in the list now, or the new last card.
                    let after = visible();
                    selection.set(after.get(at).or_else(|| after.last()).copied());
                }
                Err(error) => log::error!("trash refused: {}", crate::logging::describe(&error)),
            }
        });
    }

    // A card can vanish under the open editor (trashed elsewhere, or gone from a refreshed
    // snapshot). The route follows the snapshot rather than the render that noticed.
    {
        effect(move || {
            if let Screen::Editor(id) = screen.get()
                && snapshot_signal.with(|snapshot| !snapshot.cards.iter().any(|card| card.id == id))
            {
                screen.set(Screen::Library);
            }
        });
    }

    // Escape returns to the whole library from the grid; while a card is open the editor binds it,
    // and this one lets the key through so both can hear it.
    on_key_with(
        "escape",
        KeyOptions {
            pass_through: true,
            when: When::Always,
            ..KeyOptions::default()
        },
        move |_| match screen.get() {
            // Escape narrows the grid here, and the editor and the session bind their own Leave.
            Screen::Library => state.select(&"all".into()),
            Screen::Settings | Screen::Diagnostics => {
                state.select(&"all".into());
                screen.set(Screen::Library);
            }
            Screen::Editor(_) | Screen::Study(_) => {}
        },
    );

    view()
        .flex_1()
        .flex_row()
        .bg(tokens::background)
        .child(sidebar(state, pick))
        .child(view().flex_1().child(dynamic(move || {
            // One match on the screen: a key can only act where its screen is showing.
            match screen.get() {
                // A running session takes the whole work area until it is left.
                Screen::Study(session) => {
                    let grade_db = db.clone();
                    let grade_stopped = stopped;
                    let reopen = reopen.clone();
                    study(
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
                            screen.set(Screen::Library);
                            reopen();
                        },
                        move || screen.set(Screen::Settings),
                    )
                    .into_element()
                }
                Screen::Settings => {
                    let back = {
                        let state = state;
                        move || {
                            state.select(&"all".into());
                            screen.set(Screen::Library);
                        }
                    };
                    settings(Values::new(cap, true), library.clone(), |_| {}, back, || {})
                        .into_element()
                }
                Screen::Diagnostics => {
                    diagnostics(stopped, slow.get(), result, import.clone(), export.clone())
                        .into_element()
                }
                Screen::Editor(id) => {
                    let found = snapshot_signal
                        .with(|snapshot| snapshot.cards.iter().find(|card| card.id == id).cloned());
                    let Some(card) = found else {
                        // The card went away under the editor. The effect above moves the screen
                        // back; until that runs, show the library rather than nothing.
                        return view()
                            .flex_1()
                            .p_4()
                            .gap_2()
                            .child(field_reachable("Search cards", query, &search_handle))
                            .child(grid(tiles, selection, || {}, open))
                            .into_element();
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
                            rows: card.masks.iter().map(Cover::of).collect(),
                        });
                    let draft = db.draft(id).ok().flatten();
                    let write_save = write.clone();
                    let write_draft = write.clone();
                    let leaving = screen;
                    editor::editor()
                        .saved(card.markdown.clone())
                        .draft(draft.map(SharedString::from))
                        .template(template)
                        .diagram(diagram)
                        .attachments(attachments.to_string_lossy().to_string())
                        .on_save(move |markdown: &String| {
                            write_save(
                                Command::SaveCard {
                                    id,
                                    markdown: markdown.clone(),
                                },
                                false,
                                "SaveCard",
                            );
                            write_save(
                                Command::SetDraft {
                                    card: id,
                                    text: None,
                                },
                                true,
                                "SetDraft",
                            );
                        })
                        .on_draft(move |text: &String| {
                            write_draft(
                                Command::SetDraft {
                                    card: id,
                                    text: Some(text.clone()),
                                },
                                true,
                                "SetDraft",
                            )
                        })
                        .on_leave(move || leaving.set(Screen::Library))
                        .on_discard({
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
                                screen.set(Screen::Library);
                            }
                        })
                        .on_add_mask({
                            let write = write.clone();
                            move |mask: &Cover| {
                                write(
                                    Command::AddMask {
                                        card: id,
                                        group: mask.group,
                                        rect: typr_core::store::MaskRect::new(
                                            mask.x, mask.y, mask.w, mask.h,
                                        ),
                                    },
                                    false,
                                    "AddMask",
                                )
                            }
                        })
                        .on_merge_masks({
                            let write = write.clone();
                            move |merged: &editor::MergedMasks| {
                                write(
                                    Command::MergeMasks {
                                        card: id,
                                        group: merged.group,
                                        rects: merged
                                            .rects
                                            .iter()
                                            .map(|rect| {
                                                typr_core::store::MaskRect::new(
                                                    rect.x, rect.y, rect.w, rect.h,
                                                )
                                            })
                                            .collect(),
                                    },
                                    false,
                                    "MergeMasks",
                                )
                            }
                        })
                        .on_remove_mask_group({
                            let write = write.clone();
                            move |group: &u32| {
                                write(
                                    Command::RemoveMaskGroup {
                                        card: id,
                                        group: *group,
                                    },
                                    false,
                                    "RemoveMaskGroup",
                                )
                            }
                        })
                        .on_clear_diagram({
                            let write = write.clone();
                            move || write(Command::ClearDiagram { card: id }, false, "ClearDiagram")
                        })
                        .on_diagram_from_image({
                            let write = write.clone();
                            move |image: &String| {
                                write(
                                    Command::SetDiagram {
                                        card: id,
                                        image: image.clone(),
                                        caption: String::new(),
                                    },
                                    false,
                                    "SetDiagram",
                                )
                            }
                        })
                        .into_element()
                }
                // The grid: the sidebar's row in force picks the cards, and a tile opens one.
                Screen::Library => view()
                    .flex_1()
                    .p_4()
                    .gap_3()
                    .child(field_reachable("Search cards", query, &search_handle))
                    .child(grid(tiles, selection, || {}, open))
                    .into_element(),
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
