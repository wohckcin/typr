//! The card editor: each side is previewed, and a card with no content cannot be saved. Saving
//! reports the text the user wrote.

use nsh_core::prelude::*;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;
use typr::screens::editor::{
    Answer, CardText, Cover, Diagram, Field, GroupMerge, Leaving, MaskAdded, MaskGroup, Template,
    attach_image, cover_box, editor, field_table, has_content, image_reference, leave_decision,
    leaving_choice, prompt_count, rect_from_corners, resolve_images, template_preview,
};

#[component]
fn Probe() -> impl IntoElement {
    typr::ui::tokens::install();
    let saved = signal(String::from("nothing saved"));
    view()
        .flex_1()
        .child(
            editor()
                .saved("Question\n---\nAnswer")
                .on_save(move |saved_card: &CardText| saved.set(saved_card.text.clone())),
        )
        .child(text(move || format!("saved={}", saved.get())))
}

#[component]
fn Empty() -> impl IntoElement {
    typr::ui::tokens::install();
    editor().saved("")
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
    assert!(
        !shown.contains("hormone") && !shown.contains("pancreas"),
        "{shown}"
    );
}

#[test]
fn attaching_a_picture_copies_it_and_writes_its_reference() {
    let dir = tempfile::tempdir().expect("temp dir");
    let source = dir.path().join("cycle.png");
    std::fs::write(&source, b"png").expect("source");

    let root = dir.path().join("attachments");
    let name = attach_image(&source, &root).expect("attach");
    assert_eq!(name, "cycle.png");
    assert!(
        root.join(&name).is_file(),
        "the picture is copied beside the library"
    );
    assert_eq!(image_reference("cycle", &name), "![cycle](cycle.png)");
}

#[test]
fn a_picture_that_cannot_be_copied_is_refused() {
    let dir = tempfile::tempdir().expect("temp dir");
    let missing = dir.path().join("nothing.png");
    assert!(attach_image(&missing, &dir.path().join("attachments")).is_err());
}

#[component]
fn Restored() -> impl IntoElement {
    typr::ui::tokens::install();
    let typed = signal(String::from("nothing typed"));
    view()
        .gap_2()
        .child(text(move || format!("typed={}", typed.get())))
        .child(
            editor()
                .saved("saved text")
                .draft(Some(SharedString::from("half a card")))
                .attachments("/tmp/typr-edit/attachments")
                .on_draft(move |text: &CardText| typed.set(text.text.clone())),
        )
}

#[test]
fn an_unsaved_draft_opens_marked_unsaved() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Restored>())).unwrap();
    app.wait_for("half a card")?;
    app.wait_for("Unsaved")?;
    Ok(())
}

#[test]
fn typing_reaches_the_draft_and_the_preview() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Restored>())).unwrap();
    app.wait_for("half a card")?;
    // The source editor is reachable by its accessibility label; the caret starts at the top.
    app.type_text("label:Card source", "!")?;
    app.wait_for("text:/typed=!half a card/")?;
    // The preview follows the same draft: the typed text is drawn there too.
    assert!(app.screen().count("text:/!half a card/")? >= 1);
    Ok(())
}

#[test]
fn an_attachment_reference_resolves_against_the_attachments_folder() {
    use std::path::Path;
    let root = Path::new("/tmp/typr/attachments");
    assert_eq!(
        resolve_images("see ![chart](chart.png) and more", root),
        "see ![chart](/tmp/typr/attachments/chart.png) and more"
    );
    // A reference that escapes the folder, or is absolute, is left as written: the renderer draws
    // its placeholder instead of loading something outside the library.
    assert_eq!(
        resolve_images("![x](../secret.png)", root),
        "![x](../secret.png)"
    );
    assert_eq!(
        resolve_images("![x](/etc/passwd)", root),
        "![x](/etc/passwd)"
    );
    assert_eq!(resolve_images("![x](a.png)", Path::new("")), "![x](a.png)");
}

