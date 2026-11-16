//! Every screen in both appearances: the same text keeps its size and weight, and its colour is the
//! theme's, so it changes between light and dark. No pixels: the engine resolves the style.

use std::path::PathBuf;

use nsh_core::element::Component;
use nsh_core::platform::ColorScheme;
use nsh_core::prelude::*;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;
use typr_core::keymap::Surface;
use typr_core::session::{Entry, Session};
use typr_core::screen::Queue;
use typr_core::store::Snapshot;

use typr::screens::diagnostics::{SlowCommand, diagnostics};
use typr::screens::editor::editor;
use typr::screens::embed::embedded;
use typr::screens::grid::{Tile, grid};
use typr::screens::history::{Record, history};
use typr::screens::keymap::palette;
use typr::screens::library::library;
use typr::screens::settings::{LibraryInfo, Values, settings};
use typr::screens::sidebar::{Group, Item, Sidebar, sidebar};
use typr::screens::study::study;

/// The resolved style of the first text matching `selector`: font size in bits, weight, colour.
fn resolved<C: Component>(scheme: ColorScheme, selector: &str) -> (u32, u16, String)
where
    C::Props: Default,
{
    let mut config = EngineConfig::new(Root::of::<C>());
    config.environment.color_scheme = scheme;
    let mut app = Headless::start(config).expect("the screen starts");
    app.wait_for(selector).expect("the screen draws its text");
    let style = app.screen().span_style(selector).expect("the style resolves");
    (style.font_size.0.to_bits(), style.font_weight, format!("{:?}", style.color))
}

/// Checks one screen in both appearances.
macro_rules! check {
    ($test:ident, $screen:expr, $root:ty, $selector:expr) => {
        #[test]
        fn $test() {
            let (size_light, weight_light, colour_light) = resolved::<$root>(ColorScheme::Light, $selector);
            let (size_dark, weight_dark, colour_dark) = resolved::<$root>(ColorScheme::Dark, $selector);
            assert_eq!(size_light, size_dark, "{} moved its font size between appearances", $screen);
            assert_eq!(weight_light, weight_dark, "{} moved its weight between appearances", $screen);
            assert_ne!(colour_light, colour_dark, "{} resolved one colour in both appearances", $screen);
        }
    };
}

#[component]
fn SidebarScreen() -> impl IntoElement {
    typr::ui::tokens::install();
    sidebar(Sidebar::new(
        vec![Item {
            key: "all".into(),
            label: "All cards".into(),
            group: Group::Navigation,
            parent: None,
            due: 0,
            has_children: false,
        }],
        true,
        "search",
    ))
}

#[component]
fn LibraryScreen() -> impl IntoElement {
    typr::ui::tokens::install();
    library(Snapshot::default(), true, 0)
}

#[component]
fn GridScreen() -> impl IntoElement {
    typr::ui::tokens::install();
    grid(
        computed(|| {
            vec![Tile {
                id: 1,
                preview: "Front".into(),
                hidden_sides: 1,
                tags: Vec::new(),
                schedule: typr::screens::grid::Schedule::New,
                pinned: false,
            }]
        }),
        || {},
    )
}

#[component]
fn EditorScreen() -> impl IntoElement {
    typr::ui::tokens::install();
    editor("Question".to_owned(), |_| {}, |_| Ok("unused.png".to_owned()))
}

#[component]
fn StudyScreen() -> impl IntoElement {
    typr::ui::tokens::install();
    study(
        Session::new(Queue::Due, vec![Entry::sides(1, ["Question", "Answer"], 0)]),
        |_, _| true,
        || {},
        || {},
    )
}

#[component]
fn HistoryScreen() -> impl IntoElement {
    typr::ui::tokens::install();
    history(vec![Record {
        phase: "review".into(),
        grade: "good".into(),
        at: "09:00".into(),
        due: "tomorrow".into(),
    }])
}

#[component]
fn SettingsScreen() -> impl IntoElement {
    typr::ui::tokens::install();
    settings(
        Values::new(20, true),
        LibraryInfo {
            database: PathBuf::from("/tmp/typr/typr.sqlite3"),
            ..LibraryInfo::default()
        },
        |_| {},
        || {},
        || {},
    )
}

#[component]
fn KeymapScreen() -> impl IntoElement {
    typr::ui::tokens::install();
    palette(Surface::Library, |_| {})
}

#[component]
fn DiagnosticsScreen() -> impl IntoElement {
    typr::ui::tokens::install();
    diagnostics(
        signal(false),
        vec![SlowCommand { name: "load deck".into(), millis: 740 }],
        signal(None),
        || {},
        || {},
    )
}

#[component]
fn EmbedScreen() -> impl IntoElement {
    typr::ui::tokens::install();
    embedded("Body of the note", Vec::new(), "Note".to_owned())
}

check!(appearance_sidebar, "sidebar", SidebarScreen, "text:All cards");
check!(appearance_library, "library", LibraryScreen, "text:Search");
check!(appearance_grid, "grid", GridScreen, "text:Front");
check!(appearance_editor, "editor", EditorScreen, "text:Question >> nth=0");
check!(appearance_study, "study", StudyScreen, "text:Question");
check!(appearance_history, "history", HistoryScreen, "text:/review · good/");
check!(appearance_settings, "settings", SettingsScreen, "text:Review");
check!(appearance_keymap, "keymap", KeymapScreen, "text:Review cards that are due");
check!(appearance_diagnostics, "diagnostics", DiagnosticsScreen, "text:Slow commands");
check!(appearance_embed, "embed", EmbedScreen, "text:Body of the note");

/// The screens checked here are exactly the ones the crate declares: adding a screen without a
/// light/dark check fails here, by name.
#[test]
fn every_screen_has_a_light_and_dark_check() {
    let checked = [
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
    assert_eq!(checked, typr::screens::NAMES, "a screen was added without a light/dark check");
}
