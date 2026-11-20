//! The card editor: the markdown source on one side, its sides as previewed on the other.
//!
//! Saving is explicit. The screen reports the text through `on_save`; the caller writes it. A card
//! with no content is not saved: its save button is disabled.

use std::path::{Path, PathBuf};

use nsh_core::file_dialog::OpenDialog;
use nsh_core::prelude::*;

use nsh_core::element::{JsObject, Prop};
use typr_core::edits::has_content;
use typr_core::keymap::{Command, Surface};
use typr_core::sides;

use crate::screens::keymap::keystroke_of;

use crate::ui::{
    button::{Variant, button},
    prose::prose,
    source::{self, source_with},
    status::badge,
    tokens,
};

mod diagram;
mod dialogs;
mod fields;

pub use diagram::{Cover, Diagram, Rect, cover_box, diagram_pane, rect_from_corners};
pub use dialogs::{Answer, Leaving, clear_question, leave_decision, leaving_choice};
pub use fields::{TemplateField, field_table, template_preview};

/// The file name a chosen picture is copied to, which is also the reference the card holds. A name
/// already in use gets `-2`, `-3`, so a second picture never replaces the first.
///
/// A copy that cannot be made is refused rather than reported: a card keeps its previous picture
/// and the caller shows why.
pub fn attach_image(source: &Path, root: &Path) -> std::io::Result<String> {
    typr_core::transfer::attach(root, source)
}

/// An image destination as markdown reads it: a name with spaces or parentheses goes in `<...>`.
// shortcut: a name holding `<` or `>` is not escaped, when such names turn up.
fn destination_text(path: &str) -> String {
    if path.contains(|c: char| c.is_whitespace() || "()<>".contains(c)) {
        format!("<{path}>")
    } else {
        path.to_owned()
    }
}

/// The destination that opens `text` (what follows `](`) and how many bytes it takes through its
/// closing `)`: `<...>` as written, or bare with balanced parentheses, as CommonMark reads it.
fn destination(text: &str) -> Option<(&str, usize)> {
    if let Some(inner) = text.strip_prefix('<') {
        let end = inner.find('>')?;
        return inner[end + 1..]
            .starts_with(')')
            .then(|| (&inner[..end], end + 3));
    }
    let mut depth = 0;
    for (at, c) in text.char_indices() {
        match c {
            '(' => depth += 1,
            ')' if depth == 0 => return Some((&text[..at], at + 1)),
            ')' => depth -= 1,
            _ => {}
        }
    }
    None
}

/// The image reference written into the card: `![alt](name)`, so a card's text stays a markdown
/// image and the store copies the picture beside the library.
pub fn image_reference(alt: &str, name: &str) -> String {
    format!("![{alt}]({})", destination_text(name))
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
        let tail = &after[close + 2..];
        let Some((src, used)) = destination(tail) else {
            out.push_str(after);
            return out;
        };
        let inside = !src.trim().is_empty()
            && Path::new(src.trim()).is_absolute().eq(&false)
            && !Path::new(src.trim())
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir));
        out.push_str(&after[..close + 2]);
        if inside {
            out.push_str(&destination_text(
                &root.join(src.trim()).display().to_string(),
            ));
        } else {
            out.push_str(&tail[..used - 1]);
        }
        out.push(')');
        rest = &tail[used..];
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
        let tail = &after[close + 2..];
        let Some((src, used)) = destination(tail) else {
            break;
        };
        found.push(src.to_owned());
        rest = &tail[used..];
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

/// A template in force on the card: its text, the fields it fills from, and the deck's name.
#[derive(Clone, Debug, Default, PartialEq, JsObject)]
#[non_exhaustive]
pub struct Template {
    pub text: String,
    pub fields: Vec<TemplateField>,
    pub deck: Option<String>,
}

impl Template {
    /// A template whose text is `text`, with no fields filled and no deck named.
    pub fn new(text: impl Into<String>) -> Template {
        Template {
            text: text.into(),
            ..Template::default()
        }
    }
}

