use super::{Shape, group_of, masked, shape, shown};

fn keys(markdown: &str) -> Vec<String> {
    shape(markdown).prompt_keys()
}

#[test]
fn one_unnumbered_span_is_one_prompt() {
    assert_eq!(keys("The hormone {{insulin}} matters."), ["cloze:0"]);
}

#[test]
fn two_numbered_groups_are_two_prompts() {
    assert_eq!(
        keys("{{1::Insulin}} and {{2::glucagon}} are hormones."),
        ["cloze:1", "cloze:2"]
    );
}

#[test]
fn numbered_and_unnumbered_spans_keep_their_own_groups() {
    assert_eq!(
        keys("{{1::first}} {{loose}} {{2::second}}"),
        ["cloze:0", "cloze:1", "cloze:2"]
    );
}

#[test]
fn a_span_inside_a_fence_is_text() {
    assert_eq!(keys("```\n{{example}}\n```\nplain"), Vec::<String>::new());
}

#[test]
fn a_span_inside_inline_code_is_text() {
    assert_eq!(keys("Write `{{example}}` to hide."), Vec::<String>::new());
    assert_eq!(keys("Write ``a {{b}} c`` to hide."), Vec::<String>::new());
}

#[test]
fn an_unclosed_brace_is_text() {
    let shape = shape("half {{open");
    assert_eq!(shape.prompt_keys(), Vec::<String>::new());
    assert!(matches!(shape, Shape::Sides(_)));
}

#[test]
fn a_span_across_lines_is_text() {
    assert_eq!(keys("{{one\ntwo}}"), Vec::<String>::new());
}

#[test]
fn a_card_with_a_separator_and_a_span_has_one_face_and_no_sides() {
    let shape = shape("front {{hidden}}\n---\nback");
    assert!(shape.hides_text(), "hidden text wins over the separator");
    assert_eq!(shape.prompt_keys(), ["cloze:0"]);
}

#[test]
fn a_card_without_hidden_text_is_sides() {
    assert_eq!(shape("front\n---\nback").prompt_keys(), ["fwd"]);
    assert_eq!(shape("one line").prompt_keys(), Vec::<String>::new());
    assert_eq!(shape("one line").prompt_count(), 0);
}

#[test]
fn a_group_number_is_read_back_out_of_its_key() {
    assert_eq!(group_of("cloze:0"), Some(0));
    assert_eq!(group_of("cloze:12"), Some(12));
    assert_eq!(group_of("fwd"), None);
    assert_eq!(group_of("cloze:x"), None);
}

#[test]
fn masking_group_one_leaves_group_two_visible() {
    let text = "{{1::Insulin}} and {{2::glucagon}} are hormones.";
    let shape = shape(text);
    assert_eq!(
        masked(text, &shape, "cloze:1"),
        "[ … ] and glucagon are hormones."
    );
}

#[test]
fn masking_the_unnumbered_group_masks_every_loose_span() {
    let text = "The {{hormone}} is made by the {{pancreas}}.";
    let shape = shape(text);
    assert_eq!(
        masked(text, &shape, "cloze:0"),
        "The [ … ] is made by the [ … ]."
    );
}

#[test]
fn masking_leaves_a_card_without_hidden_text_alone() {
    let text = "front\n---\nback";
    let shape = shape(text);
    assert_eq!(masked(text, &shape, "fwd"), text);
    assert_eq!(shown(text, &shape), text);
}

#[test]
fn the_shown_form_reproduces_the_text_without_delimiters() {
    let text = "{{1::Insulin}} and {{2::glucagon}} are hormones.";
    let shape = shape(text);
    assert_eq!(shown(text, &shape), "Insulin and glucagon are hormones.");
}

#[test]
fn the_shown_form_masks_nothing_even_when_it_is_asked_for_another_group() {
    let text = "A {{b}} c {{2::d}} e";
    let shape = shape(text);
    assert_eq!(shown(text, &shape), "A b c d e");
    assert_eq!(masked(text, &shape, "cloze:2"), "A b c [ … ] e");
}
