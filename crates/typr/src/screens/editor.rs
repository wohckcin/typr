//! The card editor: the markdown source on one side, its sides as previewed on the other.
//!
//! Saving is explicit. The screen reports the text through `on_save`; the caller writes it. A card
//! with no content is not saved: its save button is disabled.

use std::path::{Path, PathBuf};

use nsh_core::prelude::*;
use nsh_core::{TouchEvent, TouchPhase};
use nsh_core::file_dialog::OpenDialog;
use std::rc::Rc;

use typr_core::keymap::{Command, Surface};
use typr_core::store::{MaskRect, MaskRow};
use typr_core::sides;

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

/// A diagram card's picture, its caption, and the rectangles drawn on it.
#[derive(Clone, Debug, PartialEq)]
pub struct Diagram {
    pub image: String,
    pub caption: String,
    pub rows: Vec<MaskRow>,
}


/// The diagram pane: the picture, a cover per rectangle labelled with its group, a drag across it
/// that adds a rectangle to the last group, and actions that merge the picked group into one
/// rectangle or remove the picked rectangle.
pub fn diagram_pane(diagram: Diagram, actions: Actions) -> impl IntoElement {
    let Diagram { image, caption, rows } = diagram;
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
    let add = actions.add_mask.clone();
    let merge = actions.merge_masks.clone();
    let remove_group = actions.remove_group.clone();
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
                    let (Some(started), Some(ended), TouchPhase::End) = (started, ended, &event.phase)
                    else {
                        return;
                    };
                    let Some(rect) = rect_from_corners((width, height), started, ended) else {
                        return;
                    };
                    if let Some(add) = &add {
                        add(group, rect);
                    }
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
                .child(text("diagram").text_xs().text_color(move || tokens::muted_foreground()))
                .child(prose(move || format!("![]({image})").into()))
                .child(dynamic(move || {
                    let dims = size.get();
                    let selected = picked.get();
                    let mut layer = view().absolute().inset(tokens::flush());
                    for (index, row) in layer_rows.clone().into_iter().enumerate() {
                        let (left, top, width, height) = cover_box(dims, row.rect);
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
                                .border_color(move || tokens::selection())
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
                .child(button(Variant::Default, "Merge group", || false, move || {
                    let (Some(index), Some(merge)) = (picked.get_untracked(), &merge) else {
                        return;
                    };
                    let group = merge_rows[index].group;
                    let rects: Vec<MaskRect> = merge_rows
                        .iter()
                        .filter(|row| row.group == group)
                        .map(|row| row.rect)
                        .collect();
                    merge(group, rects);
                }))
                .child(button(Variant::Default, "Remove group", || false, move || {
                    let (Some(index), Some(remove)) = (picked.get_untracked(), &remove_group) else {
                        return;
                    };
                    remove(remove_rows[index].group);
                })),
        )
}

/// The card the editor shows.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Card {
    pub saved: String,
    pub draft: Option<String>,
    pub template: Option<Template>,
    pub diagram: Option<Diagram>,
}

impl Card {
    pub fn new(saved: impl Into<String>) -> Card {
        Card { saved: saved.into(), ..Card::default() }
    }
}

/// What the editor reports while it is open: one boxed closure per action, so a screen that mounts
/// it passes one value and the shell supplies the store behind it.
#[derive(Clone, Default)]
pub struct Actions {
    pub save: Option<Rc<dyn Fn(String)>>,
    pub draft: Option<Rc<dyn Fn(&str)>>,
    pub leave: Option<Rc<dyn Fn()>>,
    /// Leaving without the draft: the in-progress text is thrown away.
    pub discard: Option<Rc<dyn Fn()>>,
    pub attach: Option<Rc<dyn Fn(PathBuf) -> std::io::Result<String>>>,
    pub add_mask: Option<Rc<dyn Fn(u32, MaskRect)>>,
    pub merge_masks: Option<Rc<dyn Fn(u32, Vec<MaskRect>)>>,
    pub remove_mask: Option<Rc<dyn Fn(MaskRect)>>,
    pub remove_group: Option<Rc<dyn Fn(u32)>>,
    pub clear_diagram: Option<Rc<dyn Fn()>>,
    pub switch_to_diagram: Option<Rc<dyn Fn(String)>>,
}

impl Actions {
    pub fn new() -> Actions {
        Actions::default()
    }

    pub fn save(self, f: impl Fn(String) + 'static) -> Actions {
        Actions { save: Some(Rc::new(f)), ..self }
    }

    pub fn draft(self, f: impl Fn(&str) + 'static) -> Actions {
        Actions { draft: Some(Rc::new(f)), ..self }
    }

    pub fn leave(self, f: impl Fn() + 'static) -> Actions {
        Actions { leave: Some(Rc::new(f)), ..self }
    }

    pub fn discard(self, f: impl Fn() + 'static) -> Actions {
        Actions { discard: Some(Rc::new(f)), ..self }
    }

    pub fn attach(self, f: impl Fn(PathBuf) -> std::io::Result<String> + 'static) -> Actions {
        Actions { attach: Some(Rc::new(f)), ..self }
    }

    pub fn masks(
        self,
        add: impl Fn(u32, MaskRect) + 'static,
        merge: impl Fn(u32, Vec<MaskRect>) + 'static,
        remove: impl Fn(MaskRect) + 'static,
    ) -> Actions {
        Actions {
            add_mask: Some(Rc::new(add)),
            merge_masks: Some(Rc::new(merge)),
            remove_mask: Some(Rc::new(remove)),
            ..self
        }
    }

    /// Removing a whole group, and switching a card between text and diagram.
    pub fn kinds(
        self,
        remove_group: impl Fn(u32) + 'static,
        clear_diagram: impl Fn() + 'static,
        switch_to_diagram: impl Fn(String) + 'static,
    ) -> Actions {
        Actions {
            remove_group: Some(Rc::new(remove_group)),
            clear_diagram: Some(Rc::new(clear_diagram)),
            switch_to_diagram: Some(Rc::new(switch_to_diagram)),
            ..self
        }
    }
}

/// The card editor over `initial`, saving through `on_save` and copying attached pictures through
/// `on_attach`.
pub fn editor(
    initial: String,
    on_save: impl Fn(String) + Clone + 'static,
    on_attach: impl Fn(PathBuf) -> std::io::Result<String> + Clone + 'static,
) -> impl IntoElement {
    let save = on_save.clone();
    editor_card(
        Card::new(initial),
        PathBuf::new(),
        Actions::new()
            .save(move |text| save.clone()(text))
            .attach(move |path| on_attach.clone()(path)),
    )
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
    let save = on_save.clone();
    editor_card(
        Card { saved: saved_text, draft: draft_text, ..Card::default() },
        attachments,
        Actions::new()
            .save(move |text| save.clone()(text))
            .attach(move |path| on_attach.clone()(path))
            .draft(on_draft),
    )
}

/// A template in force on the card: its text, the fields it fills from, and the deck's name.
#[derive(Clone, Debug, PartialEq)]
pub struct Template {
    pub text: String,
    pub fields: Vec<Field>,
    pub deck: Option<String>,
}

/// The whole editor: the source or the diagram pane, the preview (from the template when one is in
/// force), the field table, the image action, and the Save/Discard/Stay question before leaving
/// unsaved work. Every action goes to `actions`; an omitted one does nothing.
pub fn editor_card(card: Card, attachments: PathBuf, actions: Actions) -> AnyElement {
    let Card { saved: saved_text, draft: draft_text, template, diagram } = card;
    let on_save = actions.save.clone();
    let on_leave = actions.leave.clone();
    let on_attach = actions.attach.clone();
    let on_draft = actions.draft.clone();
    let masks = actions.clone();
    if let Some(diagram) = diagram {
        // A diagram clears back to a text card, which is the only way its picture is let go; the
        // question keeps one press from throwing the rectangles away.
        let asking_clear = signal(false);
        let clear = actions.clear_diagram.clone();
        let asked = asking_clear;
        return view()
            .flex_1()
            .p_4()
            .gap_2()
            .child(diagram_pane(diagram, masks))
            .child(button(Variant::Default, "Clear the diagram", || false, move || asked.set(true)))
            .child(clear_question(asking_clear, move || {
                if let Some(clear) = &clear {
                    clear();
                }
            }))
            .into_element();
    }
    let source_handle = source::source_handle();
    let asking = signal(false);
    let save_and_leave = on_leave.clone();
    // Discarding is the one answer that lets the stored draft go.
    let discard = actions.discard.clone().or_else(|| on_leave.clone());
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
    let report_draft = on_draft.clone();
    let save_on_leave = on_save.clone();
    let attach_fn = on_attach.clone();
    // Leaving asks first only when the draft differs from what was saved.
    let leave_now = on_leave.clone();
    let dirty = move || draft.with(|d| saved.with(|s| d != s));
    if let Some(key) = keystroke_of(Surface::Editor, Command::Leave) {
        on_key(&key, move |_| {
            if dirty() {
                asking.set(true);
            } else if let Some(leave) = &leave_now {
                leave();
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
                            let leaving = on_leave.clone();
                            let asking_again = asking;
                            let saved_now = saved;
                            let draft_now = draft;
                            button(Variant::Ghost, "Back", || false, move || {
                                if draft_now.with(|d| saved_now.with(|s| d != s)) {
                                    asking_again.set(true);
                                    return;
                                }
                                if let Some(leave) = &leaving {
                                    leave();
                                }
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
                    move |text: &str| {
                        if let Some(report) = &report_draft {
                            report(text);
                        }
                    },
                    Some(&source_handle),
                ))
                .child(button(
                    Variant::Primary,
                    "Save",
                    move || draft.with(|d| !has_content(d)),
                    move || {
                        let text = draft.get_untracked();
                        saved.set(text.clone());
                        if let Some(save) = &save_text {
                            save(text);
                        }
                    },
                ))
                .child(dynamic({
                    let switch = actions.switch_to_diagram.clone();
                    let attach = attach_fn.clone();
                    move || {
                        let (Some(switch), Some(attach)) = (switch.clone(), attach.clone()) else {
                            return view().into_element();
                        };
                        if has_content(&draft.get()) {
                            return view().into_element();
                        }
                        button(Variant::Default, "Make it a diagram", || false, move || {
                            let switch = switch.clone();
                            let attach = attach.clone();
                            spawn(async move {
                                let answer = pick_paths(OpenDialog::files().multiple(false)).await;
                                let paths: Option<Vec<PathBuf>> = answer.ok().flatten();
                                let Some(path) = paths.and_then(|paths| paths.into_iter().next()) else {
                                    return;
                                };
                                if let Ok(name) = attach(path) {
                                    switch(name);
                                }
                            });
                        })
                        .into_element()
                    }
                }))
                .child({
                    let attach = attach_fn.clone();
                    button(
                        Variant::Default,
                        "Attach image",
                        || false,
                        move || {
                            let attach = attach.clone();
                            spawn(async move {
                                let Some(attach) = attach else { return };
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
                                            if !source::insert_at_caret(&source_handle, &reference) {
                                                draft.update(|text| *text = format!("{text}\n{reference}"));
                                            }
                                            attach_error.set(None);
                                        }
                                        Err(error) => attach_error.set(Some(error.to_string().into())),
                                    }
                                });
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
        )
        .child(leave_decision(asking, move |answer| match answer {
            Answer::Save => {
                let text = draft.get_untracked();
                if has_content(&text) {
                    saved.set(text.clone());
                    if let Some(save) = &save_on_leave {
                        save(text);
                    }
                }
                if let Some(leave) = &save_and_leave {
                    leave();
                }
            }
            Answer::Discard => {
                if let Some(leave) = &discard {
                    leave();
                }
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
pub fn clear_question(open: Signal<bool>, on_clear: impl Fn() + Clone + 'static) -> impl IntoElement {
    let keep = open;
    let clear = on_clear;
    let asked = keep;
    let confirmed = clear;
    decision(open, move || {
        let asked = asked;
        let confirmed = confirmed.clone();
        view()
            .gap_3()
            .child(text("Clear this diagram?").text_sm().text_color(move || tokens::foreground()))
            .child(
                text("Its picture and every rectangle go; it is edited as text afterwards.")
                    .text_xs()
                    .text_color(move || tokens::muted_foreground()),
            )
            .child(
                view()
                    .flex_row()
                    .gap_2()
                    .child(button(Variant::Destructive, "Clear", || false, move || {
                        confirmed.clone()();
                        asked.set(false);
                    }))
                    .child(button(Variant::Default, "Stay", || false, move || asked.set(false))),
            )
    })
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
    (rect.x * size.0, rect.y * size.1, rect.w * size.0, rect.h * size.1)
}