#[test]
fn the_caption_counts_prompts_not_sides() {
    assert_eq!(prompt_count("Question\n---\nAnswer"), 1);
    assert_eq!(prompt_count("just a note"), 0);
    assert_eq!(prompt_count("A {{insulin}} and a {{pancreas}}"), 1);
    assert_eq!(prompt_count("{{1::insulin}} and {{2::pancreas}}"), 2);
    assert_eq!(prompt_count("hidden {{1::a}} beats\n---\na separator"), 1);
}

#[test]
fn leaving_asks_only_when_there_is_something_to_lose() {
    assert_eq!(leaving_choice(false), Leaving::Leave);
    assert_eq!(leaving_choice(true), Leaving::Ask);
}

#[test]
fn a_template_renders_from_its_fields() {
    let fields = vec![
        ("Word".to_owned(), "gato".to_owned()),
        ("Meaning".to_owned(), "cat".to_owned()),
    ];
    assert_eq!(
        template_preview("<<Word>>\n---\n<<Meaning>>", &fields, Some("Spanish")),
        "gato\n---\ncat"
    );
    assert_eq!(template_preview("<<Missing>>", &[], None), "");
}

#[component]
fn Decision() -> impl IntoElement {
    typr::ui::tokens::install();
    let open = signal(true);
    let answered = signal(String::from("nothing answered"));
    view()
        .gap_2()
        .child(text(move || format!("answered={}", answered.get())))
        .child(leave_decision(open, move |answer| {
            answered.set(match answer {
                Answer::Save => "save".to_owned(),
                Answer::Discard => "discard".to_owned(),
            })
        }))
}

#[test]
fn the_leave_question_answers_save_discard_or_stay() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Decision>())).unwrap();
    app.wait_for("Save this card before leaving?")?;
    app.press("role:button[Discard]")?;
    app.wait_for("answered=discard")?;
    app.wait_for_gone(
        "Save this card before leaving?",
        std::time::Duration::from_secs(5),
    )?;
    Ok(())
}

#[component]
fn Fields() -> impl IntoElement {
    typr::ui::tokens::install();
    let edited = signal(String::from("nothing edited"));
    view()
        .gap_2()
        .child(text(move || format!("edited={}", edited.get())))
        .child(field_table(
            vec![
                Field {
                    name: "Word".to_owned(),
                    value: "gato".to_owned(),
                    computed: false,
                },
                Field {
                    name: "Name".to_owned(),
                    value: "gato".to_owned(),
                    computed: true,
                },
                Field {
                    name: "Deck".to_owned(),
                    value: "Spanish".to_owned(),
                    computed: true,
                },
            ],
            move |name, value| edited.set(format!("{name}={value}")),
        ))
}

#[test]
fn the_field_table_types_a_value_and_shows_computed_rows_read_only() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Fields>())).unwrap();
    app.wait_for("Fields")?;
    assert_eq!(
        app.screen().count("text:/computed/")?,
        2,
        "both computed rows say so"
    );
    app.wait_for("Spanish")?;
    app.type_text("text:gato", "s")?;
    app.wait_for("text:/edited=Word=gatos/")?;
    Ok(())
}

#[test]
fn a_drag_across_the_picture_makes_a_fraction_rectangle() {
    let rect =
        rect_from_corners((400.0, 200.0), (200.0, 100.0), (300.0, 150.0)).expect("a rectangle");
    assert_eq!((rect.x, rect.y), (0.5, 0.5));
    assert_eq!((rect.w, rect.h), (0.25, 0.25));
    // The same fractions hold when the pane is drawn larger.
    assert_eq!(
        cover_box((800.0, 400.0), rect),
        (400.0, 200.0, 200.0, 100.0)
    );
    // A drag up and left from the same place covers the region on that side of it.
    let up_left =
        rect_from_corners((400.0, 200.0), (200.0, 100.0), (100.0, 50.0)).expect("a rectangle");
    assert_eq!(
        (up_left.x, up_left.y, up_left.w, up_left.h),
        (0.25, 0.25, 0.25, 0.25)
    );
}

