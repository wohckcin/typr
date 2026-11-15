use super::rewrite;

/// The one card the tests read: every construct the parser cannot see, as a card writes it.
const CARD: &str = "\
==the Krebs cycle==
20^th^ century
H~2~0
{漢字}(かんじ)
";

#[test]
fn equals_inside_code_are_left_alone() {
    let source = "the `a == b` comparison";
    assert_eq!(rewrite(source), source);
}

#[test]
fn a_fence_is_left_alone() {
    let source = "```\na == b\n20^th^ H~2~0 {x}(y)\n```";
    assert_eq!(rewrite(source), source);
}

#[test]
fn ruby_needs_braces_not_just_parentheses() {
    let source = "mitosis (cell division)";
    assert_eq!(rewrite(source), source);
}

#[test]
fn preprocessing_twice_changes_nothing() {
    let once = rewrite(CARD).into_owned();
    let twice = rewrite(&once).into_owned();
    assert_eq!(once, twice);
}

#[test]
fn a_highlight_is_left_for_the_renderer() {
    // The toolkit draws `==text==` on a background, so the markers must reach it unchanged.
    assert_eq!(rewrite("==the Krebs cycle=="), "==the Krebs cycle==");
}

#[test]
fn an_ordinal_is_raised() {
    assert_eq!(rewrite("20^th^ century"), "20ᵗʰ century");
}

#[test]
fn a_formula_is_lowered() {
    assert_eq!(rewrite("H~2~0"), "H₂0");
}

#[test]
fn a_gloss_keeps_both_words() {
    assert_eq!(rewrite("{漢字}(かんじ)"), "漢字 (かんじ)");
}

#[test]
fn a_doubled_tilde_stays_strikethrough() {
    assert_eq!(rewrite("~~gone~~"), "~~gone~~");
}

#[test]
fn a_character_without_a_raised_form_is_left_as_written() {
    assert_eq!(rewrite("20^週^"), "20^週^");
}
