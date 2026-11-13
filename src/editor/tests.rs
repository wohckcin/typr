use super::{Editor, preview_key, view};
use crate::store::Id;

#[test]
fn editing_one_card_leaves_another_cards_parse_alone() {
    let first = Editor::new(Some(1 as Id), "one\n---\ntwo".to_string());
    let mut second = Editor::new(Some(2 as Id), "three\n---\nfour".to_string());
    let untouched = first.memo();

    second.buffer.insert(" more");
    second.settle();

    assert_ne!(
        second.memo(),
        preview_key(Some(2), "three\n---\nfour"),
        "the edited card re-parses"
    );
    assert_eq!(
        first.memo(),
        untouched,
        "the other card's parse is untouched"
    );
    assert_ne!(
        first.memo(),
        second.memo(),
        "different cards never share a parse"
    );
}

#[test]
fn typing_into_the_buffer_changes_the_preview_source_without_a_mode() {
    let mut editor = Editor::new(None, String::new());
    assert!(!editor.dirty());
    let before = editor.memo();

    editor.buffer.insert("**bold**");
    editor.settle();

    assert!(editor.dirty());
    assert_eq!(editor.buffer.text, "**bold**");
    assert_ne!(
        editor.memo(),
        before,
        "the preview is derived from the buffer, and moved with it"
    );
}

#[test]
fn the_preview_renders_the_typed_markdown() {
    let editor = Editor::new(None, "**bold**".to_string());
    let mut runtime = rune::Runtime::new(
        editor,
        |s: &Editor, _: &mut rune::Cx<Editor>| view::<Editor>(s),
        (900.0, 600.0),
        rune::Env::new(true, 1.0),
    );
    runtime.frame(0.0);
    assert!(
        runtime.find("preview-pane").is_some(),
        "the rendered side is on screen"
    );
    assert!(runtime.find("sides").is_some(), "and so is the source");
}

#[test]
fn a_two_side_card_reports_its_sides_while_typing() {
    let mut editor = Editor::new(None, "front".to_string());
    assert_eq!(editor.sides().len(), 1);
    editor.buffer.insert("\n---\nback");
    assert_eq!(editor.sides(), vec!["front\n", "back"]);
}

#[test]
fn leaving_an_unsaved_card_asks_first() {
    let mut editor = Editor::new(None, "old".to_string());
    editor.buffer.insert(" text");
    assert_eq!(editor.request_leave(), super::Leaving::Ask);
    assert!(editor.asking);

    assert_eq!(
        editor.resolve(super::Choice::Discard),
        Some(super::Answer::Discard)
    );
    assert_eq!(editor.buffer.text, "old", "discarding puts the buffer back");
    assert!(!editor.dirty());
    assert!(!editor.asking);
}

#[test]
fn saving_clears_the_unsaved_indicator_and_leaving_then_does_not_ask() {
    let mut editor = Editor::new(None, String::new());
    editor.buffer.insert("front");
    assert!(editor.dirty());

    editor.save();
    assert!(!editor.dirty());
    assert_eq!(editor.request_leave(), super::Leaving::Leave);
    assert!(!editor.asking);
}

#[test]
fn staying_keeps_the_edited_text() {
    let mut editor = Editor::new(None, "old".to_string());
    editor.buffer.insert("er");
    editor.request_leave();

    assert_eq!(editor.resolve(super::Choice::Stay), None);
    assert_eq!(editor.buffer.text, "older");
    assert!(editor.dirty());
    assert!(!editor.asking);
}

#[test]
fn the_confirmation_lists_save_discard_and_stay() {
    let mut editor = Editor::new(None, "card".to_string());
    editor.buffer.insert(" edited");
    editor.request_leave();
    let mut runtime = rune::Runtime::new(
        editor,
        |s: &Editor, _: &mut rune::Cx<Editor>| view::<Editor>(s),
        (900.0, 600.0),
        rune::Env::new(true, 1.0),
    );
    for _ in 0..30 {
        runtime.frame(1.0 / 60.0);
    }
    for key in ["leave-save", "leave-discard", "leave-stay"] {
        assert!(runtime.find(key).is_some(), "{key} is offered");
    }

    let (x, y) = runtime.find("leave-discard").unwrap();
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.0);
    assert_eq!(runtime.state.buffer.text, "card");
    assert!(!runtime.state.asking);
}

