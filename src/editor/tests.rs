use super::{Editor, preview_key, view};
use crate::store::Id;

#[test]
fn editing_one_card_leaves_another_cards_parse_alone() {
    let first = Editor::new(Some(1 as Id), "one\n---\ntwo".to_string());
    let mut second = Editor::new(Some(2 as Id), "three\n---\nfour".to_string());
    let untouched = first.memo();

    second.buffer.insert(" more");
    second.settle();

    assert_ne!(second.memo(), preview_key(Some(2), "three\n---\nfour"), "the edited card re-parses");
    assert_eq!(first.memo(), untouched, "the other card's parse is untouched");
    assert_ne!(first.memo(), second.memo(), "different cards never share a parse");
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
    assert_ne!(editor.memo(), before, "the preview is derived from the buffer, and moved with it");
}

#[test]
fn the_preview_renders_the_typed_markdown() {
    let editor = Editor::new(None, "**bold**".to_string());
    let mut runtime = rune::Runtime::new(editor, |s: &Editor, _: &mut rune::Cx<Editor>| view::<Editor>(s),
        (900.0, 600.0), rune::Env::new(true, 1.0));
    runtime.frame(0.0);
    assert!(runtime.find("preview-pane").is_some(), "the rendered side is on screen");
    assert!(runtime.find("sides").is_some(), "and so is the source");
}

#[test]
fn a_two_side_card_reports_its_sides_while_typing() {
    let mut editor = Editor::new(None, "front".to_string());
    assert_eq!(editor.sides().len(), 1);
    editor.buffer.insert("\n---\nback");
    assert_eq!(editor.sides(), vec!["front\n", "back"]);
}
