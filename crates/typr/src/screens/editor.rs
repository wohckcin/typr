//! The card editor: the markdown source on one side, its sides as previewed on the other.
//!
//! Saving is explicit. The screen reports the text through `on_save`; the caller writes it. A card
//! with no content is not saved: its save button is disabled.

use nsh_core::prelude::*;
use typr_core::sides;

use crate::ui::{
    button::{Variant, button},
    prose::prose,
    source::source,
    status::badge,
};

/// Whether the text holds anything once trimmed; an empty card is not saved.
pub fn has_content(markdown: &str) -> bool {
    !markdown.trim().is_empty()
}

/// A side as its preview shows it: hidden answers masked, the rest as written.
pub fn preview_of(side: &str) -> String {
    let shape = typr_core::hidden::shape(side);
    match &shape {
        typr_core::hidden::Shape::Hidden { .. } => typr_core::hidden::masked_all(side, &shape),
        typr_core::hidden::Shape::Sides(_) => side.to_owned(),
    }
}

/// The card editor over `initial`. Saving reports the text through `on_save`.
pub fn editor(initial: String, on_save: impl Fn(String) + Clone + 'static) -> impl IntoElement {
    let draft = signal(initial.clone());
    let saved = signal(initial);
    let save_text = on_save.clone();
    let state_badge = dynamic(move || {
        let dirty = draft.with(|d| saved.with(|s| d != s));
        badge(if dirty { "Unsaved" } else { "Saved" }).into_element()
    });
    view()
        .flex_1()
        .flex_row()
        .gap_3()
        .p_4()
        .child(
            view()
                .flex_1()
                .gap_2()
                .child(state_badge)
                .child(source(draft))
                .child(button(
                    Variant::Primary,
                    "Save",
                    move || draft.with(|d| !has_content(d)),
                    move || {
                        let text = draft.get_untracked();
                        saved.set(text.clone());
                        save_text(text);
                    },
                )),
        )
        .child(
            view()
                .flex_1()
                .gap_3()
                .child(dynamic(move || {
                    let text = draft.get();
                    let sides_of: Vec<String> = sides::split(&text).iter().map(|s| s.to_string()).collect();
                    let mut column = view().gap_3();
                    for side in sides_of {
                        column = column.child(prose(move || preview_of(&side).into()));
                    }
                    column.into_element()
                }))
        )
}
