//! The card editor: the markdown source on one side, its sides as previewed on the other.
//!
//! Saving is explicit. The screen reports the text through `on_save`; the caller writes it. A card
//! with no content is not saved: its save button is disabled.

use std::path::{Path, PathBuf};

use nsh_core::file_dialog::OpenDialog;
use nsh_core::prelude::*;
use nsh_core::{TouchEvent, TouchPhase};

use nsh_core::element::{JsObject, Prop};
use typr_core::keymap::{Command, Surface};
use typr_core::sides;
use typr_core::store::{MaskRect, MaskRow};

use crate::screens::keymap::keystroke_of;

use crate::ui::{
    button::{Variant, button},
    input::field_with,
    overlay::decision,
    prose::prose,
    source::{self, source_with},
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
        .ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "the file has no name")
        })?;
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
        let Some(end) = after[close + 2..].find(')') else {
            break;
        };
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

/// One cover on a diagram's picture: the group it belongs to, and its rectangle as fractions of
/// the picture.
#[derive(Clone, Copy, Debug, Default, PartialEq, JsObject)]
pub struct Cover {
    pub group: u32,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Cover {
    /// A store row as the pane draws it.
    pub fn of(row: &MaskRow) -> Cover {
        Cover {
            group: row.group,
            x: row.rect.x,
            y: row.rect.y,
            w: row.rect.w,
            h: row.rect.h,
        }
    }

    /// The rectangle the store holds.
    fn rect(&self) -> MaskRect {
        Rect {
            x: self.x,
            y: self.y,
            w: self.w,
            h: self.h,
        }
        .rect()
    }
}

/// A diagram card's picture, its caption, and the rectangles drawn on it.
#[derive(Clone, Debug, Default, PartialEq, JsObject)]
pub struct Diagram {
    pub image: String,
    pub caption: String,
    pub rows: Vec<Cover>,
}

/// The diagram pane: the picture, a cover per rectangle labelled with its group, a drag across it
/// that adds a rectangle to the last group, and actions that merge the picked group into one
/// rectangle or remove the picked group.
pub fn diagram_pane(diagram: Diagram, cx: Scope<EditorEvent>) -> impl IntoElement {
    let Diagram {
        image,
        caption,
        rows,
    } = diagram;
    // How large the picture is drawn, and where a finger went down on it.
    let size = signal((300.0_f32, 200.0_f32));
    let from = signal(None::<(f32, f32)>);
    let to = signal(None::<(f32, f32)>);
    let picked = signal(None::<usize>);
    // The groups in the order they first appear, so the strip is stable as rectangles are added.
    let mut groups: Vec<u32> = Vec::new();
    for row in &rows {
        if !groups.contains(&row.group) {
            groups.push(row.group);
        }
    }
    // A drag joins the last group, so repeated drags build one answer.
    let group = groups.last().copied().unwrap_or(1);
    let layer_rows = rows.clone();
    let merge_rows = rows.clone();
    let remove_rows = rows;
    let mut strip = view().flex_row().gap_2();
    for (index, row) in layer_rows.clone().into_iter().enumerate() {
        strip = strip.child(button(
            Variant::Default,
            format!("group {}", row.group),
            || false,
            move || picked.set(Some(index)),
        ));
    }
    view()
        .flex_1()
        .gap_2()
        .claim_touches(TouchClaim::on_move())
        // A drag across the picture runs from where the finger went down to where it lifted.
        .on_touch(move |event: &TouchEvent| {
            let (width, height) = size.get_untracked();
            // The picture takes the touches itself (`PointerEvents::BoxOnly`), so a finger's
            // position is already relative to it.
            let position = event
                .points
                .first()
                .map(|point| (point.location.x.0, point.location.y.0));
            match event.phase {
                TouchPhase::Start => {
                    if let Some(at) = position {
                        from.set(Some(at));
                        to.set(Some(at));
                    }
                }
                TouchPhase::Move => {
                    if let Some(at) = position {
                        to.set(Some(at));
                    }
                }
                TouchPhase::End | TouchPhase::Cancel => {
                    let started = from.get_untracked();
                    let ended = position.or_else(|| to.get_untracked());
                    from.set(None);
                    to.set(None);
                    let (Some(started), Some(ended), TouchPhase::End) =
                        (started, ended, &event.phase)
                    else {
                        return;
                    };
                    let Some(rect) = rect_from_corners((width, height), started, ended) else {
                        return;
                    };
                    cx.emit(EditorEvent::AddMask(Cover {
                        group,
                        x: rect.x,
                        y: rect.y,
                        w: rect.w,
                        h: rect.h,
                    }));
                }
            }
        })
        .child(
            view()
                .flex_1()
                .overflow_hidden()
                .accessible(true)
                .role(AccessibilityRole::Image)
                .label("the picture")
                .pointer_events(PointerEvents::BoxOnly)
                .on_layout(move |bounds: &Bounds<Points>| {
                    size.set((bounds.size.width.0, bounds.size.height.0));
                })
                .child(
                    text("diagram")
                        .text_xs()
                        .text_color(tokens::muted_foreground),
                )
                .child(prose(move || format!("![]({image})").into()))
                .child(dynamic(move || {
                    let dims = size.get();
                    let selected = picked.get();
                    let mut layer = view().absolute().inset(tokens::flush());
                    for (index, row) in layer_rows.clone().into_iter().enumerate() {
                        let (left, top, width, height) = cover_box(dims, row.rect());
                        let fill = move || match selected == Some(index) {
                            true => tokens::selection(),
                            false => tokens::selection().opacity(0.25),
                        };
                        layer = layer.child(
                            view()
                                .absolute()
                                .left(px(left))
                                .top(px(top))
                                .w(px(width))
                                .h(px(height))
                                .bg(fill)
                                .border_1()
                                .border_color(tokens::selection)
                                .accessible(true)
                                .label(format!("rectangle in group {}", row.group))
                                .child(text(format!("group {}", row.group)).text_xs()),
                        );
                    }
                    layer.into_element()
                })),
        )
        .child(text(caption).text_sm())
        .child(strip)
        .child(
            view()
                .flex_row()
                .gap_2()
                .child(button(
                    Variant::Default,
                    "Merge group",
                    || false,
                    move || {
                        let Some(index) = picked.get_untracked() else {
                            return;
                        };
                        let group = merge_rows[index].group;
                        let rects: Vec<Rect> = merge_rows
                            .iter()
                            .filter(|row| row.group == group)
                            .map(|row| Rect::of(row.rect()))
                            .collect();
                        cx.emit(EditorEvent::MergeMasks(MergedMasks { group, rects }));
                    },
                ))
                .child(button(
                    Variant::Default,
                    "Remove group",
                    || false,
                    move || {
                        let Some(index) = picked.get_untracked() else {
                            return;
                        };
                        cx.emit(EditorEvent::RemoveMaskGroup(remove_rows[index].group));
                    },
                )),
        )
}

/// A template in force on the card: its text, the fields it fills from, and the deck's name.
#[derive(Clone, Debug, Default, PartialEq, JsObject)]
pub struct Template {
    pub text: String,
    pub fields: Vec<Field>,
    pub deck: Option<String>,
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
}

/// A change to a diagram's covers.
/// A rectangle as fractions of the picture.
#[derive(Clone, Copy, Debug, Default, PartialEq, JsObject)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    fn of(rect: MaskRect) -> Rect {
        Rect {
            x: rect.x,
            y: rect.y,
            w: rect.w,
            h: rect.h,
        }
    }

    fn rect(self) -> MaskRect {
        MaskRect::new(self.x, self.y, self.w, self.h)
    }
}

