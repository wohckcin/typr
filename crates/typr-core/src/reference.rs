//! `[[…]]` references between cards.
//!
//! Stored text keeps the author's `[[name]]`: a reference is found at save time and resolved into a
//! row, and the render pass rewrites it into a markdown link the renderer can make pressable. An
//! unresolved reference stays exactly as it was typed, so a typo is visible and fixable instead of
//! silently disappearing.

use std::collections::HashMap;

use crate::hidden;
use crate::model::Id;

/// The scheme a resolved reference becomes, and the one the renderer intercepts to open a card.
pub const SCHEME: &str = "typr-card:";

const OPEN: &str = "[[";
const CLOSE: &str = "]]";

/// One reference as it sits in the text.
struct Span<'a> {
    start: usize,
    end: usize,
    name: &'a str,
}

/// The names `text` refers to, outside fenced blocks and inline code.
///
/// Order is the order they appear in, and a name repeated twice appears twice: the caller decides
/// whether that is one row or two.
pub fn links(text: &str) -> Vec<&str> {
    spans(text).into_iter().map(|span| span.name).collect()
}

/// `text` with every reference the map resolves turned into a markdown link.
///
/// The label is the name as written, and the target is `typr-card:<id>`; a name absent from the map
/// is left as its `[[…]]` text so the renderer can show it as unresolved.
pub fn rewrite(text: &str, resolved: &HashMap<String, Id>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    for span in spans(text) {
        let Some(id) = resolved.get(span.name) else {
            continue;
        };
        out.push_str(&text[at..span.start]);
        out.push('[');
        out.push_str(span.name);
        out.push_str("](");
        out.push_str(SCHEME);
        out.push_str(&id.to_string());
        out.push(')');
        at = span.end;
    }
    out.push_str(&text[at..]);
    out
}

/// The reference spans in `text`.
///
/// A reference is one line and holds no brackets: `[` and `]` inside it would end the link the
/// render pass builds, so `[[a [b] c]]` is prose about brackets rather than a reference.
fn spans(text: &str) -> Vec<Span<'_>> {
    let code = hidden::code_ranges(text);
    let mut found = Vec::new();
    let mut at = 0;
    while let Some(open) = text[at..].find(OPEN).map(|offset| at + offset) {
        let after = open + OPEN.len();
        if hidden::inside(&code, open) {
            at = after;
            continue;
        }
        let Some(close) = text[after..].find(CLOSE).map(|offset| after + offset) else {
            break;
        };
        let inner = &text[after..close];
        let usable = !inner.trim().is_empty()
            && !inner.contains('\n')
            && !inner.contains(['[', ']'])
            && !hidden::inside(&code, close);
        if !usable {
            at = after;
            continue;
        }
        let end = close + CLOSE.len();
        found.push(Span {
            start: open,
            end,
            name: inner.trim(),
        });
        at = end;
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_finds_references_and_their_names() {
        assert_eq!(
            links("see [[Krebs cycle]] and [[Glycolysis]]"),
            vec!["Krebs cycle", "Glycolysis"]
        );
        assert_eq!(
            links("[[ padded ]]"),
            vec!["padded"],
            "the name is what resolves, not the spacing around it"
        );
        assert!(links("no references here").is_empty());
        assert!(
            links("[[open but never closed").is_empty(),
            "an unterminated `[[` is text"
        );
    }

    #[test]
    fn links_ignores_code() {
        let fenced = "```\n[[fenced]]\n```\n[[real]]";
        assert_eq!(links(fenced), vec!["real"]);

        let inline = "type `[[inline]]` and see [[real]]";
        assert_eq!(links(inline), vec!["real"]);

        let tilde = "~~~\n[[tilde]]\n~~~";
        assert!(links(tilde).is_empty(), "a tilde fence is a fence too");
    }

    #[test]
    fn links_rejects_what_cannot_be_a_link() {
        assert!(links("[[]]").is_empty(), "an empty reference is no name");
        assert!(
            links("[[a [b] c]]").is_empty(),
            "a bracket inside would end the link the render pass builds"
        );
        assert!(
            links("[[one\ntwo]]").is_empty(),
            "a name is one line because the list shows it on one line"
        );
    }

    #[test]
    fn rewrite_turns_resolved_references_into_card_links() {
        let resolved = HashMap::from([("Krebs cycle".to_string(), 7)]);
        assert_eq!(
            rewrite("see [[Krebs cycle]]", &resolved),
            "see [Krebs cycle](typr-card:7)"
        );
    }

    #[test]
    fn rewrite_leaves_unresolved_text_alone() {
        let resolved = HashMap::from([("Krebs cycle".to_string(), 7)]);
        assert_eq!(
            rewrite("[[Krebs cycle]] and [[Glycolysis]]", &resolved),
            "[Krebs cycle](typr-card:7) and [[Glycolysis]]",
            "an unresolved reference keeps the author's words, exactly"
        );
        assert_eq!(
            rewrite("`[[Krebs cycle]]`", &resolved),
            "`[[Krebs cycle]]`",
            "code is not rewritten"
        );
    }
}
