use super::{Editor, MaskChange, caption, preview_body, preview_key, view};
use typr_core::hidden;
use typr_core::store::{CardKind, Id, MaskRect, MaskRow};

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

#[test]
fn an_attachment_that_fails_says_so_in_the_header() {
    let mut editor = Editor::new(None, "card".to_string());
    let missing = std::path::Path::new("/nonexistent/picture.png");
    assert!(editor.attach_image(missing).is_err(), "refused");
    let refusal = editor.refusal().expect("the header says why");
    assert!(refusal.contains("not attached"), "got {refusal}");
    assert_eq!(editor.buffer.text, "card", "and nothing was inserted");
}

#[test]
fn the_tag_line_keeps_one_spelling_of_each_tag() {
    let mut editor = Editor::new(Some(1), "front".to_string());
    editor.load_tags(&["Grammar".to_string(), "verbs".to_string()]);
    assert_eq!(
        editor.tag_list(),
        vec!["Grammar", "verbs"],
        "it shows the card's tags"
    );
    assert!(!editor.dirty(), "loading the card's tags is not an edit");

    editor
        .tags
        .replace_text("Grammar, grammar  , Verbs, ".to_string());
    assert_eq!(
        editor.tag_list(),
        vec!["Grammar", "Verbs"],
        "a tag that differs only by case is dropped, keeping the first spelling and the typed case"
    );
    assert!(editor.dirty(), "a changed tag line is an unsaved change");
}

#[test]
fn the_tag_field_keeps_each_keystroke() {
    let editor = Editor::new(Some(1 as Id), "front\n---\nback".to_string());
    let mut runtime = rune::Runtime::new(
        editor,
        |s: &Editor, _: &mut rune::Cx<Editor>| view::<Editor>(s),
        (960.0, 640.0),
        rune::Env::new(true, 1.0),
    );
    runtime.frame(0.0);
    let (x, y) = runtime.find("tags").expect("the tag field");
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.1);
    for letter in ["a", "b", "c"] {
        runtime.key(rune::KeyEvent {
            key: rune::Key::Text(letter.to_string()),
            mods: rune::Mods::default(),
        });
        runtime.frame(0.1);
    }
    assert_eq!(runtime.state.tags.text, "abc");
}

#[test]
fn saving_marks_the_tags_as_stored_too() {
    let mut editor = Editor::new(Some(1), "front\n---\nback".to_string());
    editor.load_tags(&["Grammar".to_string()]);
    assert!(!editor.dirty(), "a loaded card is clean");

    editor.tags.replace_text("Grammar, verbs".to_string());
    assert!(editor.dirty(), "the tag line counts as unsaved work");

    assert!(editor.save());
    assert!(
        !editor.dirty(),
        "saving stores the tags as well as the sides"
    );
    assert_eq!(editor.tag_list(), vec!["Grammar", "verbs"]);
}

#[test]
fn a_draft_lands_in_a_card_that_has_nothing_typed() {
    let mut editor = Editor::new(None, String::new());
    editor.restore_draft("");
    assert_eq!(editor.buffer.text, "", "an empty draft changes nothing");

    editor.restore_draft("half a thought");
    assert_eq!(editor.buffer.text, "half a thought");

    let mut typed = Editor::new(None, String::new());
    typed.buffer.replace_text("what the user typed".to_string());
    typed.restore_draft("a stale draft");
    assert_eq!(
        typed.buffer.text, "what the user typed",
        "a draft never overwrites what is on screen"
    );
}

#[test]
fn a_card_that_hides_text_previews_with_its_first_group_masked() {
    let mut editor = Editor::new(None, "The {{gland}} is small.".to_string());
    editor.settle();

    assert_eq!(
        preview_body(&editor.buffer.text),
        "The [ … ] is small.",
        "the preview asks the question a session will ask"
    );
    assert!(
        !preview_body(&editor.buffer.text).contains("gland"),
        "the answer is not on screen while it is being written"
    );
}

#[test]
fn the_preview_masks_every_group_while_a_review_masks_only_the_one_asked_for() {
    let text = "Our lungs pull {{2::oxygen}} from the {{1::air}}";

    assert_eq!(
        preview_body(text),
        "Our lungs pull [2 … ] from the [1 … ]",
        "the preview hides every answer and names the group each mask belongs to"
    );
    assert_eq!(
        hidden::masked(text, &hidden::shape(text), "cloze:1"),
        "Our lungs pull oxygen from the [ … ]",
        "a review hides only the group it asks about"
    );
}

