//! The NSH app's root and its shell: the sidebar beside the work area. Pages and screens join the
//! shell as they are ported.
//!
//! The sidebar's Settings row opens the settings screen in the work area; Escape returns to the
//! library. Anything else keeps the work area empty until its screen is wired.

mod editor;
mod navigation;
mod screen;
mod store_bridge;
mod study;

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

use nsh_core::prelude::*;
use typr_core::clock::{Clock, SystemClock};
use typr_core::keymap::{Command as Key, Surface, bindings_of};
use typr_core::store::{
    Command, Handle, Id, Outcome, SettingKey, SettingValue, Snapshot, Store, Theme,
};

use crate::screens::diagnostics::{SlowCommand, diagnostics};
use crate::screens::grid::grid;
use crate::screens::library::{cards_in, empty_message, offers_new_card, tiles_for};
use crate::screens::settings::{LibraryInfo, Values, settings};
use crate::screens::sidebar::{
    NEW_DECK_PREFIX, Sidebar, due_counts, items_from, search_chord, sidebar,
};
use crate::ui::{
    input::field_reachable,
    overlay::{self, Toasts},
    tokens,
};

pub use store_bridge::{WakeSignal, export_to, import_from};
pub use study::{overdue_days, wake_session};

use self::editor::EditorController;
use self::navigation::LibraryNav;
use self::screen::{Screen, screen_for};
use self::store_bridge::StoreBridge;
use self::study::StudyController;

/// The shell over `snapshot`: `library_open` says whether the store opened, which the sidebar
/// states. Tests drive this with a fixture; the app drives the same function with its store worker.
pub fn shell(snapshot: Snapshot, library_open: bool) -> impl IntoElement {
    shell_over(None, snapshot, library_open, LibraryInfo::default(), None)
}

