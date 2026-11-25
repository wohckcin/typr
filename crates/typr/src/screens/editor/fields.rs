//! The field table a templated card fills, and the template rendered from it.

use nsh_core::element::JsObject;
use nsh_core::prelude::*;

use crate::ui::{input::field, status::badge, tokens};

/// One row of a card's field table: the name a template uses, the value typed for it, and whether
/// the app computes it (which makes it read-only).
#[derive(Clone, Debug, Default, PartialEq, JsObject)]
#[non_exhaustive]
pub struct TemplateField {
    pub name: String,
    pub value: String,
    pub computed: bool,
}

impl TemplateField {
    /// A field the user fills.
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> TemplateField {
        TemplateField {
            name: name.into(),
            value: value.into(),
            computed: false,
        }
    }

    /// A field the app works out, drawn read-only.
    pub fn computed(name: impl Into<String>, value: impl Into<String>) -> TemplateField {
        TemplateField {
            computed: true,
            ..TemplateField::new(name, value)
        }
    }
}

/// The fields a card's template fills from, as the editor shows them: a value per name, computed
/// rows read-only, and `on_edit(name, value)` when the user types one.
pub fn field_table(
    fields: Vec<TemplateField>,
    on_edit: impl Fn(&str, &str) + Clone + 'static,
) -> impl IntoElement {
    let mut column = view().gap_2().child(badge().label("Fields"));
    for row in fields {
        let name = row.name.clone();
        let edit = on_edit.clone();
        if row.computed {
            column = column.child(
                view()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(
                        text(row.name)
                            .text_xs()
                            .text_color(tokens::muted_foreground),
                    )
                    .child(text(row.value).text_sm().text_color(tokens::foreground))
                    .child(badge().label("computed")),
            );
        } else {
            let value = signal(row.value);
            let edited = name.clone();
            column = column.child(
                view()
                    .gap_1()
                    .child(text(name).text_xs().text_color(tokens::muted_foreground))
                    .child(
                        field()
                            .placeholder("Value")
                            .value(move || SharedString::from(value.get()))
                            .on_change(move |ev: &crate::ui::input::FieldInput| {
                                value.set(ev.text.clone());
                                edit(&edited, &ev.text);
                            }),
                    ),
            );
        }
    }
    column
}

/// A card's text as its template renders it, for the editor's preview of a templated card.
pub fn template_preview(template: &str, fields: &[(String, String)], deck: Option<&str>) -> String {
    typr_core::template::render_card(template, fields, deck)
}