#[test]
fn the_caption_counts_prompts_for_hidden_text_and_sides_for_the_rest() {
    assert_eq!(caption("{{1::alpha}} and {{2::beta}}"), "2 prompts");
    assert_eq!(caption("The {{gland}} is small."), "1 prompt");
    assert_eq!(caption("front\n---\nback"), "2 sides");
    assert_eq!(
        caption("only a note"),
        "One side: this is a note and never queues"
    );
    assert_eq!(caption("   "), "Empty card");
}

#[test]
fn the_preview_says_prompts_on_screen_for_a_hidden_text_card() {
    let mut editor = Editor::new(None, "{{1::alpha}} and {{2::beta}}".to_string());
    editor.settle();
    let mut runtime = rune::Runtime::new(
        editor,
        |s: &Editor, _: &mut rune::Cx<Editor>| view::<Editor>(s),
        (900.0, 600.0),
        rune::Env::new(true, 1.0),
    );
    for _ in 0..30 {
        runtime.frame(1.0 / 60.0);
    }

    let caption = runtime
        .elements()
        .into_iter()
        .filter(|element| element.key.as_deref() == Some("preview-caption"))
        .filter_map(|element| element.text)
        .collect::<Vec<String>>()
        .join("");

    assert_eq!(
        caption, "2 prompts",
        "the preview counts prompts, not sides"
    );
    assert_eq!(runtime.state.prompts(), 2);
}

#[test]
fn the_preview_masks_every_group_and_names_the_one_it_hides() {
    let body = preview_body("Our lungs pull {{2::oxygen}} from the {{1::air}}.");
    assert!(body.contains("[1 … ]"), "the first group is named: {body}");
    assert!(body.contains("[2 … ]"), "the second group is named: {body}");
    assert!(!body.contains("oxygen"), "no answer is readable: {body}");
    assert!(!body.contains("air."), "no answer is readable: {body}");
}

fn editor_with_picture() -> (tempfile::TempDir, Editor) {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let source = dir.path().join("plate.png");
    std::fs::write(&source, b"not really a png").expect("a picture to copy");
    let mut editor = Editor::new(None, String::new());
    editor.attachments = crate::attachments::Attachments::detached(dir.path().join("attachments"));
    (dir, editor)
}

#[test]
fn choosing_an_image_copies_it_and_stamps_the_card() {
    let (dir, mut editor) = editor_with_picture();
    let source = dir.path().join("plate.png");

    editor.choose_image(&source).expect("the picture is copied");

    assert_eq!(
        editor.kind,
        CardKind::Diagram,
        "choosing a picture makes a diagram"
    );
    assert!(editor.diagram());
    let name = editor.image.clone().expect("the copied name");
    let stored = editor
        .attachments
        .resolve(&name)
        .expect("the copy is in the attachments folder");
    assert!(
        stored.exists(),
        "the picture was copied: {}",
        stored.display()
    );
    assert!(editor.dirty(), "the picture is not stored yet");
    assert!(
        editor.saveable(),
        "a card with a picture is worth keeping without text"
    );
}

#[test]
fn an_image_that_cannot_be_copied_leaves_the_card_alone() {
    let (_dir, mut editor) = editor_with_picture();

    let refused = editor.choose_image(std::path::Path::new("/nonexistent/nothing.png"));

    assert!(
        refused.is_err(),
        "a picture that is not there cannot be copied"
    );
    assert!(editor.image.is_none());
    assert!(!editor.diagram(), "the card stays a text card");
    assert!(!editor.dirty());
}