/// What the editor opens with: the card as the store has it.
#[derive(Clone, PartialEq, Props)]
pub struct EditorProps {
    /// The card's text as last saved.
    pub saved: Prop<SharedString>,
    /// The text typed since, when a draft was stored.
    pub draft: Prop<Option<SharedString>>,
    /// The template in force, when the card fills one.
    #[prop(default)]
    pub template: Prop<Option<Template>>,
    /// The diagram, when the card is one rather than text.
    #[prop(default)]
    pub diagram: Prop<Option<Diagram>>,
    /// The directory attached pictures are copied to.
    #[prop(default)]
    pub attachments: Prop<SharedString>,
    /// Why the last save was refused, shown under the Save button until the caller clears it.
    #[prop(default)]
    pub save_error: Prop<Option<SharedString>>,
}

/// The card's text, as the editor reports it.
#[derive(Clone, Debug, Default, PartialEq, JsObject)]
pub struct CardText {
    /// The whole text.
    pub text: String,
}

/// The fields of a templated card, as the user has them.
#[derive(Clone, Debug, Default, PartialEq, JsObject)]
pub struct FieldValues {
    pub fields: Vec<TemplateField>,
}

/// One rectangle dragged onto the picture, and the group it joins.
#[derive(Clone, Copy, Debug, Default, PartialEq, JsObject)]
pub struct MaskAdded {
    /// The cover the drag drew.
    pub mask: Cover,
}

/// One group's rectangles, to be replaced by the one that holds them all.
#[derive(Clone, Debug, Default, PartialEq, JsObject)]
pub struct GroupMerge {
    /// The group the rectangles belong to.
    pub group: u32,
    /// Every rectangle of that group.
    pub rects: Vec<Rect>,
}

/// One group of covers, to be let go.
#[derive(Clone, Copy, Debug, Default, PartialEq, JsObject)]
pub struct MaskGroup {
    /// The group's number.
    pub group: u32,
}

/// The picture a card becomes a diagram of.
#[derive(Clone, Debug, Default, PartialEq, JsObject)]
pub struct DiagramStart {
    /// The picture's name, already copied beside the library.
    pub name: String,
}

/// What the editor reports: the card's text, and every edit to what it draws. The caller writes
/// them down; the editor holds nothing of the store.
#[derive(Event)]
pub enum EditorEvent {
    /// The text, to write down as the card.
    Save(CardText),
    /// A templated card's fields, to write down so its text is rendered from them.
    SaveFields(FieldValues),
    /// The text as it is typed, to keep as the draft.
    Draft(CardText),
    /// Leaving, with nothing unsaved.
    Leave,
    /// Leaving without the draft: the in-progress text is thrown away.
    Discard,
    /// A rectangle dragged on the picture.
    AddMask(MaskAdded),
    /// One group's rectangles to replace with the one that holds them all.
    MergeMasks(GroupMerge),
    /// Every rectangle of a group let go.
    RemoveMaskGroup(MaskGroup),
    /// The picture and every rectangle let go; the card is text again.
    ClearDiagram,
    /// The card becomes a diagram of the picture copied under `name`.
    StartDiagram(DiagramStart),
}