#[test]
fn a_click_or_a_twitch_adds_nothing() {
    assert_eq!(
        rect_from_corners((400.0, 200.0), (100.0, 100.0), (100.0, 100.0)),
        None,
        "a click"
    );
    assert_eq!(
        rect_from_corners((400.0, 200.0), (100.0, 0.0), (102.0, 100.0)),
        None,
        "narrower than one percent"
    );
    assert_eq!(
        rect_from_corners((400.0, 200.0), (0.0, 100.0), (100.0, 101.0)),
        None,
        "shorter than one percent"
    );
    assert_eq!(
        rect_from_corners((0.0, 0.0), (0.0, 0.0), (100.0, 100.0)),
        None,
        "no pane yet"
    );
}

#[component]
fn Templated() -> impl IntoElement {
    typr::ui::tokens::install();
    view().child(
        editor()
            .saved("stored")
            .template(Template {
                text: "<<Word>>\n---\n<<Meaning>>".to_owned(),
                fields: vec![
                    Field {
                        name: "Word".to_owned(),
                        value: "gato".to_owned(),
                        computed: false,
                    },
                    Field {
                        name: "Meaning".to_owned(),
                        value: "cat".to_owned(),
                        computed: false,
                    },
                    Field {
                        name: "Name".to_owned(),
                        value: "gato".to_owned(),
                        computed: true,
                    },
                ],
                deck: Some("Spanish".to_owned()),
            })
            .attachments("/tmp/typr-edit/attachments"),
    )
}

#[test]
fn a_templated_card_shows_its_field_table_and_previews_the_template() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Templated>())).unwrap();
    app.wait_for("Fields")?;
    app.wait_for("gato")?;
    app.wait_for("cat")?;
    assert_eq!(
        app.screen().count("text:/computed/")?,
        1,
        "only Name is computed"
    );
    Ok(())
}

#[test]
fn a_card_without_a_template_shows_no_field_table() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
    app.wait_for("Question")?;
    assert!(
        !app.screen().shows("Fields"),
        "an untemplated card has no field table"
    );
    Ok(())
}

#[component]
fn LeavingProbe() -> impl IntoElement {
    typr::ui::tokens::install();
    let left = signal(String::from("not left"));
    let saved = signal(String::from("not saved"));
    view()
        .gap_2()
        .child(text(move || {
            format!("left={} saved={}", left.get(), saved.get())
        }))
        .child(
            editor()
                .saved("saved text")
                .draft(Some(SharedString::from("saved text!")))
                .on_save(move |saved_card: &CardText| saved.set(saved_card.text.clone()))
                .on_leave(move || left.set("yes".to_owned()))
                .on_discard(move || left.set("yes".to_owned())),
        )
}

#[test]
fn leaving_a_dirty_editor_asks_and_stay_keeps_it() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<LeavingProbe>())).unwrap();
    app.wait_for("text:/left=not left/")?;
    app.press_key("escape")?;
    app.wait_for("Save this card before leaving?")?;
    app.press("role:button[Stay]")?;
    app.wait_for_gone(
        "Save this card before leaving?",
        std::time::Duration::from_secs(5),
    )?;
    app.wait_for("text:/left=not left/")?;
    Ok(())
}

#[test]
fn leaving_a_dirty_editor_can_discard() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<LeavingProbe>())).unwrap();
    app.wait_for("text:/left=not left/")?;
    app.press_key("escape")?;
    app.wait_for("Save this card before leaving?")?;
    app.press("role:button[Discard]")?;
    app.wait_for("text:/left=yes saved=not saved/")?;
    Ok(())
}

#[test]
fn leaving_a_dirty_editor_can_save_first() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<LeavingProbe>())).unwrap();
    app.wait_for("text:/left=not left/")?;
    app.press_key("escape")?;
    app.wait_for("Save this card before leaving?")?;
    app.press("role:button[Save and leave]")?;
    app.wait_for("text:/left=yes saved=saved text!/")?;
    Ok(())
}

