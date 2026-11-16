//! The card editor: each side is previewed, and a card with no content cannot be saved. Saving
//! reports the text the user wrote.

use nsh_core::prelude::*;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;
use typr::screens::editor::{attach_image, editor, has_content, image_reference};

#[component]
fn Probe() -> impl IntoElement {
    typr::ui::tokens::install();
    let saved = signal(String::from("nothing saved"));
    view()
        .flex_1()
        .child(editor(
            "Question\n---\nAnswer".to_owned(),
            move |text| saved.set(text),
            |_| Ok("unused.png".to_owned()),
        ))
        .child(text(move || format!("saved={}", saved.get())))
}

#[component]
fn Empty() -> impl IntoElement {
    typr::ui::tokens::install();
    editor(String::new(), |_| {}, |_| Ok("unused.png".to_owned()))
}

#[test]
fn each_side_of_the_card_is_previewed() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
    app.wait_for("Question")?;
    app.wait_for("Answer")?;
    Ok(())
}

#[test]
fn a_card_with_no_content_cannot_be_saved() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Empty>())).unwrap();
    app.wait_for("role:button[Save]")?;
    assert!(!app.screen().enabled("role:button[Save]")?);
    Ok(())
}

#[test]
fn pressing_save_reports_the_text() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
    app.wait_for("role:button[Save]")?;
    app.press("role:button[Save]")?;
    app.wait_for("text:/saved=Question/")?;
    Ok(())
}

#[test]
fn only_text_with_something_in_it_has_content() {
    assert!(has_content("Question"));
    assert!(!has_content("  \n\t "));
}

#[test]
fn a_hidden_answer_is_previewed_masked() {
    let shown = typr::screens::editor::preview_of("The {{hormone}} is made by the {{pancreas}}.");
    assert!(!shown.contains("hormone") && !shown.contains("pancreas"), "{shown}");
}

#[test]
fn attaching_a_picture_copies_it_and_writes_its_reference() {
    let dir = tempfile::tempdir().expect("temp dir");
    let source = dir.path().join("cycle.png");
    std::fs::write(&source, b"png").expect("source");

    let root = dir.path().join("attachments");
    let name = attach_image(&source, &root).expect("attach");
    assert_eq!(name, "cycle.png");
    assert!(root.join(&name).is_file(), "the picture is copied beside the library");
    assert_eq!(image_reference("cycle", &name), "![cycle](cycle.png)");
}

#[test]
fn a_picture_that_cannot_be_copied_is_refused() {
    let dir = tempfile::tempdir().expect("temp dir");
    let missing = dir.path().join("nothing.png");
    assert!(attach_image(&missing, &dir.path().join("attachments")).is_err());
}
