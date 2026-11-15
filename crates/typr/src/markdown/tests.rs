use rune::{Env, Runtime};

use super::CASES;
use typr_core::markdown_ext::supported::{SUPPORTED, Support};

/// A document with nothing around it, so the runtime's frames draw only the markdown.
struct Doc(String);

fn draw(s: &Doc, _: &mut rune::Cx<Doc>) -> rune::El {
    super::body(&s.0)
}

/// Every text run the runtime laid out, in order.
fn drawn(source: &str) -> String {
    let mut runtime = Runtime::new(
        Doc(source.to_string()),
        draw,
        (900.0, 600.0),
        Env::new(true, 1.0),
    );
    runtime.frame(0.0);
    runtime.frame(0.1);
    runtime
        .elements()
        .into_iter()
        .filter(|element| !element.exiting)
        .filter_map(|element| element.text)
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn every_supported_construct_has_a_case() {
    let listed: Vec<&str> = SUPPORTED.iter().map(|(name, _)| *name).collect();
    let cases: Vec<&str> = CASES.iter().map(|(name, _)| *name).collect();
    assert_eq!(
        listed, cases,
        "the inventory and the cases name the same constructs"
    );
}

#[test]
fn a_degraded_construct_keeps_its_words() {
    for (name, source) in CASES {
        let Some((_, support)) = SUPPORTED.iter().find(|(listed, _)| listed == name) else {
            panic!("{name} is not in the inventory");
        };
        if *support == Support::Rendered {
            continue;
        }
        let shown = drawn(source);
        assert!(!shown.trim().is_empty(), "{name} rendered nothing");
    }
}

#[test]
fn a_highlight_loses_its_markers_on_the_page() {
    let shown = drawn("==the Krebs cycle==");
    assert!(shown.contains("the Krebs cycle"), "{shown}");
    assert!(!shown.contains("=="), "no marker reaches the page: {shown}");
}

#[test]
fn an_unknown_construct_is_shown_as_written() {
    let source = "unclosed [[[badge] and \\frac{a}{b}";
    let shown = drawn(source);
    assert!(shown.contains("[[[badge]"), "{shown}");
    assert!(shown.contains("\\frac{a}{b}"), "{shown}");
}

#[test]
fn the_preview_and_the_review_render_the_same_highlight() {
    let source = "==the Krebs cycle==";
    // The review draws the card's line; the editor's preview wraps it first.
    let review = super::text(source);
    let preview = super::text(&crate::editor::preview_body(source));
    assert_eq!(review, preview);
    let shown = drawn(&preview);
    assert!(!shown.contains("=="), "{shown}");
}
