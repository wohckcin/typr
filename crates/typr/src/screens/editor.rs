//! The card editor: the markdown source on one side, its sides as previewed on the other.
//!
//! Saving is explicit. The screen reports the text through `on_save`; the caller writes it. A card
//! with no content is not saved: its save button is disabled.

use std::path::{Path, PathBuf};

use nsh_core::prelude::*;
use nsh_core::file_dialog::OpenDialog;
use typr_core::keymap::{Command, Surface};
use typr_core::sides;

use crate::screens::keymap::keystroke_of;

use crate::ui::{
    button::{Variant, button},
    input::field_with,
    overlay::decision,
    prose::prose,
    source::source_with,
    status::badge,
    tokens,
};

/// Whether the text holds anything once trimmed; an empty card is not saved.
pub fn has_content(markdown: &str) -> bool {
    !markdown.trim().is_empty()
}

/// The file name a chosen picture is copied to, which is also the reference the card holds.
///
/// A copy that cannot be made is refused rather than reported: a card keeps its previous picture
/// and the caller shows why.
pub fn attach_image(source: &Path, root: &Path) -> std::io::Result<String> {
    let name = source
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "the file has no name"))?;
    std::fs::create_dir_all(root)?;
    std::fs::copy(source, root.join(name))?;
    Ok(name.to_owned())
}

/// The image reference written into the card: `![alt](name)`, so a card's text stays a markdown
/// image and the store copies the picture beside the library.
pub fn image_reference(alt: &str, name: &str) -> String {
    format!("![{alt}]({name})")
}

/// The card's text with `![alt](name)` rewritten to the absolute path under `root`, so the renderer
/// loads the copy beside the library. A reference that escapes `root` (an absolute path, a `..`
/// segment) or names nothing is left as written, which draws the renderer's placeholder.
pub fn resolve_images(markdown: &str, root: &Path) -> String {
    if root.as_os_str().is_empty() {
        return markdown.to_owned();
    }
    let mut out = String::with_capacity(markdown.len());
    let mut rest = markdown;
    while let Some(at) = rest.find("![") {
        out.push_str(&rest[..at]);
        let after = &rest[at..];
        let Some(close) = after.find("](") else {
            out.push_str(after);
            return out;
        };
        let Some(end) = after[close + 2..].find(')') else {
            out.push_str(after);
            return out;
        };
        let src = &after[close + 2..close + 2 + end];
        let inside = !src.trim().is_empty()
            && Path::new(src.trim()).is_absolute().eq(&false)
            && !Path::new(src.trim())
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir));
        out.push_str(&after[..close + 2]);
        if inside {
            out.push_str(&root.join(src.trim()).display().to_string());
        } else {
            out.push_str(src);
        }
        out.push(')');
        rest = &after[close + 2 + end + 1..];
    }
    out.push_str(rest);
    out
}

/// The prompts a card's text holds: one per numbered hidden-text group, the plain group as one, or
/// one for a card of two or more sides. A note holds none.
pub fn prompt_count(markdown: &str) -> usize {
    let shape = typr_core::hidden::shape(markdown);
    if let typr_core::hidden::Shape::Sides(sides) = &shape {
        return usize::from(sides.len() >= 2);
    }
    shape.prompt_keys().len()
}

/// Whether a card's text names any local image, and the reference of each, in order.
pub fn image_references(markdown: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = markdown;
    while let Some(at) = rest.find("![") {
        let after = &rest[at + 2..];
        let Some(close) = after.find("](") else { break };
        let Some(end) = after[close + 2..].find(')') else { break };
        found.push(after[close + 2..close + 2 + end].to_owned());
        rest = &after[close + 2 + end + 1..];
    }
    found
}

/// A side as its preview shows it: hidden answers masked, the rest as written.
pub fn preview_of(side: &str) -> String {
    let shape = typr_core::hidden::shape(side);
    match &shape {
        typr_core::hidden::Shape::Hidden { .. } => typr_core::hidden::masked_all(side, &shape),
        typr_core::hidden::Shape::Sides(_) => side.to_owned(),
    }
}

/// The card editor over `initial`. Saving reports the text through `on_save`; `on_attach` copies a
/// chosen picture into `attachments` and returns its file name, which is what the card references.
pub fn editor(
    initial: String,
    on_save: impl Fn(String) + Clone + 'static,
    on_attach: impl Fn(PathBuf) -> std::io::Result<String> + Clone + 'static,
) -> impl IntoElement {
    editor_with_draft(initial, None, PathBuf::new(), on_save, on_attach, |_| {})
}

/// The editor opening on an unsaved draft when there is one, and reporting each edit so the caller
/// can write the draft down. A draft that differs from the saved text opens marked "Unsaved".
pub fn editor_with_draft(
    saved_text: String,
    draft_text: Option<String>,
    attachments: PathBuf,
    on_save: impl Fn(String) + Clone + 'static,
    on_attach: impl Fn(PathBuf) -> std::io::Result<String> + Clone + 'static,
    on_draft: impl Fn(&str) + 'static,
) -> impl IntoElement {
    editor_full(
        saved_text,
        draft_text,
        attachments,
        None,
        on_save,
        on_attach,
        on_draft,
        || {},
    )
}