#[component]
fn CleanLeaving() -> impl IntoElement {
    typr::ui::tokens::install();
    let left = signal(String::from("not left"));
    view()
        .gap_2()
        .child(text(move || format!("left={}", left.get())))
        .child(
            editor()
                .saved("saved text")
                .on_leave(move || left.set("yes".to_owned())),
        )
}

#[test]
fn leaving_a_clean_editor_does_not_ask() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<CleanLeaving>())).unwrap();
    app.wait_for("text:/left=not left/")?;
    app.press_key("escape")?;
    app.wait_for("left=yes")?;
    assert!(!app.screen().shows("Save this card before leaving?"));
    Ok(())
}

#[component]
fn DiagramCard() -> impl IntoElement {
    typr::ui::tokens::install();
    let added = signal(String::from("none"));
    let merged = signal(String::from("none"));
    let removed = signal(String::from("none"));
    let cleared = signal(String::from("none"));
    view()
        .flex_1()
        .gap_2()
        .child(text(move || {
            format!(
                "added={} merged={} removed={} cleared={}",
                added.get(),
                merged.get(),
                removed.get(),
                cleared.get()
            )
        }))
        .child(
            editor()
                .saved("")
                .diagram(Diagram {
                    image: "cat.png".to_owned(),
                    caption: "the cat".to_owned(),
                    rows: vec![
                        Cover {
                            group: 1,
                            x: 0.1,
                            y: 0.1,
                            w: 0.2,
                            h: 0.2,
                        },
                        Cover {
                            group: 2,
                            x: 0.6,
                            y: 0.6,
                            w: 0.2,
                            h: 0.2,
                        },
                    ],
                })
                .on_add_mask(move |added_mask: &MaskAdded| {
                    let mask = added_mask.mask;
                    added.set(format!("{} w={:.2} x={:.2}", mask.group, mask.w, mask.x))
                })
                .on_merge_masks(move |merge: &GroupMerge| {
                    merged.set(format!("{} n={}", merge.group, merge.rects.len()))
                })
                .on_remove_mask_group(move |group: &MaskGroup| {
                    removed.set(format!("group={}", group.group))
                })
                .on_clear_diagram(move || cleared.set("yes".to_owned())),
        )
}

#[test]
fn a_diagram_card_shows_its_picture_its_caption_and_its_groups() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<DiagramCard>())).unwrap();
    app.wait_for("the cat")?;
    assert!(app.screen().shows("group 1"), "the first group's cover");
    assert!(app.screen().shows("group 2"), "the second group's cover");
    Ok(())
}

#[test]
fn a_drag_across_the_pane_adds_a_rectangle_to_the_last_group() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<DiagramCard>())).unwrap();
    app.wait_for("the cat")?;
    let by = Point {
        x: px(80.0),
        y: px(60.0),
    };
    app.drag("text:diagram", by)?;
    app.wait_for("text:/added=2 /")?;
    let drawn = app.screen().texts().join(" ");
    assert!(!drawn.contains("w=0.00"), "the rectangle has size: {drawn}");
    Ok(())
}

#[test]
fn a_click_on_the_pane_adds_nothing() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<DiagramCard>())).unwrap();
    app.wait_for("the cat")?;
    let nowhere = Point {
        x: px(0.0),
        y: px(0.0),
    };
    app.drag("text:diagram", nowhere)?;
    app.wait_for("text:/added=none/")?;
    Ok(())
}

#[test]
fn merging_a_picked_group_reports_its_rectangles_and_removing_it_reports_the_group()
-> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<DiagramCard>())).unwrap();
    app.wait_for("the cat")?;
    app.press("role:button[group 1]")?;
    app.press("role:button[Merge group]")?;
    app.wait_for("text:/merged=1 n=1/")?;
    app.press("role:button[group 2]")?;
    app.press("role:button[Remove group]")?;
    app.wait_for("text:/removed=group=2/")?;
    Ok(())
}

