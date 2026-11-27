//! The settings screen, grouped as Review, Appearance and Library. A setting is checked before it is
//! accepted; a refused value keeps the previous one and names the setting and its limit.
//!
//! Persistence to the library database is not part of this screen: accepted values are reported
//! through callbacks, and the caller stores them. The Library group reports the file itself — its
//! path, schema version, size, the attachments folder and the log path — each with a control that
//! shows it in the file manager, plus the import and export actions.

use std::path::PathBuf;
use std::rc::Rc;

use nsh_core::prelude::*;
pub use typr_core::store::check_new_card_cap;
use typr_core::store::{Db, Theme};

use crate::ui::{
    button::{Variant, button},
    input::field,
    rows::{rows, setting_row},
    segmented::segmented,
    size::Size,
    status::badge,
    tokens,
};

/// The values the screen edits. Each is a signal the caller owns.
///
/// Built by [`Values::new`] alone: its fields are signals created under the caller's owner, so a
/// `Default` would make values nobody created.
#[derive(Clone, Copy)]
#[non_exhaustive]
pub struct Values {
    /// The typed cap, as the user wrote it.
    pub cap_text: Signal<String>,
    /// The cap in force: the last accepted value.
    pub cap: Signal<u32>,
    /// The appearance in force. The caller replaces this with its own signal to store and apply it.
    pub theme: Signal<Theme>,
    /// The refusal shown under the cap, if the last edit was refused.
    pub refusal: Signal<Option<SharedString>>,
}

impl Values {
    /// Values starting at `cap`.
    pub fn new(cap: u32) -> Self {
        Self {
            cap_text: signal(cap.to_string()),
            cap: signal(cap),
            theme: signal(Theme::default()),
            refusal: signal(None),
        }
    }

    /// Takes the typed cap if it passes; otherwise keeps the previous one and states why.
    pub fn accept_cap(&self) {
        let typed = self.cap_text.get_untracked();
        match check_new_card_cap(&typed) {
            Ok(value) => {
                self.cap.set(value);
                self.refusal.set(None);
            }
            Err(reason) => {
                self.cap_text.set(self.cap.get_untracked().to_string());
                self.refusal.set(Some(reason.into()));
            }
        }
    }
}

/// Where the library keeps its files, and what the file records about itself.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct LibraryInfo {
    /// The SQLite file the library lives in.
    pub database: PathBuf,
    /// The schema version the file is at after this open.
    pub schema: i64,
    /// The file's size in bytes.
    pub bytes: u64,
    /// Where a card's pictures are copied.
    pub attachments: PathBuf,
    /// Where the app writes its log.
    pub logs: PathBuf,
}

impl LibraryInfo {
    /// What `db` reports about its own file. A folder or file that cannot be measured is left out of
    /// the numbers rather than failing the screen.
    pub fn of(db: &Db) -> Self {
        let database = db.path().to_path_buf();
        let bytes = std::fs::metadata(&database)
            .map(|meta| meta.len())
            .unwrap_or(0);
        Self {
            schema: db.schema_facts().written,
            attachments: database
                .parent()
                .map(|dir| dir.join("attachments"))
                .unwrap_or_default(),
            logs: database
                .parent()
                .map(|dir| dir.join("logs"))
                .unwrap_or_default(),
            database,
            bytes,
        }
    }
}

/// A byte count as a reader takes it in: bytes, whole kilobytes, or megabytes to one place.
pub fn human_size(bytes: u64) -> String {
    match bytes {
        0..1024 => format!("{bytes} bytes"),
        1024..1_048_576 => format!("{} KB", bytes / 1024),
        _ => format!("{:.1} MB", bytes as f64 / 1_048_576.0),
    }
}

/// The screen's title.
fn header() -> impl IntoElement {
    text("Settings").text_sm().text_color(tokens::foreground)
}

/// The control of the new-card row: a field that takes the room there is, and Apply.
fn cap_control(values: Values, accept: impl Fn() + 'static) -> impl IntoElement {
    view()
        .flex_1()
        .gap_1()
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    view().w_20().child(
                        field()
                            .size(Size::Sm)
                            .placeholder("20")
                            .value(move || SharedString::from(values.cap_text.get()))
                            .on_change(move |ev: &crate::ui::input::FieldInput| {
                                values.cap_text.set(ev.text.clone())
                            }),
                    ),
                )
                .child(
                    button()
                        .variant(Variant::Default)
                        .size(Size::Sm)
                        .label("Apply")
                        .disabled(|| false)
                        .on_press(accept),
                ),
        )
        .child(dynamic(move || match values.refusal.get() {
            Some(reason) => text(reason)
                .text_xs()
                .text_color(tokens::destructive)
                .into_element(),
            None => view().into_element(),
        }))
}

