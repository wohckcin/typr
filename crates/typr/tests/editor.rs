//! The card editor: each side is previewed, and a card with no content cannot be saved. Saving
//! reports the text the user wrote.

use nsh_core::prelude::*;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;
use typr::screens::editor::{editor, has_content};

#[component]
fn Probe() -> impl IntoElement {
    typr::ui::tokens::install();
    let saved = signal(String::from("nothing saved"));
    view()
        .flex_1()
        .child(editor("Question\n---\nAnswer".to_owned(), move |text| saved.set(text)))
        .child(text(move || format!("saved={}", saved.get())))
}

#[component]
fn Empty() -> impl IntoElement {
    typr::ui::tokens::install();
    editor(String::new(), |_| {})
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
