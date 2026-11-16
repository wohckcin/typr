//! The card editor: the markdown source on one side, its sides as previewed on the other.
//!
//! Saving is explicit. The screen reports the text through `on_save`; the caller writes it. A card
//! with no content is not saved: its save button is disabled.

use std::path::{Path, PathBuf};

use nsh_core::prelude::*;
use nsh_core::file_dialog::OpenDialog;
use typr_core::sides;

use crate::ui::{
    button::{Variant, button},
    prose::prose,
    source::{source, source_with},
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
    editor_with_draft(initial.clone(), None, on_save, on_attach, |_| {})
}

/// The editor opening on an unsaved draft when there is one, and reporting each edit so the caller
/// can write the draft down. A draft that differs from the saved text opens marked "Unsaved".
pub fn editor_with_draft(
    saved_text: String,
    draft_text: Option<String>,
    on_save: impl Fn(String) + Clone + 'static,
    on_attach: impl Fn(PathBuf) -> std::io::Result<String> + Clone + 'static,
    on_draft: impl Fn(&str) + 'static,
) -> impl IntoElement {
    let draft = signal(draft_text.unwrap_or_else(|| saved_text.clone()));
    let saved = signal(saved_text);
    let attach_error = signal(None::<SharedString>);
    let save_text = on_save.clone();
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
                .child(state_badge)
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