/// The whole editor: the source or the diagram pane, the preview (from the template when one is in
/// force), the field table, the image action, and the Save/Discard/Stay question before leaving
/// unsaved work. Every change leaves as an [`EditorEvent`]; what no one listens for is dropped.
#[component]
pub fn Editor(props: EditorProps, cx: Scope<EditorEvent>) -> impl IntoElement {
    let saved_text = props.saved.get_untracked().to_string();
    let template = props.template.get_untracked();
    let diagram = props.diagram.get_untracked();
    let root: std::rc::Rc<PathBuf> =
        std::rc::Rc::new(PathBuf::from(props.attachments.get_untracked().to_string()));
    let attach_root = root.clone();
    let preview_root = root.clone();
    if let Some(diagram) = diagram {
        // A diagram clears back to a text card, which is the only way its picture is let go; the
        // question keeps one press from throwing the rectangles away.
        let asking_clear = signal(false);
        return view()
            .flex_1()
            .p_4()
            .gap_2()
            .child(diagram_pane(diagram, cx))
            .child(
                button()
                    .variant(Variant::Default)
                    .label("Clear the diagram…")
                    .disabled(|| false)
                    .on_press(move || asking_clear.set(true)),
            )
            .child(clear_question(asking_clear, move || {
                cx.emit(EditorEvent::ClearDiagram);
            }))
            .into_element();
    }
    let source_handle = source::source_handle();
    let asking = signal(false);
    // Discarding is the one answer that lets the stored draft go.
    let draft = signal(
        props
            .draft
            .get_untracked()
            .map(|text| text.to_string())
            .unwrap_or_else(|| saved_text.clone()),
    );
    let saved = signal(saved_text);
    let preview_template = template.clone();
    let table_template = template;
    let fields = signal(
        table_template
            .as_ref()
            .map(|template| {
                template
                    .fields
                    .iter()
                    .filter(|field| !field.computed)
                    .map(|field| (field.name.clone(), field.value.clone()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
    );
    let attach_error = signal(None::<SharedString>);
    let save_error = props.save_error;
    let templated = table_template.is_some();
    let opened_fields = signal(fields.get_untracked());
    // Leaving asks first only when the draft or a field differs from what was saved.
    let dirty = move || {
        draft.with(|d| saved.with(|s| d != s))
            || fields.with(|now| opened_fields.with(|opened| now != opened))
    };
    // A templated card's text is rendered from its fields, so its fields are what Save writes.
    let emit_save = move || {
        if templated {
            let fields = fields
                .get_untracked()
                .into_iter()
                .map(|(name, value)| TemplateField::new(name, value))
                .collect();
            cx.emit(EditorEvent::SaveFields(FieldValues { fields }));
        } else {
            let text = draft.get_untracked();
            cx.emit(EditorEvent::Save(CardText { text }));
        }
    };
    if let Some(key) = keystroke_of(Surface::Editor, Command::Leave) {
        on_key(&key, move |_| {
            if dirty() {
                asking.set(true);
            } else {
                cx.emit(EditorEvent::Leave);
            }
        });
    }
    let state_badge = dynamic(move || {
        badge()
            .label(if dirty() { "Unsaved" } else { "Saved" })
            .into_element()
    });
    view()
        .flex_1()
        .gap_3()
        .p_4()
        // The header sits above both columns, so nothing the columns draw can cover the way back.
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(state_badge)
                .child({
                    // The way back, reachable without a key: it asks what the key asks.
                    let asking_again = asking;
                    button()
                        .variant(Variant::Ghost)
                        .label("Back")
                        .disabled(|| false)
                        .on_press(move || {
                            if dirty() {
                                asking_again.set(true);
                                return;
                            }
                            cx.emit(EditorEvent::Leave);
                        })
                        .into_element()
                })
                .child(dynamic(move || {
                    let count = prompt_count(&draft.get());
                    let label = match count {
                        0 => "note".to_owned(),
                        1 => "1 prompt".to_owned(),
                        n => format!("{n} prompts"),
                    };
                    badge().label(label).into_element()
                })),
        )
        .child(
            view()
                .flex_row()
                .flex_1()
                .gap_3()
                .child(
                    view()
                        .flex_1()
                        .gap_2()
                        .child(source_with(
                            draft,
                            move |text: &str| {
                                cx.emit(EditorEvent::Draft(CardText {
                                    text: text.to_owned(),
                                }))
                            },
                            Some(&source_handle),
                        ))
                        .child(
                            button()
                                .variant(Variant::Primary)
                                .label("Save")
                                .disabled(move || draft.with(|d| !has_content(d)))
                                .on_press(emit_save),
                        )
                        .child(dynamic({
                            let drawn_root = root.clone();
                            move || {
                                if has_content(&draft.get()) {
                                    return view().into_element();
                                }
                                let pressed_root = drawn_root.clone();
                                button()
                                    .variant(Variant::Default)
                                    .label("Make it a diagram…")
                                    .disabled(|| false)
                                    .on_press(move || {
                                        let root = pressed_root.clone();
                                        spawn(async move {
                                            let answer =
                                                pick_paths(OpenDialog::files().multiple(false))
                                                    .await;
                                            let paths: Option<Vec<PathBuf>> = answer.ok().flatten();
                                            let Some(path) =
                                                paths.and_then(|paths| paths.into_iter().next())
                                            else {
                                                return;
                                            };
                                            if let Ok(name) = attach_image(&path, root.as_path()) {
                                                cx.emit(EditorEvent::StartDiagram(DiagramStart {
                                                    name,
                                                }));
                                            }
                                        });
                                    })
                                    .into_element()
                            }
                        }))
                        .child(
                            button()
                                .variant(Variant::Default)
                                .label("Attach image…")
                                .disabled(|| false)
                                .on_press(move || {
                                    let root = attach_root.clone();
                                    spawn(async move {
                                        let answer =
                                            pick_paths(OpenDialog::files().multiple(true)).await;
                                        let paths: Option<Vec<PathBuf>> = answer.ok().flatten();
                                        let Some(path) =
                                            paths.and_then(|paths| paths.into_iter().next())
                                        else {
                                            return;
                                        };
                                        match attach_image(&path, root.as_path()) {
                                            Ok(name) => {
                                                let reference = image_reference(
                                                    path.file_stem()
                                                        .and_then(|stem| stem.to_str())
                                                        .unwrap_or("image"),
                                                    &name,
                                                );
                                                if !source::insert_at_caret(
                                                    &source_handle,
                                                    &reference,
                                                ) {
                                                    draft.update(|text| {
                                                        *text = format!("{text}\n{reference}")
                                                    });
                                                    let text = draft.get_untracked();
                                                    cx.emit(EditorEvent::Draft(CardText { text }));
                                                }
                                                attach_error.set(None);
                                            }
                                            Err(error) => {
                                                attach_error.set(Some(error.to_string().into()))
                                            }
                                        }
                                    });
                                }),
                        )
                        .child(dynamic(move || match save_error.get() {
                            Some(reason) => text(reason)
                                .text_xs()
                                .text_color(tokens::destructive)
                                .into_element(),
                            None => view().into_element(),
                        }))
                        .child(dynamic(move || match attach_error.get() {
                            Some(reason) => text(reason)
                                .text_xs()
                                .text_color(tokens::destructive)
                                .into_element(),
                            None => view().into_element(),
                        })),
                )
                .child(
                    view()
                        .flex_1()
                        .gap_3()
                        .child(dynamic(move || {
                            let root = preview_root.clone();
                            let sides_of: Vec<String> = match &preview_template {
                                // A templated card's text is the template rendered from its fields.
                                Some(template) => {
                                    let rendered = template_preview(
                                        &template.text,
                                        &fields.get(),
                                        template.deck.as_deref(),
                                    );
                                    sides::split(&rendered)
                                        .iter()
                                        .map(|side| side.to_string())
                                        .collect()
                                }
                                None => sides::split(&draft.get())
                                    .iter()
                                    .map(|side| side.to_string())
                                    .collect(),
                            };
                            let mut column = view().gap_3();
                            for side in sides_of {
                                let drawn = resolve_images(&preview_of(&side), root.as_path());
                                column = column.child(
                                    prose().source(move || SharedString::from(drawn.clone())),
                                );
                            }
                            column.into_element()
                        }))
                        .child(dynamic(move || match &table_template {
                            // The field table is the card's own only when a template is in force.
                            Some(template) => {
                                field_table(template.fields.clone(), move |name, value| {
                                    fields.update(|rows| {
                                        match rows.iter_mut().find(|(field, _)| field == name) {
                                            Some(row) => row.1 = value.to_owned(),
                                            None => rows.push((name.to_owned(), value.to_owned())),
                                        }
                                    });
                                })
                                .into_element()
                            }
                            None => view().into_element(),
                        })),
                ),
        )
        .child(leave_decision(asking, move |answer| match answer {
            Answer::Save => {
                if has_content(&draft.get_untracked()) {
                    emit_save();
                }
                cx.emit(EditorEvent::Leave);
            }
            Answer::Discard => {
                cx.emit(EditorEvent::Discard);
            }
        }))
        .into_element()
}