/// The one shell: the library when it opens and when it does not. `store` is the background worker
/// handle the screens write through, absent when the library could not be opened — `notice` then
/// says why, and the sidebar states it instead of a bare "Library closed".
pub fn shell_over(
    store: Option<(Rc<Handle>, Arc<WakeSignal>)>,
    snapshot: Snapshot,
    library_open: bool,
    library: LibraryInfo,
    notice: Option<SharedString>,
) -> impl IntoElement {
    let snapshot_signal = signal(snapshot);
    let screen = signal(Screen::Library);
    let stopped = signal(!library_open);
    let slow = signal(Vec::<SlowCommand>::new());
    let results = Toasts::new();

    // Keep ownership of the worker handle and wake guard inside the reactive owner so unmounting
    // the shell closes the wake loop and joins the store worker thread.
    let (bridge, owner) = StoreBridge::new(store, snapshot_signal, stopped, slow);
    let _owner = stored(owner);

    let now = SystemClock.now_ms();
    let due = due_counts(&snapshot_signal.get_untracked(), now);
    let state = Sidebar::new(
        items_from(&snapshot_signal.get_untracked(), &due, now),
        library_open,
    );
    effect(move || {
        let snapshot = snapshot_signal.get();
        let now = SystemClock.now_ms();
        state.refresh(items_from(&snapshot, &due_counts(&snapshot, now), now));
    });
    if let Some(notice) = notice {
        state.note(notice);
    }

    let query = signal(String::new());
    let search_handle = element_ref();
    let selection = signal(None::<Id>);

    let nav = LibraryNav::new(
        snapshot_signal,
        state,
        query,
        selection,
        screen,
        search_handle,
        bridge.clone(),
    );
    let editor_ctl = EditorController::new(
        bridge.clone(),
        snapshot_signal,
        screen,
        library.attachments.clone(),
    );
    let study_ctl = StudyController::new(bridge.clone(), snapshot_signal, screen);

    let tiles = computed(move || {
        let snapshot = snapshot_signal.get();
        let now = SystemClock.now_ms();
        let cards = cards_in(&snapshot, state.selected_key().as_deref(), now);
        tiles_for(&cards, &query.get(), now)
    });

    // The appearance the window follows is the library's `theme`, not `nsh.json`'s. One effect
    // stores a change and then applies it, so the setting is in the library before the frame that
    // draws it; at mount it only applies, which puts the stored theme on the first frame.
    let theme = signal(snapshot_signal.with(|s| s.settings.theme));
    bind_theme(theme, bridge.clone());

    let export = {
        let bridge = bridge.clone();
        move || bridge.spawn_export(results)
    };
    let import = {
        let bridge = bridge.clone();
        let attachments = library.attachments.clone();
        move || bridge.spawn_import(attachments.clone(), results)
    };

    // The command list: one element for the palette and the shortcut panel. Each opens on its
    // chord and closes on the same chord or Escape.
    let palette_open = signal(false);
    let panel_open = signal(false);
    let close = Rc::new(move || {
        palette_open.set(false);
        panel_open.set(false);
    });

    let run = command_runner(
        nav.clone(),
        editor_ctl.clone(),
        study_ctl.clone(),
        state,
        selection,
        screen,
        palette_open,
        panel_open,
    );
    bind_library_keys(screen, run.clone());

    let pick = sidebar_picker(bridge.clone(), state, screen);

    // The field names its chord: no sidebar row carries it any more.
    let search_placeholder = match search_chord() {
        chord if chord.is_empty() => "Search cards".to_owned(),
        chord => format!("Search cards  {chord}"),
    };
    // Memos, so a snapshot that leaves a count alone does not rebuild its button.
    let queued_in = move |key: &'static str| {
        computed(move || snapshot_signal.with(|s| cards_in(s, Some(key), SystemClock.now_ms()).len()))
    };
    let (due_n, new_n) = (queued_in("inbox:due"), queued_in("inbox:new"));
    let library_view = {
        let editor_new = editor_ctl.clone();
        let editor_open = editor_ctl.clone();
        let study_ctl = study_ctl.clone();
        move || {
            let editor_new = editor_new.clone();
            let editor_open = editor_open.clone();
            let review = |label: &'static str, count: Memo<usize>, queue: typr_core::screen::Queue| {
                let study = study_ctl.clone();
                dynamic(move || {
                    let study = study.clone();
                    let n = count.get();
                    crate::ui::button::button()
                        .label(format!("{label} {n}"))
                        .disabled(n == 0)
                        .on_press(move || study.start(queue))
                        .into_element()
                })
            };
            let key = move || state.selected_key();
            view()
                .flex_1()
                .p_4()
                .gap_3()
                .child(
                    view()
                        .flex_row()
                        .gap_2()
                        .child(review("Review due", due_n, typr_core::screen::Queue::Due))
                        .child(review("Review new", new_n, typr_core::screen::Queue::New)),
                )
                .child(field_reachable(search_placeholder.clone(), query, &search_handle))
                .child(grid(
                    tiles,
                    selection,
                    move || query.with(|q| empty_message(key().as_deref(), q)),
                    move || query.with(|q| offers_new_card(key().as_deref(), q)),
                    move || editor_new.new_card(state),
                    move |id: Id| editor_open.open(id),
                ))
                .into_element()
        }
    };

    // The system title bar is hidden (`App`'s window): the shell starts below the window buttons, and a
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
            match screen.get() {
                Screen::Study(session) => study_ctl.render(session).into_element(),
                Screen::Settings => {
                    let cap = snapshot_signal.get_untracked().settings.new_per_day;
                    let mut values = Values::new(cap);
                    values.theme = theme;
                    let bridge = bridge.clone();
                    settings(
                        values,
                        library.clone(),
                        move |cap| {
                            bridge.apply(Command::SetSetting {
                                key: SettingKey::NewPerDay,
                                value: SettingValue::Number(cap),
                            });
                        },
                        import.clone(),
                        export.clone(),
                        move || screen.set(Screen::Diagnostics),
                    )
                    .into_element()
                }
                Screen::Trash => {
                    let restore = bridge.clone();
                    let empty = bridge.clone();
                    crate::screens::trash::trash(
                        snapshot_signal,
                        move |id: Id| restore.apply(Command::RestoreCard { id }),
                        move || empty.apply(Command::EmptyTrash),
                    )
                    .into_element()
                }
                Screen::Diagnostics => {
                    diagnostics(stopped, slow.get(), results, import.clone(), export.clone())
                        .into_element()
                }
                Screen::Editor(id) => editor_ctl.render(id, library_view.clone()),
                // Untracked: a read while building the library must not rebuild it, which would
                // drop its buttons before they first render and panic on a disposed prop.
                Screen::Library => untracked(&library_view),
            }
        })))
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

fn bind_theme(theme: Signal<Theme>, bridge: StoreBridge) {
    let stored_theme = Cell::new(theme.get_untracked());
    effect(move || {
        let chosen = theme.get();
        if chosen != stored_theme.get() {
            stored_theme.set(chosen);
            bridge.apply(Command::SetSetting {
                key: SettingKey::Theme,
                value: SettingValue::Theme(chosen),
            });
        }
        set_color_scheme(match chosen {
            Theme::System => Appearance::System,
            Theme::Light => Appearance::Light,
            Theme::Dark => Appearance::Dark,
        });
    });
}