/// One group's rectangles merged into one that holds them all.
#[derive(Clone, Debug, Default, PartialEq, JsObject)]
pub struct MergedMasks {
    pub group: u32,
    pub rects: Vec<Rect>,
}

/// What the editor reports: the card's text, and every edit to what it draws. The caller writes
/// them down; the editor holds nothing of the store.
#[derive(Event)]
pub enum EditorEvent {
    /// The text, to write down as the card.
    Save(String),
    /// The text as it is typed, to keep as the draft.
    Draft(String),
    /// Leaving, with nothing unsaved.
    Leave,
    /// Leaving without the draft: the in-progress text is thrown away.
    Discard,
    /// A rectangle dragged on the picture.
    AddMask(Cover),
    /// One group's rectangles to replace with the one that holds them all.
    MergeMasks(MergedMasks),
    /// Every rectangle of a group let go.
    RemoveMaskGroup(u32),
    /// The picture and every rectangle let go; the card is text again.
    ClearDiagram,
    /// The card becomes a diagram of the picture copied under `name`.
    DiagramFromImage(String),
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
            .child(button(
                Variant::Default,
                "Clear the diagram",
                || false,
                move || asking_clear.set(true),
            ))
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
    // Leaving asks first only when the draft differs from what was saved.
    let dirty = move || draft.with(|d| saved.with(|s| d != s));
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
        let dirty = draft.with(|d| saved.with(|s| d != s));
        badge(if dirty { "Unsaved" } else { "Saved" }).into_element()
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
                    let saved_now = saved;
                    let draft_now = draft;
                    button(
                        Variant::Ghost,
                        "Back",
                        || false,
                        move || {
                            if draft_now.with(|d| saved_now.with(|s| d != s)) {
                                asking_again.set(true);
                                return;
                            }
                            cx.emit(EditorEvent::Leave);
                        },
                    )
                    .into_element()
                })
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
                            move |text: &str| cx.emit(EditorEvent::Draft(text.to_owned())),
                            Some(&source_handle),
                        ))
                        .child(button(
                            Variant::Primary,
                            "Save",
                            move || draft.with(|d| !has_content(d)),
                            move || {
                                let text = draft.get_untracked();
                                saved.set(text.clone());
                                cx.emit(EditorEvent::Save(text));
                            },
                        ))
                        .child(dynamic({
                            let drawn_root = root.clone();
                            move || {
                                if has_content(&draft.get()) {
                                    return view().into_element();
                                }
                                let pressed_root = drawn_root.clone();
                                button(
                                    Variant::Default,
                                    "Make it a diagram",
                                    || false,
                                    move || {
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
                                                cx.emit(EditorEvent::DiagramFromImage(name));
                                            }
                                        });
                                    },
                                )
                                .into_element()
                            }
                        }))
                        .child(button(
                            Variant::Default,
                            "Attach image",
                            || false,
                            move || {
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
                                            if !source::insert_at_caret(&source_handle, &reference)
                                            {
                                                draft.update(|text| {
                                                    *text = format!("{text}\n{reference}")
                                                });
                                                let text = draft.get_untracked();
                                                cx.emit(EditorEvent::Draft(text));
                                            }
                                            attach_error.set(None);
                                        }
                                        Err(error) => {
                                            attach_error.set(Some(error.to_string().into()))
                                        }
                                    }
                                });
                            },
                        ))
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
                                column = column.child(prose(move || drawn.clone().into()));
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
                let text = draft.get_untracked();
                if has_content(&text) {
                    saved.set(text.clone());
                    cx.emit(EditorEvent::Save(text));
                }
                cx.emit(EditorEvent::Leave);
            }
            Answer::Discard => {
                cx.emit(EditorEvent::Discard);
            }
        }))
        .into_element()
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

