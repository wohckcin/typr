//! The settings screen, grouped as Review, Appearance and Library. A setting is checked before it is
//! accepted; a refused value keeps the previous one and names the setting and its limit.
//!
//! Persistence to the library database is not part of this screen: accepted values are reported
//! through callbacks, and the caller stores them.

use nsh_core::prelude::*;

use crate::ui::{input::field, status::badge, toggle::toggle, tokens};

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
        return Err(format!("New cards a day must be {NEW_CARD_CAP_MAX} or less"));
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

fn section(name: &'static str, body: impl IntoElement) -> impl IntoElement {
    view()
        .gap_2()
        .child(badge(name))
        .child(body)
}

/// The settings screen over `values`. `on_change` is called with the cap in force after a change.
pub fn settings(values: Values, on_change: impl Fn(u32) + Clone + 'static) -> impl IntoElement {
    let cap_changed = on_change.clone();
    let accept = move || {
        values.accept_cap();
        cap_changed(values.cap.get_untracked());
    };
    view()
        .flex_1()
        .gap_4()
        .p_4()
        .child(section(
            "Review",
            view()
                .gap_2()
                .child(field("New cards a day", values.cap_text))
                .child(dynamic(move || {
                    match values.refusal.get() {
                        Some(reason) => text(reason).text_xs().text_color(move || tokens::destructive()).into_element(),
                        None => view().into_element(),
                    }
                }))
                .child(crate::ui::button::button(crate::ui::button::Variant::Default, "Apply", || false, accept)),
        ))
        .child(section("Appearance", toggle("Show review counts", values.show_counts)))
        .child(section("Library", text("Database, attachments and log paths are shown here.").text_sm().text_color(move || tokens::muted_foreground())))
}