/// A template in force on the card: its text, the fields it fills from, and the deck's name.
#[derive(Clone, Debug, PartialEq)]
pub struct Template {
    pub text: String,
    pub fields: Vec<Field>,
    pub deck: Option<String>,
}

/// The whole editor: the source, the preview (from the template when one is in force), the field
/// table, the image action, and the Save/Discard/Stay question before leaving unsaved work.
pub fn editor_full(
    saved_text: String,
    draft_text: Option<String>,
    attachments: PathBuf,
    template: Option<Template>,
    on_save: impl Fn(String) + Clone + 'static,
    on_attach: impl Fn(PathBuf) -> std::io::Result<String> + Clone + 'static,
    on_draft: impl Fn(&str) + 'static,
    on_leave: impl Fn() + Clone + 'static,
) -> impl IntoElement {
    let asking = signal(false);
    let save_and_leave = on_leave.clone();
    let discard = on_leave.clone();
    let draft = signal(draft_text.unwrap_or_else(|| saved_text.clone()));
    let saved = signal(saved_text.clone());
    let preview_template = template.clone();
    let fields = signal(
        template
            .as_ref()
            .map(|t| {
                t.fields
                    .iter()
                    .filter(|field| !field.computed)
                    .map(|field| (field.name.clone(), field.value.clone()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
    );
    let attach_error = signal(None::<SharedString>);
    let save_text = on_save.clone();
    let save_on_leave = on_save.clone();
    // Leaving asks first only when the draft differs from what was saved.
    let leave_now = on_leave.clone();
    let dirty = move || draft.with(|d| saved.with(|s| d != s));
    if let Some(key) = keystroke_of(Surface::Editor, Command::Leave) {
        on_key(&key, move |_| {
            if dirty() {
                asking.set(true);
            } else {
                leave_now();
            }
        });
    }
    let attach = on_attach.clone();
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
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .child(state_badge)
                        .child(dynamic(move || {
                            let count = prompt_count(&draft.get());
                            let label = match count {
                                0 => "note".to_owned(),
                                1 => "1 prompt".to_owned(),
                                n => format!("{n} prompts"),
                            };
                            badge(label).into_element()
                        })),
                )
                .child(source_with(draft, on_draft))
                .child(button(
                    Variant::Primary,
                    "Save",
                    move || draft.with(|d| !has_content(d)),
                    move || {
                        let text = draft.get_untracked();
                        saved.set(text.clone());
                        save_text(text);
                    },
                ))
                .child({
                    let attach = attach.clone();
                    button(
                        Variant::Default,
                        "Attach image",
                        || false,
                        {
                            let attach = attach.clone();
                            move || {
                                let attach = attach.clone();
                                spawn(async move {
                                    let answer = pick_paths(OpenDialog::files().multiple(true)).await;
                                    let paths: Option<Vec<PathBuf>> = answer.ok().flatten();
                                    let Some(path) = paths.and_then(|paths| paths.into_iter().next()) else {
                                        return;
                                    };
                                    match attach(path.clone()) {
                                        Ok(name) => {
                                            let reference = image_reference(
                                                path.file_stem().and_then(|s| s.to_str()).unwrap_or("image"),
                                                &name,
                                            );
                                            draft.update(|text| *text = format!("{text}\n{reference}"));
                                            attach_error.set(None);
                                        }
                                        Err(error) => attach_error.set(Some(error.to_string().into())),
                                    }
                                });
                            }
                        },
                    )
                })
                .child(dynamic(move || match attach_error.get() {
                    Some(reason) => text(reason)
                        .text_xs()
                        .text_color(move || tokens::destructive())
                        .into_element(),
                    None => view().into_element(),
                })),
        )
        .child(
            view()
                .flex_1()
                .gap_3()
                .child(dynamic(move || {
                    let root = attachments.clone();
                    let sides_of: Vec<String> = match &preview_template {
                        // A templated card's text is the template rendered from its fields.
                        Some(template) => {
                            let rendered = template_preview(&template.text, &fields.get(), template.deck.as_deref());
                            sides::split(&rendered).iter().map(|side| side.to_string()).collect()
                        }
                        None => sides::split(&draft.get()).iter().map(|side| side.to_string()).collect(),
                    };
                    let mut column = view().gap_3();
                    for side in sides_of {
                        let drawn = resolve_images(&preview_of(&side), &root);
                        column = column.child(prose(move || drawn.clone().into()));
                    }
                    column.into_element()
                }))
                .child(dynamic(move || match &template {
                    // The field table is the card's own only when a template is in force.
                    Some(template) => field_table(
                        template.fields.clone(),
                        move |name, value| {
                            fields.update(|rows| {
                                match rows.iter_mut().find(|(field, _)| field == name) {
                                    Some(row) => row.1 = value.to_owned(),
                                    None => rows.push((name.to_owned(), value.to_owned())),
                                }
                            });
                        },
                    )
                    .into_element(),
                    None => view().into_element(),
                }))
        )
        .child(leave_decision(asking, move |answer| match answer {
            Answer::Save => {
                let text = draft.get_untracked();
                if has_content(&text) {
                    saved.set(text.clone());
                    save_on_leave.clone()(text);
                }
                save_and_leave.clone()();
            }
            Answer::Discard => discard.clone()(),
        }))
}

/// What leaving a card with unsaved edits does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Leaving {
    /// Nothing to lose: leave.
    Leave,
    /// Ask first.
    Ask,
}

/// The user's answer when asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    Save,
    Discard,
}

