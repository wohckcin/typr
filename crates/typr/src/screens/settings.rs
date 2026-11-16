//! The settings screen, grouped as Review, Appearance and Library. A setting is checked before it is
//! accepted; a refused value keeps the previous one and names the setting and its limit.
//!
//! Persistence to the library database is not part of this screen: accepted values are reported
//! through callbacks, and the caller stores them. The Library group reports the file itself — its
//! path, schema version, size, the attachments folder and the log path — each with a control that
//! shows it in the file manager, plus the import and export actions.

use std::path::PathBuf;

use nsh_core::prelude::*;
use typr_core::store::Db;

use crate::ui::{
    button::{Variant, button},
    input::field,
    status::badge,
    toggle::toggle,
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
#[derive(Clone, Copy)]
pub struct Values {
    /// The typed cap, as the user wrote it.
    pub cap_text: Signal<String>,
    /// The cap in force: the last accepted value.
    pub cap: Signal<u32>,
    /// Whether the interface draws reviews' counts.
    pub show_counts: Signal<bool>,
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

fn section(name: &'static str, body: impl IntoElement) -> impl IntoElement {
    // A section's name is a label, not a chip: a badge here drew a full-width bordered box that
    // read as an empty field.
    view()
        .gap_2()
        .child(text(name).text_xs().text_color(tokens::muted_foreground))
        .child(body)
}

/// One reported path: the full path, and a control that shows it in the file manager.
fn path_row(label: &'static str, path: PathBuf) -> impl IntoElement {
    let shown = path.display().to_string();
    let reveal = path.clone();
    view()
        .gap_1()
        .child(text(label).text_xs().text_color(tokens::muted_foreground))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(text(shown).text_sm().text_color(tokens::foreground))
                .child(button(
                    Variant::Ghost,
                    move || -> SharedString { format!("Show {label} in the file manager").into() },
                    || false,
                    move || {
                        let path = reveal.clone();
                        spawn(async move {
                            let _ = reveal_path(path).await;
                        });
                    },
                )),
        )
}

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
    let database = path_row("Database", library.database.clone());
    let attachments = path_row("Attachments", library.attachments.clone());
    let logs = path_row("Log", library.logs.clone());
    let (schema, bytes) = (library.schema, library.bytes);
    view()
        .flex_1()
        .gap_4()
        .p_4()
        .child(section(
            "Review",
            view()
                .gap_2()
                .child(field("New cards a day", values.cap_text))
                .child(dynamic(move || match values.refusal.get() {
                    Some(reason) => text(reason)
                        .text_xs()
                        .text_color(tokens::destructive)
                        .into_element(),
                    None => view().into_element(),
                }))
                .child(view().flex_row().child(button(
                    Variant::Default,
                    "Apply",
                    || false,
                    accept,
                ))),
        ))
        .child(section(
            "Appearance",
            toggle("Show review counts", values.show_counts),
        ))
        .child(section(
            "Library",
            view()
                .gap_3()
                .child(database)
                .child(attachments)
                .child(logs)
                .child(
                    view()
                        .flex_row()
                        .gap_2()
                        .child(badge(format!("Schema {schema}")))
                        .child(badge(format!("{} bytes", bytes))),
                )
                .child(
                    view()
                        .flex_row()
                        .gap_2()
                        .child(button(Variant::Default, "Import", || false, on_import))
                        .child(button(Variant::Default, "Export", || false, on_export)),
                ),
        ))
}
