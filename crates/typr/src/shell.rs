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
use typr_core::edits::{deck_for_new_card, selection_after_trash};
use typr_core::keymap::{Command as Key, Surface, bindings_of};
use typr_core::session::{Entry, Session};
use typr_core::store::{
    Command, Db, Grade, Id, Outcome, SettingKey, SettingValue, Snapshot, StoreError, Theme,
};

use crate::screens::diagnostics::{SlowCommand, diagnostics};
use crate::screens::editor::{self, Cover, Diagram, Template, TemplateField};
use crate::screens::grid::grid;
use crate::screens::library::{cards_in, tiles_for};
use crate::screens::settings::{LibraryInfo, Values, settings};
use crate::screens::sidebar::{
    NEW_DECK_PREFIX, SEARCH_KEY, Sidebar, due_counts, items_from, search_chord, sidebar,
};
use crate::screens::study::{entries_for, study};
use crate::ui::{
    input::field_reachable,
    overlay::{self, Toasts},
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

/// How many study days before `now` a prompt fell due, which is what a session's `Entry` orders and
/// reshuffles by.
pub fn overdue_days(now_ms: i64, due_at: i64, _offset_ms: i64) -> i64 {
    (now_ms - due_at).max(0)
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
    /// The cards that were deleted and can still be restored.
    Trash,
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
            | (Screen::Diagnostics, Screen::Diagnostics)
            | (Screen::Trash, Screen::Trash) => true,
            (Screen::Editor(one), Screen::Editor(two)) => one == two,
            (Screen::Study(one), Screen::Study(two)) => Rc::ptr_eq(one, two),
            _ => false,
        }
    }
}

/// The screen a sidebar row opens: Settings and Trash are screens, every other row filters the grid.
fn screen_for(key: &SharedString) -> Screen {
    match key.as_ref() {
        "settings" => Screen::Settings,
        "trash" => Screen::Trash,
        _ => Screen::Library,
    }
}

/// The shell over `snapshot`: `library_open` says whether the store opened, which the sidebar
/// states. Tests drive this with a fixture; the app drives the same function with its store.
pub fn shell(snapshot: Snapshot, library_open: bool) -> impl IntoElement {
    shell_over(None, snapshot, library_open, LibraryInfo::default(), None)
}