#[test]
fn an_empty_card_is_refused_and_explained_instead_of_rendered() {
    let mut editor = Editor::new(None, String::new());
    editor.buffer.replace_text("  \n\n---\n \t".to_string());
    assert!(!editor.saveable());

    assert!(!editor.save(), "nothing is stored");
    assert_eq!(editor.refusal(), Some("This card is empty"));
    assert!(editor.dirty());

    let runtime_view = |s: &Editor, _: &mut rune::Cx<Editor>| view::<Editor>(s);
    let mut runtime = rune::Runtime::new(
        editor,
        runtime_view,
        (900.0, 600.0),
        rune::Env::new(true, 1.0),
    );
    runtime.frame(0.0);
    assert!(
        runtime.find("preview-empty").is_some(),
        "the preview explains itself"
    );
    assert!(runtime.find("refusal").is_some());

    runtime.state.buffer.insert("front");
    assert_eq!(
        runtime.state.refusal(),
        None,
        "and the refusal clears once there is content"
    );
    assert!(runtime.state.save());
    assert!(!runtime.state.dirty());
}

#[test]
fn a_missing_attachment_renders_a_placeholder_instead_of_breaking_the_preview() {
    let dir = tempfile::tempdir().unwrap();
    let mut editor = Editor::new(None, "Look:\n\n![diagram](diagram.png)".to_string());
    editor.attachments = crate::attachments::Attachments::detached(dir.path().to_path_buf());

    let mut runtime = rune::Runtime::new(
        editor,
        |s: &Editor, _: &mut rune::Cx<Editor>| view::<Editor>(s),
        (900.0, 600.0),
        rune::Env::new(true, 1.0),
    );
    for _ in 0..5 {
        runtime.frame(1.0 / 60.0);
    }
    assert!(
        runtime.find("preview-pane").is_some(),
        "the preview still draws"
    );
    assert_eq!(
        runtime.state.attachments.resolve("diagram.png"),
        None,
        "and nothing was loaded"
    );
}

#[test]
fn attaching_an_image_copies_it_and_writes_its_reference() {
    let dir = tempfile::tempdir().unwrap();
    let source_dir = tempfile::tempdir().unwrap();
    let source = source_dir.path().join("kreb cycle.png");
    std::fs::write(&source, b"png").unwrap();

    let mut editor = Editor::new(None, "front".to_string());
    editor.attachments = crate::attachments::Attachments::detached(dir.path().to_path_buf());

    let name = editor.attach_image(&source).unwrap();
    assert_eq!(name, "kreb cycle.png");
    assert!(
        editor.buffer.text.contains("![kreb cycle](kreb cycle.png)"),
        "the reference is in the card"
    );
    assert!(
        editor.attachments.resolve(&name).is_some(),
        "and the file is beside the library"
    );
}

#[test]
fn the_confirmation_saves_the_buffer_it_offers_to_save() {
    let mut editor = Editor::new(None, "card".to_string());
    editor.buffer.insert(" edited");
    editor.request_leave();
    let mut runtime = rune::Runtime::new(
        editor,
        |s: &Editor, _: &mut rune::Cx<Editor>| view::<Editor>(s),
        (900.0, 600.0),
        rune::Env::new(true, 1.0),
    );
    for _ in 0..30 {
        runtime.frame(1.0 / 60.0);
    }

    let (x, y) = runtime.find("leave-save").unwrap();
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.0);
    assert_eq!(runtime.state.saved, "card edited", "Save keeps the edit");
    assert!(!runtime.state.dirty(), "and nothing is left unsaved");
    assert!(!runtime.state.asking, "the question is answered");
}

#[test]
fn the_save_button_stores_the_buffer_without_leaving() {
    let mut editor = Editor::new(None, "card".to_string());
    editor.buffer.insert(" edited");
    let mut runtime = rune::Runtime::new(
        editor,
        |s: &Editor, _: &mut rune::Cx<Editor>| view::<Editor>(s),
        (900.0, 600.0),
        rune::Env::new(true, 1.0),
    );
    for _ in 0..30 {
        runtime.frame(1.0 / 60.0);
    }

    let (x, y) = runtime.find("save-card").unwrap();
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.0);
    assert_eq!(runtime.state.saved, "card edited");
    assert!(!runtime.state.dirty());
}
