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

/// The most new cards a day may be asked for.
pub const NEW_CARD_CAP_MAX: u32 = 10_000;

/// Checks a new-card cap typed by the user. The error names the limit so the user knows what is
/// allowed.
pub fn check_new_card_cap(text: &str) -> Result<u32, String> {
    let value: u32 = text
        .trim()
        .parse()
        .map_err(|_| format!("New cards a day must be a whole number up to {NEW_CARD_CAP_MAX}"))?;
    if value > NEW_CARD_CAP_MAX {
        return Err(format!(
            "New cards a day must be {NEW_CARD_CAP_MAX} or less"
        ));
    }
    Ok(value)
}

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
    /// Whether the interface draws reviews' counts.
    pub show_counts: Signal<bool>,
    /// The appearance in force. The caller replaces this with its own signal to store and apply it.
    pub theme: Signal<Theme>,
    /// What was typed in "Find a setting".
    pub query: Signal<String>,
    /// The refusal shown under the cap, if the last edit was refused.
    pub refusal: Signal<Option<SharedString>>,
}

impl Values {
    /// Values starting at `cap` with counts shown or hidden.
    pub fn new(cap: u32, show_counts: bool) -> Self {
        Self {
            cap_text: signal(cap.to_string()),
            cap: signal(cap),
            show_counts: signal(show_counts),
            theme: signal(Theme::default()),
            query: signal(String::new()),
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

/// Whether a row labelled `label` shows for `query`: every row for an empty query, otherwise the
/// rows whose label contains the text, ignoring case.
pub fn row_matches(label: &str, query: &str) -> bool {
    let query = query.trim().to_lowercase();
    query.is_empty() || label.to_lowercase().contains(&query)
}

/// The header: where the screen is, and a field that narrows the rows by name.
fn header(values: Values) -> impl IntoElement {
    view()
        .flex_row()
        .flex_wrap()
        .items_center()
        .justify_between()
        .gap_3()
        .child(
            view()
                .flex_row()
                .gap_2()
                .child(text("Settings").text_sm().text_color(tokens::muted_foreground))
                .child(text("/").text_sm().text_color(tokens::muted_foreground))
                .child(text("General").text_sm().text_color(tokens::foreground)),
        )
        .child(
            view().flex_1().child(
                field()
                    .size(Size::Sm)
                    .placeholder("Find a setting")
                    .value(move || SharedString::from(values.query.get()))
                    .on_change(move |text: &String| values.query.set(text.to_string())),
            ),
        )
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
                    view().flex_1().child(
                        field()
                            .size(Size::Sm)
                            .placeholder("20")
                            .value(move || SharedString::from(values.cap_text.get()))
                            .on_change(move |text: &String| values.cap_text.set(text.to_string())),
                    ),
                )
                .child(
                    button()
                        .variant(Variant::Default)
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
    // keeps its natural width and the button wraps under it when the two do not fit.
    view()
        .flex_row()
        .flex_wrap()
        .items_center()
        .justify_end()
        .gap_2()
        .child(
            text(shown)
                .text_sm()
                .text_color(tokens::muted_foreground),
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
                        let _ = reveal_path(path).await;
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
) -> impl IntoElement {
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
        (
            "Show review counts",
            Rc::new(move || {
                segmented(
                    vec![("Off", false), ("On", true)],
                    move || values.show_counts.get(),
                    move |on| values.show_counts.set(on),
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
                .child(badge().label(format!("{bytes} bytes")))
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
    let labels: Vec<&'static str> = list.iter().map(|(label, _)| *label).collect();
    let mut card = rows();
    for (label, build) in list {
        card = card.child(when(
            move || row_matches(label, &values.query.get()),
            move || setting_row(label, build()),
        ));
    }
    view()
        .flex_1()
        .gap_4()
        .p_4()
        .child(header(values))
        .child(card)
        .child(when(
            move || {
                let query = values.query.get();
                labels.iter().all(|label| !row_matches(label, &query))
            },
            || {
                text("No setting matches")
                    .text_sm()
                    .text_color(tokens::muted_foreground)
            },
        ))
}