/// The question before a diagram is cleared: every rectangle is thrown away and the card becomes
/// empty text.
pub fn clear_question(
    open: Signal<bool>,
    on_clear: impl Fn() + Clone + 'static,
) -> impl IntoElement {
    let keep = open;
    let clear = on_clear;
    let asked = keep;
    let confirmed = clear;
    decision(open, move || {
        let asked = asked;
        let confirmed = confirmed.clone();
        view()
            .gap_3()
            .child(
                text("Clear this diagram?")
                    .text_sm()
                    .text_color(tokens::foreground),
            )
            .child(
                text("Its picture and every rectangle go; it is edited as text afterwards.")
                    .text_xs()
                    .text_color(tokens::muted_foreground),
            )
            .child(
                view()
                    .flex_row()
                    .gap_2()
                    .child(button(
                        Variant::Destructive,
                        "Clear",
                        || false,
                        move || {
                            confirmed.clone()();
                            asked.set(false);
                        },
                    ))
                    .child(button(
                        Variant::Default,
                        "Stay",
                        || false,
                        move || asked.set(false),
                    )),
            )
    })
}

/// The question a dirty editor asks: save, discard, or stay. `open` false hides it; Stay closes it.
pub fn leave_decision(
    open: Signal<bool>,
    on_answer: impl Fn(Answer) + Clone + 'static,
) -> impl IntoElement {
    let save = on_answer.clone();
    let discard = on_answer;
    decision(open, move || {
        let stay = open;
        let save = save.clone();
        let discard = discard.clone();
        view()
            .gap_3()
            .child(
                text("Save this card before leaving?")
                    .text_sm()
                    .text_color(tokens::foreground),
            )
            .child(
                view()
                    .flex_row()
                    .gap_2()
                    .child(button(
                        Variant::Primary,
                        "Save and leave",
                        || false,
                        move || {
                            save(Answer::Save);
                            stay.set(false);
                        },
                    ))
                    .child(button(
                        Variant::Default,
                        "Discard",
                        || false,
                        move || {
                            discard(Answer::Discard);
                            stay.set(false);
                        },
                    ))
                    .child(button(
                        Variant::Ghost,
                        "Stay",
                        || false,
                        move || stay.set(false),
                    )),
            )
    })
}