/// One reported path as a row's control: the full path, and a button that shows it in the file
/// manager.
fn path_control(label: &'static str, path: PathBuf) -> impl IntoElement {
    let shown = path.display().to_string();
    let reveal = path.clone();
    // The path is in full, never elided: a path cut to nothing in a narrow window is not a path. It
    // wraps inside the row, and the button wraps under it when the two do not fit.
    view()
        .w_full()
        .flex_row()
        .flex_wrap()
        .items_center()
        .justify_end()
        .gap_2()
        .child(
            text(shown)
                .text_sm()
                .text_color(tokens::muted_foreground)
                .flex_1()
                .min_w_0(),
        )
        .child(
            button()
                .variant(Variant::Ghost)
                .label(move || -> SharedString {
                    format!("Show {label} in the file manager").into()
                })
                .disabled(|| false)
                .on_press(move || {
                    let path = reveal.clone();
                    spawn(async move {
                        if let Err(error) = reveal_path(path).await {
                            log::warn!("the file manager did not open: {error}");
                        }
                    });
                }),
        )
}

type Build = Rc<dyn Fn() -> AnyElement>;

/// The settings screen over `values` and the library's own file. `on_change` is called with the cap
/// in force after a change; `on_import` and `on_export` run the caller's dialogs.
pub fn settings(
    values: Values,
    library: LibraryInfo,
    on_change: impl Fn(u32) + Clone + 'static,
    on_import: impl Fn() + 'static,
    on_export: impl Fn() + 'static,
    on_diagnostics: impl Fn() + 'static,
) -> impl IntoElement {
    let diagnostics = Rc::new(on_diagnostics);
    let cap_changed = on_change.clone();
    let accept = move || {
        values.accept_cap();
        cap_changed(values.cap.get_untracked());
    };
    let (schema, bytes) = (library.schema, library.bytes);
    let (import, export) = (Rc::new(on_import), Rc::new(on_export));
    // Review, Appearance, Library: the order carries the grouping, as there are no headings.
    let mut list: Vec<(&'static str, Build)> = vec![
        (
            "New cards a day",
            Rc::new(move || cap_control(values, accept.clone()).into_element()),
        ),
        (
            "Theme",
            Rc::new(move || {
                segmented(
                    Theme::ALL.map(|theme| (theme.label(), theme)).to_vec(),
                    move || values.theme.get(),
                    move |theme| values.theme.set(theme),
                )
                .into_element()
            }),
        ),
    ];
    for (label, path) in [
        ("Database", library.database.clone()),
        ("Attachments", library.attachments.clone()),
        ("Log", library.logs.clone()),
    ] {
        list.push((
            label,
            Rc::new(move || path_control(label, path.clone()).into_element()),
        ));
    }
    list.push((
        "Library file",
        Rc::new(move || {
            view()
                .flex_row()
                .gap_2()
                .child(badge().label(format!("Schema {schema}")))
                .child(badge().label(human_size(bytes)))
                .into_element()
        }),
    ));
    list.push((
        "Import a library",
        Rc::new(move || {
            let import = import.clone();
            button()
                .variant(Variant::Default)
                .label("Import")
                .disabled(|| false)
                .on_press(move || import())
                .into_element()
        }),
    ));
    list.push((
        "Export the library",
        Rc::new(move || {
            let export = export.clone();
            button()
                .variant(Variant::Default)
                .label("Export")
                .disabled(|| false)
                .on_press(move || export())
                .into_element()
        }),
    ));
    list.push((
        "Diagnostics",
        Rc::new(move || {
            let diagnostics = diagnostics.clone();
            button()
                .variant(Variant::Ghost)
                .label("Open diagnostics")
                .disabled(|| false)
                .on_press(move || diagnostics())
                .into_element()
        }),
    ));
    let mut card = rows();
    for (label, build) in list {
        card = card.child(setting_row(label, build()));
    }
    // Wrapped paths make the rows taller than a short window, so the screen scrolls.
    scroll()
        .flex_1()
        .child(view().gap_4().p_4().child(header()).child(card))
}