#[test]
fn a_drag_stores_a_fractional_rectangle() {
    let mut editor = Editor::new(None, String::new());
    editor.set_kind(CardKind::Diagram);
    let mut runtime = rune::Runtime::new(
        editor,
        |s: &Editor, _: &mut rune::Cx<Editor>| view::<Editor>(s),
        (900.0, 700.0),
        rune::Env::new(true, 1.0),
    );
    runtime.frame(0.0);
    runtime.frame(0.1);
    let pane = runtime
        .elements()
        .into_iter()
        .find(|element| element.key.as_deref() == Some("diagram-pane"))
        .map(|element| element.rect)
        .expect("the picture pane");

    // A quarter of the pane wide and a third of it tall, measured from the pane the frame drew.
    let (from_x, from_y) = (pane[0] + pane[2] * 0.2, pane[1] + pane[3] * 0.1);
    let (to_x, to_y) = (pane[0] + pane[2] * 0.45, pane[1] + pane[3] * 0.43);
    runtime.pointer_down(from_x, from_y);
    runtime.frame(0.11);
    runtime.pointer_move(to_x, to_y);
    runtime.frame(0.12);
    runtime.pointer_up(to_x, to_y);
    runtime.frame(0.2);

    let masks = &runtime.state.masks;
    assert_eq!(masks.len(), 1, "the drag made one rectangle");
    let width = masks[0].rect.w;
    assert!(
        (width - 0.25).abs() < 0.03,
        "a quarter-width drag across a {}-point pane is about a quarter: {width}",
        pane[2]
    );
    let height = masks[0].rect.h;
    assert!(
        (height - 0.33).abs() < 0.05,
        "a third-height drag is about a third: {height}"
    );
    assert_eq!(masks[0].group, 1, "the first rectangle opens group one");
}

#[test]
fn a_click_adds_nothing() {
    let mut editor = Editor::new(None, String::new());
    editor.set_kind(CardKind::Diagram);
    let mut runtime = rune::Runtime::new(
        editor,
        |s: &Editor, _: &mut rune::Cx<Editor>| view::<Editor>(s),
        (900.0, 700.0),
        rune::Env::new(true, 1.0),
    );
    runtime.frame(0.0);
    runtime.frame(0.1);
    let pane = runtime
        .elements()
        .into_iter()
        .find(|element| element.key.as_deref() == Some("diagram-pane"))
        .map(|element| element.rect)
        .expect("the picture pane");
    // The top of the pane, above the controls, so the click reaches the pane itself.
    let (x, y) = (pane[0] + pane[2] / 2.0, pane[1] + 6.0);

    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.2);

    assert!(
        runtime.state.masks.is_empty(),
        "a press and release in place is a click, not a rectangle"
    );
}

#[test]
fn merging_selects_one_group_for_both() {
    let mut editor = Editor::new(None, String::new());
    editor.set_kind(CardKind::Diagram);
    editor.masks = vec![
        MaskRow {
            group: 1,
            rect: MaskRect::new(0.1, 0.1, 0.1, 0.1),
        },
        MaskRow {
            group: 2,
            rect: MaskRect::new(0.5, 0.1, 0.1, 0.1),
        },
    ];
    editor.selected = 1;

    let change = editor.merge_selected().expect("two rectangles to merge");

    assert_eq!(
        editor.masks[0].group, editor.masks[1].group,
        "both answer together"
    );
    match change {
        MaskChange::Regroup { rects, group } => {
            assert_eq!(group, 1, "the second rectangle joins the first group");
            assert_eq!(rects.len(), 2);
        }
        other => panic!("merging stores one group: {other:?}"),
    }
}

#[test]
fn a_rectangle_can_be_placed_and_resized_in_steps() {
    let mut editor = Editor::new(None, String::new());
    editor.set_kind(CardKind::Diagram);

    // The middle of the picture, then wider and taller in steps: what the buttons and the Alt
    // chords both call.
    let added = editor.add_rect(MaskRect::new(0.35, 0.35, 0.3, 0.3));
    assert!(matches!(added, MaskChange::Add { group: 1, .. }));
    editor
        .resize_selected(0.05, 0.0)
        .expect("a rectangle to widen");
    editor
        .resize_selected(0.0, 0.05)
        .expect("a rectangle to heighten");

    let rect = editor.masks[0].rect;
    assert!((rect.w - 0.35).abs() < 1e-6, "wider by a step: {}", rect.w);
    assert!((rect.h - 0.35).abs() < 1e-6, "taller by a step: {}", rect.h);
    assert_eq!(
        editor.add_rect(MaskRect::new(0.1, 0.1, 0.1, 0.1)),
        MaskChange::Add {
            group: 2,
            rect: MaskRect::new(0.1, 0.1, 0.1, 0.1)
        },
        "a second rectangle asks in its own group"
    );
}