#[allow(clippy::too_many_arguments)]
fn command_runner(
    nav: LibraryNav,
    editor: EditorController,
    study: StudyController,
    state: Sidebar,
    selection: Signal<Option<Id>>,
    screen: Signal<Screen>,
    palette_open: Signal<bool>,
    panel_open: Signal<bool>,
) -> Rc<dyn Fn(Key)> {
    Rc::new(move |command: Key| match command {
        Key::StartDueSession => study.start(typr_core::screen::Queue::Due),
        Key::StartNewSession => study.start(typr_core::screen::Queue::New),
        Key::NewCard => editor.new_card(state),
        Key::DeleteCard => nav.delete_card(),
        Key::EditCard => {
            if let Some(id) = selection.get_untracked() {
                editor.open(id);
            }
        }
        Key::FocusSearch => nav.focus_search(),
        Key::SelectionNext => nav.step(1),
        Key::SelectionPrevious => nav.step(-1),
        Key::SelectionFirst => nav.select_end(true),
        Key::SelectionLast => nav.select_end(false),
        Key::Settings => {
            state.select(&"settings".into());
            screen.set(Screen::Settings);
        }
        Key::ShowPalette => palette_open.update(|open| *open = !*open),
        Key::ShowShortcuts => panel_open.update(|open| *open = !*open),
        _ => {}
    })
}

fn bind_library_keys(screen: Signal<Screen>, run: Rc<dyn Fn(Key)>) {
    for binding in bindings_of(Surface::Library) {
        let Some(stroke) = crate::screens::keymap::keystroke_of_chord(binding.chord) else {
            continue;
        };
        let run = run.clone();
        let command = binding.command;
        on_key(&stroke, move |_| {
            if matches!(screen.get(), Screen::Library) {
                run(command);
            }
        });
    }
}

fn sidebar_picker(
    bridge: StoreBridge,
    state: Sidebar,
    screen: Signal<Screen>,
) -> impl Fn(&SharedString) + Clone + 'static {
    move |key: &SharedString| {
        if let Some(name) = key.strip_prefix(NEW_DECK_PREFIX) {
            bridge.submit(
                Command::CreateDeck {
                    parent: None,
                    name: name.to_owned(),
                },
                true,
                Some(Box::new(move |result| {
                    if let Ok(Outcome::Deck(id)) = result {
                        state.select(&format!("deck:{id}").into());
                        screen.set(Screen::Library);
                    }
                })),
            );
        } else {
            state.select(key);
            screen.set(screen_for(key));
        }
    }
}

/// The app's root: its one window. Since nsh 0.7 the host window never shows, so the app declares
/// the window it lives in; the hidden title bar it had in `nsh.json` is set here.
#[component]
pub fn App() -> impl IntoElement {
    crate::ui::tokens::install();
    // Resolve the local study-day offset on the main thread before spawning the store worker.
    let _ = crate::day_offset::local_offset_ms();
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
    // Fields are set one by one: both structs are `#[non_exhaustive]`.
    let mut bar = nsh_core::desktop::TitleBarOptions::default();
    bar.style = nsh_core::desktop::TitleBarStyle::Hidden;
    bar.height = Some(tokens::title_bar_height());
    let mut attributes = nsh_core::desktop::WindowAttributes::default();
    attributes.title_bar = Some(bar);
    nsh_core::element::desktop::window()
        .id("main")
        .title("typr")
        .default_size(tokens::window_size())
        .restore_bounds(true)
        .attributes(attributes)
        .content(library_shell)
}

/// The real library when it opens, and an empty, closed one when it does not, so the sidebar says
/// so rather than hiding it.
fn library_shell() -> AnyElement {
    match crate::paths::open_from_environment() {
        Ok(db) => {
            let library = LibraryInfo::of(&db);
            let wake = Arc::new(WakeSignal::new());
            match Handle::spawn(db, Box::new(SystemClock), wake.waker()) {
                Ok(handle) => {
                    let snapshot = (*handle.snapshot()).clone();
                    shell_over(Some((Rc::new(handle), wake)), snapshot, true, library, None)
                        .into_element()
                }
                Err(error) => {
                    let why = crate::logging::describe(&error);
                    log::error!("store worker failed to start: {why}");
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
