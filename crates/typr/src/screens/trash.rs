//! The trash screen: the cards that were deleted and can still come back, newest first, each with
//! Restore, and Empty trash behind a confirmation drawn on the screen itself.

use nsh_core::prelude::*;
use typr_core::store::{Id, Snapshot};

use crate::ui::{
    button::{Variant, button},
    surface, tokens,
};

/// One trashed card as the screen lists it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct TrashRow {
    /// The card.
    pub id: Id,
    /// What the card said, cut short.
    pub preview: String,
    /// The name of the deck it will return to.
    pub deck: String,
    /// When it was trashed, in milliseconds.
    pub trashed_at: i64,
}

/// The trashed cards of `snapshot`, most recently trashed first.
pub fn trash_rows(snapshot: &Snapshot) -> Vec<TrashRow> {
    let mut rows: Vec<TrashRow> = snapshot
        .cards
        .iter()
        .filter_map(|card| {
            let trashed_at = card.trashed_at?;
            Some(TrashRow {
                id: card.id,
                preview: card.preview.clone(),
                deck: snapshot
                    .decks
                    .iter()
                    .find(|deck| deck.id == card.deck)
                    .map(|deck| deck.name.clone())
                    .unwrap_or_default(),
                trashed_at,
            })
        })
        .collect();
    rows.sort_by_key(|row| std::cmp::Reverse(row.trashed_at));
    rows
}

/// The trash over `snapshot`. `on_restore` brings one card back; `on_empty` destroys every trashed
/// card, and runs only after the confirming press.
pub fn trash(
    snapshot: Signal<Snapshot>,
    on_restore: impl Fn(Id) + Clone + 'static,
    on_empty: impl Fn() + Clone + 'static,
) -> impl IntoElement {
    let confirming = signal(false);
    view().flex_1().gap_4().p_4().child(dynamic(move || {
        let rows = snapshot.with(trash_rows);
        if rows.is_empty() {
            confirming.set(false);
            return view()
                .gap_1()
                .child(
                    text("Trash is empty")
                        .text_sm()
                        .text_color(tokens::foreground),
                )
                .child(
                    text("Deleted cards stay here until you empty the trash")
                        .text_xs()
                        .text_color(tokens::muted_foreground),
                )
                .into_element();
        }
        let count = rows.len();
        let mut list = view().gap_2();
        for row in rows {
            let restore = on_restore.clone();
            let id = row.id;
            let preview = if row.preview.is_empty() {
                "(no text)".to_owned()
            } else {
                row.preview
            };
            list = list.child(
                surface::surface().child(
                    view()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .gap_3()
                        .child(
                            view()
                                .flex_1()
                                .child(text(preview).text_sm().text_color(tokens::foreground))
                                .child(
                                    text(row.deck)
                                        .text_xs()
                                        .text_color(tokens::muted_foreground),
                                ),
                        )
                        .child(
                            button()
                                .variant(Variant::Ghost)
                                .label("Restore")
                                .disabled(|| false)
                                .on_press(move || restore(id)),
                        ),
                ),
            );
        }
        let on_empty = on_empty.clone();
        // Read inside the one dynamic, not a nested one: a press rebuilds the rows under this owner
        // instead of one the next render disposes.
        let footer = {
            if confirming.get() {
                let on_empty = on_empty.clone();
                view()
                    .flex_row()
                    .flex_wrap()
                    .items_center()
                    .gap_2()
                    .child(
                        text(format!(
                            "Destroy {count} {}? This cannot be undone.",
                            if count == 1 { "card" } else { "cards" }
                        ))
                        .text_sm()
                        .text_color(tokens::destructive),
                    )
                    .child(
                        button()
                            .variant(Variant::Ghost)
                            .label("Cancel")
                            .disabled(|| false)
                            .on_press(move || confirming.set(false)),
                    )
                    .child(
                        button()
                            .variant(Variant::Destructive)
                            .label("Destroy")
                            .disabled(|| false)
                            .on_press(move || {
                                confirming.set(false);
                                on_empty();
                            }),
                    )
                    .into_element()
            } else {
                button()
                    .variant(Variant::Ghost)
                    .label("Empty trash")
                    .disabled(|| false)
                    .on_press(move || confirming.set(true))
                    .into_element()
            }
        };
        view().gap_4().child(footer).child(list).into_element()
    }))
}
