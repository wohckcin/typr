use super::split;

#[test]
fn a_bare_separator_splits_a_card() {
    assert_eq!(split("Question\n---\nAnswer"), vec!["Question\n", "Answer"]);
}

#[test]
fn padding_and_carriage_returns_do_not_stop_a_separator() {
    assert_eq!(split("front\r\n  ---  \r\nback"), vec!["front\r\n", "back"]);
}

#[test]
fn hyphens_with_other_content_are_not_a_separator() {
    assert_eq!(
        split("above\n--- below\n***\n___"),
        vec!["above\n--- below\n***\n___"]
    );
}

#[test]
fn a_separator_inside_a_fence_is_content() {
    let markdown = "before\n```\n---\n```\nafter";
    assert_eq!(split(markdown), vec![markdown]);
}

#[test]
fn a_tilde_fence_also_hides_a_separator() {
    let markdown = "before\n~~~\n---\n~~~\nafter";
    assert_eq!(split(markdown), vec![markdown]);
}

#[test]
fn an_empty_side_is_dropped_and_a_note_has_one_side() {
    assert_eq!(split("only side\n---\n   \n").len(), 1);
    assert_eq!(split("   \n---\n").len(), 0);
}

#[test]
fn three_sides_keep_their_order() {
    assert_eq!(
        split("one\n---\ntwo\n---\nthree"),
        vec!["one\n", "two\n", "three"]
    );
}