/// Whether leaving needs the user's answer first.
pub fn leaving_choice(dirty: bool) -> Leaving {
    if dirty { Leaving::Ask } else { Leaving::Leave }
}

/// The question a dirty editor asks: save, discard, or stay. `open` false hides it; Stay closes it.
pub fn leave_decision(open: Signal<bool>, on_answer: impl Fn(Answer) + Clone + 'static) -> impl IntoElement {
    let save = on_answer.clone();
    let discard = on_answer;
    decision(open, move || {
        let stay = open;
        let save = save.clone();
        let discard = discard.clone();
        view()
            .gap_3()
            .child(text("Save this card before leaving?").text_sm().text_color(move || tokens::foreground()))
            .child(
                view()
                    .flex_row()
                    .gap_2()
                    .child(button(Variant::Primary, "Save and leave", || false, move || {
                        save(Answer::Save);
                        stay.set(false);
                    }))
                    .child(button(Variant::Default, "Discard", || false, move || {
                        discard(Answer::Discard);
                        stay.set(false);
                    }))
                    .child(button(Variant::Ghost, "Stay", || false, move || stay.set(false))),
            )
    })
}

/// One row of a card's field table: the name a template uses, the value typed for it, and whether
/// the app computes it (which makes it read-only).
#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub name: String,
    pub value: String,
    pub computed: bool,
}

/// The fields a card's template fills from, as the editor shows them: a value per name, computed
/// rows read-only, and `on_edit(name, value)` when the user types one.
pub fn field_table(fields: Vec<Field>, on_edit: impl Fn(&str, &str) + Clone + 'static) -> impl IntoElement {
    let mut column = view().gap_2().child(badge("Fields"));
    for row in fields {
        let name = row.name.clone();
        let edit = on_edit.clone();
        if row.computed {
            column = column.child(
                view()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(text(row.name).text_xs().text_color(move || tokens::muted_foreground()))
                    .child(text(row.value).text_sm().text_color(move || tokens::foreground()))
                    .child(badge("computed")),
            );
        } else {
            let value = signal(row.value);
            let edited = name.clone();
            column = column.child(
                view()
                    .gap_1()
                    .child(text(name).text_xs().text_color(move || tokens::muted_foreground()))
                    .child(field_with("Value", value, move |text| edit(&edited, text))),
            );
        }
    }
    column
}

/// A card's text as its template renders it, for the editor's preview of a templated card.
pub fn template_preview(template: &str, fields: &[(String, String)], deck: Option<&str>) -> String {
    typr_core::template::render_card(template, fields, deck)
}

/// The rectangle a drag across the picture covers: the pointer lands at the middle and moves by
/// `translation` points on a pane `size` points wide, and the corners are stored as fractions.
///
/// `None` when the drag covers less than one percent on either axis — the spec's floor — so a click
/// or a twitch adds nothing.
pub fn rect_from_drag(size: (f32, f32), translation: (f32, f32)) -> Option<typr_core::store::MaskRect> {
    let (pane_w, pane_h) = size;
    if pane_w <= 0.0 || pane_h <= 0.0 {
        return None;
    }
    let (dx, dy) = (translation.0 / pane_w, translation.1 / pane_h);
    let (x0, x1) = (0.5_f32.min(0.5 + dx), 0.5_f32.max(0.5 + dx));
    let (y0, y1) = (0.5_f32.min(0.5 + dy), 0.5_f32.max(0.5 + dy));
    let rect = typr_core::store::MaskRect::new(
        x0.clamp(0.0, 1.0),
        y0.clamp(0.0, 1.0),
        (x1 - x0).clamp(0.0, 1.0),
        (y1 - y0).clamp(0.0, 1.0),
    );
    if rect.w < 0.01 || rect.h < 0.01 || !rect.inside_the_image() {
        return None;
    }
    Some(rect)
}

/// Where a cover sits inside a pane `size` points wide, in points: `(left, top, width, height)`.
pub fn cover_box(size: (f32, f32), rect: typr_core::store::MaskRect) -> (f32, f32, f32, f32) {
    (rect.x * size.0, rect.y * size.1, rect.w * size.0, rect.h * size.1)
}