/// The one shell: the library when it opens and when it does not. `db` is the store the screens
/// write through, absent when the library could not be opened — `notice` then says why, and the
/// sidebar states it instead of a bare "Library closed".
pub fn shell_over(
    db: Option<Rc<Db>>,
    snapshot: Snapshot,
    library_open: bool,
    library: LibraryInfo,
    notice: Option<SharedString>,
) -> impl IntoElement {
    let snapshot_signal = signal(snapshot);
    let screen = signal(Screen::Library);
    let now = SystemClock.now_ms();
    let due = due_counts(&snapshot_signal.get_untracked(), now);
    let state = Sidebar::new(
        items_from(&snapshot_signal.get_untracked(), &due, now),
        library_open,
        search_chord(),
    );
    // The rows state counts, so a write that changes the snapshot (a deck made, a card restored)
    // changes them too.
    effect(move || {
        let snapshot = snapshot_signal.get();
        let now = SystemClock.now_ms();
        state.refresh(items_from(&snapshot, &due_counts(&snapshot, now), now));
    });
    if let Some(notice) = notice {
        state.note(notice);
    }
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
            if let Some(db) = db.as_ref()
                && let Ok(next) = db.snapshot()
            {
                snapshot_signal.set(next);
            }
        })
    };
    // The cards the grid shows now, in the order it shows them: the keyboard walks this list.
    let visible = {
        Rc::new(move || {
            let snapshot = snapshot_signal.get_untracked();
            let cards = cards_in(
                &snapshot,
                state.selected_key().as_deref(),
                SystemClock.now_ms(),
            );
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
            let Some(db) = db.as_ref() else {
                return;
            };
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
                    .map(|p| (p.card, p.key, overdue_days(now, p.due_at, offset)))
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
    let results = Toasts::new();
    let tiles = {
        computed(move || {
            let snapshot = snapshot_signal.get();
            let cards = cards_in(
                &snapshot,
                state.selected_key().as_deref(),
                SystemClock.now_ms(),
            );
            tiles_for(&cards, &query.get(), SystemClock.now_ms())
        })
    };

    // One writer for the store: it times a command, records a slow one, flags a stoppage when the
    // store refuses it, and re-reads the snapshot a view reads when `republish` is set. Draft writes
    // skip the re-publish because they come once a keystroke and must not rebuild the editor.
    // `None` means no library is open to write to.
    let write = {
        let db = db.clone();
        Rc::new(
            move |command: Command, republish: bool| -> Option<Result<Outcome, StoreError>> {
                let db = db.as_ref()?;
                let started = std::time::Instant::now();
                let result = db.apply(&command, SystemClock.now_ms());
                match &result {
                    Ok(_) => {
                        let millis =
                            u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX);
                        if millis >= SLOW_COMMAND_MS {
                            slow.update(|list| {
                                list.push(SlowCommand {
                                    name: command.name().into(),
                                    millis,
                                })
                            });
                        }
                        if republish && let Ok(next) = db.snapshot() {
                            snapshot_signal.set(next);
                        }
                    }
                    Err(error) => {
                        stopped.set(true);
                        log::error!(
                            "store refused a command: {}",
                            crate::logging::describe(error)
                        );
                    }
                }
                Some(result)
            },
        )
    };
    let apply = {
        let write = write.clone();
        Rc::new(move |command: Command| {
            write(command, true);
        })
    };
    let quiet_apply = {
        let write = write.clone();
        Rc::new(move |command: Command| {
            write(command, false);
        })
    };

    // The appearance the window follows is the library's `theme`, not `nsh.json`'s. One effect
    // stores a change and then applies it, so the setting is in the library before the frame that
    // draws it; at mount it only applies, which puts the stored theme on the first frame.
    let theme = signal(snapshot_signal.with(|s| s.settings.theme));
    {
        let write = write.clone();
        let stored = std::cell::Cell::new(theme.get_untracked());
        effect(move || {
            let chosen = theme.get();
            if chosen != stored.get() {
                stored.set(chosen);
                write(
                    Command::SetSetting {
                        key: SettingKey::Theme,
                        value: SettingValue::Theme(chosen),
                    },
                    true,
                );
            }
            set_color_scheme(match chosen {
                Theme::System => Appearance::System,
                Theme::Light => Appearance::Light,
                Theme::Dark => Appearance::Dark,
            });
        });
    }

    // The transfer entry points: a save dialog picks where the library goes, an open dialog picks
    // the archive; the screen states what came of either.
    let export = {
        let db = db.clone();
        move || {
            let Some(db) = db.clone() else {
                return;
            };
            spawn(async move {
                match pick_save_path(SaveDialog::new().file_name("typr-library.sqlite3")).await {
                    Ok(Some(path)) => results.push(export_to(&db, &path).into()),
                    Ok(None) => {}
                    Err(why) => results.push(why.into()),
                }
            });
        }
    };
    let import = {
        let db = db.clone();
        let attachments = attachments.clone();
        move || {
            let Some(db) = db.clone() else {
                return;
            };
            let attachments = attachments.clone();
            spawn(async move {
                match pick_paths(OpenDialog::files()).await {
                    Ok(Some(paths)) => {
                        let Some(archive) = paths.into_iter().next() else {
                            return;
                        };
                        results.push(import_from(&db, &archive, &attachments).into());
                    }
                    Ok(None) => {}
                    Err(why) => results.push(why.into()),
                }
            });
        }
    };

    let open = { move |id: Id| screen.set(Screen::Editor(id)) };

    // The library's keys act only on the library, so `c` inside the editor makes nothing and `s`
    // inside a session starts nothing.
    let in_library = move || matches!(screen.get(), Screen::Library);

    // The command list: one element for the palette and the shortcut panel. Each opens on its
    // chord and closes on the same chord or Escape.
    let palette_open = signal(false);
    let panel_open = signal(false);
    let close = Rc::new(move || {
        palette_open.set(false);
        panel_open.set(false);
    });

    let new_card = {
        let write = write.clone();
        let state_for_card = state;
        Rc::new(move || {
            let Some(deck) = deck_for_new_card(
                &snapshot_signal.get_untracked(),
                state_for_card.selected_key().as_deref(),
            ) else {
                return;
            };
            let made = write(
                Command::CreateCard {
                    deck,
                    markdown: String::new(),
                },
                true,
            );
            if let Some(Ok(Outcome::Card(id))) = made {
                screen.set(Screen::Editor(id));
            }
        })
    };

    let delete_card = {
        let write = write.clone();
        let visible = visible.clone();
        Rc::new(move || {
            let Some(id) = selection.get_untracked() else {
                return;
            };
            let before = visible();
            if !before.contains(&id) {
                return;
            }
            if let Some(Ok(_)) = write(Command::TrashCard { id }, true) {
                selection.set(selection_after_trash(&before, id, &visible()));
            }
        })
    };

    let focus_search = {
        let handle = search_handle;
        Rc::new(move || handle.focus())
    };

    // One dispatcher: a chord and a palette item run the same action, so the two cannot drift.
    // A command the app does not carry out yet is never reached from the palette, which lists it
    // dimmed (see `Command::runnable`).
    let run: Rc<dyn Fn(Key)> = {
        let start_session = start_session.clone();
        let open_selected = open_selected.clone();
        let step = step.clone();
        let select_end = select_end.clone();
        let new_card = new_card.clone();
        let delete_card = delete_card.clone();
        let focus_search = focus_search.clone();
        Rc::new(move |command: Key| match command {
            Key::StartDueSession => start_session(typr_core::screen::Queue::Due),
            Key::StartNewSession => start_session(typr_core::screen::Queue::New),
            Key::NewCard => new_card(),
            Key::DeleteCard => delete_card(),
            Key::EditCard => open_selected(),
            Key::FocusSearch => focus_search(),
            Key::SelectionNext => step(1),
            Key::SelectionPrevious => step(-1),
            Key::SelectionFirst => select_end(true),
            Key::SelectionLast => select_end(false),
            Key::Settings => {
                state.select(&"settings".into());
                screen.set(Screen::Settings);
            }
            Key::ShowPalette => palette_open.update(|open| *open = !*open),
            Key::ShowShortcuts => panel_open.update(|open| *open = !*open),
            _ => {}
        })
    };

    for binding in bindings_of(Surface::Library) {
        let Some(stroke) = crate::screens::keymap::keystroke_of_chord(binding.chord) else {
            continue;
        };
        let here = in_library;
        let run = run.clone();
        let command = binding.command;
        on_key(&stroke, move |_| {
            if here() {
                run(command);
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

    // What a press on the sidebar does. Import and a named deck are actions, not rows to select; the
    // search row shows the library and puts the caret in its field; every other row is a selection
    // that names a screen.
    let pick = {
        let write = write.clone();
        let import = import.clone();
        let focus_search = focus_search.clone();
        move |key: &SharedString| {
            if key.as_ref() == "import" {
                import();
            } else if key.as_ref() == SEARCH_KEY {
                let shown = matches!(screen.get_untracked(), Screen::Library);
                screen.set(Screen::Library);
                if shown {
                    focus_search();
                } else {
                    // The field is drawn again by the screen change; focus it once it exists.
                    let focus_search = focus_search.clone();
                    spawn(async move { focus_search() });
                }
            } else if let Some(name) = key.strip_prefix(NEW_DECK_PREFIX) {
                let made = write(
                    Command::CreateDeck {
                        parent: None,
                        name: name.to_owned(),
                    },
                    true,
                );
                if let Some(Ok(Outcome::Deck(id))) = made {
                    state.select(&format!("deck:{id}").into());
                    screen.set(Screen::Library);
                }
            } else {
                state.select(key);
                screen.set(screen_for(key));
            }
        }
    };

    // The empty deck's button runs what the new-card chord runs: the one closure, so a press and
    // `c` cannot come to different cards.
    let on_new = new_card.clone();
    // The library screen, drawn by its own branch and by the editor's missing-card fallback.
    let library_view = move || {
        let on_new = on_new.clone();
        view()
            .flex_1()
            .p_4()
            .gap_3()
            .child(field_reachable("Search cards", query, &search_handle))
            .child(grid(tiles, selection, move || on_new(), open))
            .into_element()
    };

    // The system title bar is hidden (nsh.json): the shell starts below the window buttons, and a
    // strip over that space moves the window.
    let bar = move || nsh_core::element::desktop::title_bar().top;
    view()
        .flex_1()
        .flex_row()
        .pt(bar)
        .bg(tokens::background)
        .child(
            view()
                .absolute()
                .top_0()
                .left_0()
                .right_0()
                .h(bar)
                .window_drag(true),
        )
        .child(sidebar(state, pick))
        .child(view().flex_1().child(dynamic(move || {
            // One match on the screen: a key can only act where its screen is showing.
            match screen.get() {
                // A running session takes the whole work area until it is left.
                Screen::Study(session) => {
                    let grade_write = write.clone();
                    let reopen = reopen.clone();
                    study(
                        session,
                        move |card: Id, key: String, grade: Grade| {
                            let command = Command::Grade {
                                card,
                                key,
                                grade,
                                duration_ms: None,
                            };
                            match grade_write(command, false) {
                                Some(Ok(_)) => Ok(()),
                                _ => Err(crate::screens::study::GradeRefused),
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
                    let cap = snapshot_signal.get_untracked().settings.new_per_day;
                    let mut values = Values::new(cap, true);
                    values.theme = theme;
                    let write_cap = write.clone();
                    settings(
                        values,
                        library.clone(),
                        move |cap| {
                            write_cap(
                                Command::SetSetting {
                                    key: SettingKey::NewPerDay,
                                    value: SettingValue::Number(cap),
                                },
                                true,
                            );
                        },
                        import.clone(),
                        export.clone(),
                        move || screen.set(Screen::Diagnostics),
                    )
                    .into_element()
                }
                Screen::Trash => {
                    let restore = apply.clone();
                    let empty = apply.clone();
                    crate::screens::trash::trash(
                        snapshot_signal,
                        move |id: Id| restore(Command::RestoreCard { id }),
                        move || empty(Command::EmptyTrash),
                    )
                    .into_element()
                }
                Screen::Diagnostics => {
                    diagnostics(stopped, slow.get(), results, import.clone(), export.clone())
                        .into_element()
                }
                Screen::Editor(id) => {
                    let found = snapshot_signal
                        .with(|snapshot| snapshot.cards.iter().find(|card| card.id == id).cloned());
                    let Some(card) = found else {
                        // The card went away under the editor. The effect above moves the screen
                        // back; until that runs, show the library rather than nothing.
                        return library_view();
                    };
                    let template = card.template.as_ref().map(|text| Template {
                        text: text.clone(),
                        fields: card
                            .fields
                            .iter()
                            .map(|(name, value)| TemplateField {
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
                    let draft = db.as_ref().and_then(|db| db.draft(id).ok().flatten());
                    let write_save = apply.clone();
                    let write_draft = quiet_apply.clone();
                    let leaving = screen;
                    editor::editor()
                        .saved(card.markdown.clone())
                        .draft(draft.map(SharedString::from))
                        .template(template)
                        .diagram(diagram)
                        .attachments(attachments.to_string_lossy().to_string())
                        .on_save(move |saved: &editor::CardText| {
                            write_save(Command::SaveCard {
                                id,
                                markdown: saved.text.clone(),
                            });
                        })
                        .on_draft(move |draft: &editor::CardText| {
                            write_draft(Command::SetDraft {
                                card: id,
                                text: Some(draft.text.clone()),
                            })
                        })
                        .on_leave(move || leaving.set(Screen::Library))
                        .on_discard({
                            let write = quiet_apply.clone();
                            move || {
                                // Discarding drops the stored draft, so the next open is the card.
                                write(Command::SetDraft {
                                    card: id,
                                    text: None,
                                });
                                screen.set(Screen::Library);
                            }
                        })
                        .on_add_mask({
                            let write = apply.clone();
                            move |added: &editor::MaskAdded| {
                                let mask = added.mask;
                                write(Command::AddMask {
                                    card: id,
                                    group: mask.group,
                                    rect: typr_core::store::MaskRect::new(
                                        mask.x, mask.y, mask.w, mask.h,
                                    ),
                                })
                            }
                        })
                        .on_merge_masks({
                            let write = apply.clone();
                            move |merged: &editor::GroupMerge| {
                                write(Command::MergeMasks {
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
                                })
                            }
                        })
                        .on_remove_mask_group({
                            let write = apply.clone();
                            move |masked: &editor::MaskGroup| {
                                write(Command::RemoveMaskGroup {
                                    card: id,
                                    group: masked.group,
                                })
                            }
                        })
                        .on_clear_diagram({
                            let write = apply.clone();
                            move || write(Command::ClearDiagram { card: id })
                        })
                        .on_start_diagram({
                            let write = apply.clone();
                            move |started: &editor::DiagramStart| {
                                write(Command::SetDiagram {
                                    card: id,
                                    image: started.name.clone(),
                                    caption: String::new(),
                                })
                            }
                        })
                        .into_element()
                }
                // The grid: the sidebar's row in force picks the cards, and a tile opens one.
                Screen::Library => library_view(),
            }
        })))
        // The command list, drawn over whatever screen is open. Both open on their chord, run a
        // command or list the bindings, and close on the chord again or on Escape.
        .child(overlay::decision(palette_open, {
            let run = run.clone();
            let close = close.clone();
            move || {
                let run = run.clone();
                let dismiss = close.clone();
                let cancel = close.clone();
                crate::screens::keymap::panel(
                    Surface::Library,
                    crate::ui::command::Mode::Palette,
                    move |id: &SharedString| {
                        if let Some(command) = crate::screens::keymap::command_of(id) {
                            run(command);
                        }
                        dismiss();
                    },
                    move || cancel(),
                )
            }
        }))
        .child(overlay::decision(panel_open, {
            let close = close.clone();
            move || {
                let dismiss = close.clone();
                let cancel = close.clone();
                crate::screens::keymap::panel(
                    Surface::Library,
                    crate::ui::command::Mode::Panel,
                    move |_: &SharedString| dismiss(),
                    move || cancel(),
                )
            }
        }))
}

/// The app's root: the real library when it opens, and an empty, closed one when it does not, so
/// the sidebar says so rather than hiding it.
#[component]
pub fn App() -> impl IntoElement {
    crate::ui::tokens::install();
    // The data folder is the engine's, so it is read here, inside the mount, and the log lands in
    // it before anything else can fail.
    if let Ok(root) = crate::paths::data_root() {
        match crate::logging::install(
            &crate::paths::logs_root(&root),
            crate::logging::level_from_env(),
        ) {
            Ok(path) => log::info!("typr starts; log at {}", path.display()),
            Err(error) => log::error!("the log could not be installed: {error}"),
        }
    }
    match crate::paths::open_from_environment() {
        Ok(db) => {
            let library = LibraryInfo::of(&db);
            let db = Rc::new(db);
            match db.snapshot() {
                Ok(snapshot) => shell_over(Some(db), snapshot, true, library, None).into_element(),
                Err(error) => {
                    let why = crate::logging::describe(&error);
                    log::error!("library snapshot failed: {why}");
                    shell_over(None, Snapshot::default(), false, library, Some(why.into()))
                        .into_element()
                }
            }
        }
        Err(error) => {
            let why = crate::logging::describe(&error);
            log::error!("library did not open: {why}");
            shell_over(
                None,
                Snapshot::default(),
                false,
                LibraryInfo::default(),
                Some(why.into()),
            )
            .into_element()
        }
    }
}