#[test]
fn a_cover_says_which_group_it_belongs_to() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<DiagramCard>())).unwrap();
    app.wait_for("the cat")?;
    assert_eq!(
        app.screen().count("label:rectangle in group 1")?,
        1,
        "the cover on group one says so"
    );
    assert_eq!(app.screen().count("label:rectangle in group 2")?, 1);
    Ok(())
}

#[test]
fn the_diagram_clears_only_after_the_question() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<DiagramCard>())).unwrap();
    app.wait_for("the cat")?;
    app.press("role:button[Clear the diagram]")?;
    app.wait_for("Clear this diagram?")?;
    app.press("role:button[Stay]")?;
    app.wait_for_gone("Clear this diagram?", std::time::Duration::from_secs(5))?;
    app.wait_for("text:/cleared=none/")?;
    app.press("role:button[Clear the diagram]")?;
    app.wait_for("Clear this diagram?")?;
    app.press("role:button[Clear]")?;
    app.wait_for("text:/cleared=yes/")?;
    Ok(())
}

#[component]
fn EmptyCard() -> impl IntoElement {
    typr::ui::tokens::install();
    view().flex_1().child(editor().saved(""))
}

#[component]
fn WrittenCard() -> impl IntoElement {
    typr::ui::tokens::install();
    view().flex_1().child(editor().saved("gato"))
}

#[test]
fn an_empty_card_offers_the_diagram_switch_and_a_written_one_does_not() -> Result<(), String> {
    let mut empty = Headless::start(EngineConfig::new(Root::of::<EmptyCard>())).unwrap();
    empty.wait_for("Make it a diagram")?;
    let mut written = Headless::start(EngineConfig::new(Root::of::<WrittenCard>())).unwrap();
    written.wait_for("Save")?;
    assert!(
        !written.screen().shows("Make it a diagram"),
        "switching a card with text would hide the text"
    );
    Ok(())
}

#[component]
fn CaretProbe() -> impl IntoElement {
    typr::ui::tokens::install();
    let value = signal("xyz".to_owned());
    let handle = typr::ui::source::source_handle();
    view()
        .flex_1()
        .gap_2()
        .child(text(move || value.get()))
        .child(
            typr::ui::button::button()
                .label("Insert")
                .on_press(move || {
                    typr::ui::source::insert_at_caret(&handle, "!");
                }),
        )
        .child(typr::ui::source::source_with(value, |_| {}, Some(&handle)))
}

#[test]
fn a_reference_is_typed_at_the_caret_not_appended() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<CaretProbe>())).unwrap();
    app.wait_for("xyz")?;
    app.press("role:button[Insert]")?;
    // The editor opens with its caret at the start, so the text goes in front of what is there.
    app.wait_for("text:/^!xyz$/")?;
    Ok(())
}

#[component]
fn DynamicHolder() -> impl IntoElement {
    typr::ui::tokens::install();
    let tick = signal(String::from(""));
    let left = signal(String::from("not left"));
    view()
        .flex_1()
        .gap_2()
        .child(text(move || {
            format!("tick={} left={}", tick.get(), left.get())
        }))
        .child(dynamic(move || {
            let _ = tick.get();
            editor()
                .saved("saved text")
                .draft(Some(SharedString::from("saved text!")))
                .on_leave(move || left.set("yes".to_owned()))
                .into_element()
        }))
}

#[test]
fn a_dirty_editor_inside_dynamic_still_asks() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<DynamicHolder>())).unwrap();
    app.wait_for("Unsaved")?;
    app.press_key("escape")?;
    app.wait_for("Save this card before leaving?")?;
    Ok(())
}

#[test]
fn the_way_back_asks_while_the_source_holds_the_keyboard() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<LeavingProbe>())).unwrap();
    app.wait_for("text:/left=not left/")?;
    // The focused code editor keeps Escape for itself, so the header's way back is the path that
    // always works.
    app.type_text("label:Card source", "?")?;
    app.press("role:button[Back]")?;
    app.wait_for("Save this card before leaving?")?;
    Ok(())
}