/// One row of a card's field table: the name a template uses, the value typed for it, and whether
/// the app computes it (which makes it read-only).
#[derive(Clone, Debug, Default, PartialEq, JsObject)]
pub struct Field {
    pub name: String,
    pub value: String,
    pub computed: bool,
}

/// The fields a card's template fills from, as the editor shows them: a value per name, computed
/// rows read-only, and `on_edit(name, value)` when the user types one.
pub fn field_table(
    fields: Vec<Field>,
    on_edit: impl Fn(&str, &str) + Clone + 'static,
) -> impl IntoElement {
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
                    .child(
                        text(row.name)
                            .text_xs()
                            .text_color(tokens::muted_foreground),
                    )
                    .child(text(row.value).text_sm().text_color(tokens::foreground))
                    .child(badge("computed")),
            );
        } else {
            let value = signal(row.value);
            let edited = name.clone();
            column = column.child(
                view()
                    .gap_1()
                    .child(text(name).text_xs().text_color(tokens::muted_foreground))
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

/// The rectangle between two points on the picture, as fractions of the pane `size` points wide.
///
/// `None` when either side is under one percent — the spec's floor — so a click or a twitch adds
/// nothing, and when the pane has no size yet.
pub fn rect_from_corners(size: (f32, f32), from: (f32, f32), to: (f32, f32)) -> Option<MaskRect> {
    let (pane_w, pane_h) = size;
    if pane_w <= 0.0 || pane_h <= 0.0 {
        return None;
    }
    let fraction = |value: f32, span: f32| (value / span).clamp(0.0, 1.0);
    let x0 = fraction(from.0.min(to.0), pane_w);
    let x1 = fraction(from.0.max(to.0), pane_w);
    let y0 = fraction(from.1.min(to.1), pane_h);
    let y1 = fraction(from.1.max(to.1), pane_h);
    // The far edge is clamped twice over: a drag that runs off the picture ends at its edge, and
    // the sum stays within one without a float-error overshoot the store would refuse.
    let rect = MaskRect::new(x0, y0, (x1 - x0).min(1.0 - x0), (y1 - y0).min(1.0 - y0));
    if rect.w < 0.01 || rect.h < 0.01 || !rect.inside_the_image() {
        return None;
    }
    Some(rect)
}

/// Where a cover sits inside a pane `size` points wide, in points: `(left, top, width, height)`.
pub fn cover_box(size: (f32, f32), rect: typr_core::store::MaskRect) -> (f32, f32, f32, f32) {
    (
        rect.x * size.0,
        rect.y * size.1,
        rect.w * size.0,
        rect.h * size.1,
    )
}
