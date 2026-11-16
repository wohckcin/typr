//! Card embeds: `![[Name]]` written in a card, read outside fenced blocks and inline code.
//!
//! An embed is not a link. A link is rewritten where the library can resolve it and stays one
//! sentence; an embed is replaced by a fenced block, so the card keeps the sentence around it and
//! the renderer draws the named card's text in a block of its own. The block's body is the name,
//! not the text, because a body written into the card would be read back as part of the card:
//! `---` inside an embedded card would cut the embedding card into another side.
//!
//! Stored text keeps `![[Name]]`. The pre-pass runs where a card is drawn, next to the reference
//! pass, and answers a name with the card behind it, if any.

pub mod preprocess;
pub mod supported;

pub use preprocess::rewrite;

use crate::hidden;

/// The info string of the fenced block a resolved embed is written as.
pub const INFO: &str = "typr-embed";

/// The info string of the fenced block an embed whose name has no card behind it is written as.
///
/// The pre-pass is the one place that knows which names resolve, so it says so in the fence the
/// renderer reads. A target can still go away between the pass and the draw; the renderer asks
/// [`INFO`] blocks for their body and draws the same marker when the answer is empty.
pub const MISSING: &str = "typr-embed-missing";

const OPEN: &str = "![[";
const CLOSE: &str = "]]";

/// The names this text embeds, in the order they were written.
pub fn embeds(text: &str) -> Vec<String> {
    spans(text)
        .into_iter()
        .map(|span| span.name.to_string())
        .collect()
}

/// Rewrite every embed in `text` into a fenced block the renderer can recognise.
///
/// `resolve` answers a name with its card's text, or `None` when no card holds the name; that
/// answer only chooses the info string, because the block carries the name and not the text. The
/// renderer asks again for the text, so a card that changed since the pass is drawn as it is now.
pub fn expand(text: &str, resolve: impl Fn(&str) -> Option<String>) -> String {
    let spans = spans(text);
    if spans.is_empty() {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len() + spans.len() * 32);
    let mut at = 0;
    for span in spans {
        out.push_str(&text[at..span.start]);
        // A fenced block only starts a block at the beginning of a line, so an embed written in
        // the middle of a sentence gets one of its own.
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str("```");
        out.push_str(if resolve(span.name).is_some() {
            INFO
        } else {
            MISSING
        });
        out.push('\n');
        out.push_str(span.name);
        out.push_str("\n```\n");
        at = span.end;
    }
    out.push_str(&text[at..]);
    out
}

/// One embed, as written.
struct Span<'a> {
    /// The byte offset of the `!`.
    start: usize,
    /// The byte offset just past the closing `]]`.
    end: usize,
    /// The name between the brackets, trimmed.
    name: &'a str,
}

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
        // A name is one line, cannot hold the brackets that would end it early, and cannot be
        // empty: `![[ ]]` names nothing, so it stays the text it is.
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
    use crate::hidden::{self, Shape};

    /// The embeds' names, for the tests that only care which names are read.
    fn names(text: &str) -> Vec<String> {
        embeds(text)
    }

    #[test]
    fn an_embed_yields_the_name_it_written() {
        assert_eq!(names("see ![[Krebs cycle]] here"), vec!["Krebs cycle"]);
        assert_eq!(names("![[ first ]] ![[second]]"), vec!["first", "second"]);
        // A reference is not an embed: the `!` is what makes the difference.
        assert_eq!(names("see [[Krebs cycle]]"), Vec::<String>::new());
        assert_eq!(names("![[ ]]"), Vec::<String>::new());
        assert_eq!(names("![[not\none line]]"), Vec::<String>::new());
    }

    #[test]
    fn an_embed_inside_code_is_left_alone() {
        assert_eq!(names("`![[A]]`"), Vec::<String>::new());
        assert_eq!(names("```\n![[A]]\n```"), Vec::<String>::new());
        assert_eq!(names("~~~\n![[A]]\n~~~"), Vec::<String>::new());
        assert_eq!(
            names("`![[A]]` and ![[B]]"),
            vec!["B"],
            "the one outside the code still counts"
        );
    }

    #[test]
    fn an_embed_is_written_as_a_fence_with_its_name() {
        let drawn = expand("before ![[A]] after", |name| {
            (name == "A").then(|| "the body".to_string())
        });
        assert!(
            drawn.contains("```typr-embed\nA\n```"),
            "the block names the card it draws: {drawn:?}"
        );
        assert!(!drawn.contains("![[A]]"), "the embed is gone: {drawn:?}");
        assert!(
            drawn.starts_with("before "),
            "the sentence keeps its text: {drawn:?}"
        );
        assert!(
            drawn.contains("\n```typr-embed\nA\n```"),
            "a fence starts its own line: {drawn:?}"
        );
        assert!(
            drawn.ends_with(" after"),
            "the sentence survives: {drawn:?}"
        );
    }

    #[test]
    fn an_embed_with_no_card_is_written_as_a_missing_one() {
        let drawn = expand("see ![[Nope]]", |_| None);
        assert!(
            drawn.contains("```typr-embed-missing\nNope\n```"),
            "the renderer is told the name has no card: {drawn:?}"
        );
    }

    #[test]
    fn an_embed_does_not_add_a_side() {
        // The embedded card is three sides long. If its text were written into the embedding
        // card, those separators would cut it into four; the fence keeps it one side.
        let body = "one\n---\ntwo\n---\nthree";
        let drawn = expand("front\n\n![[Long]]\n", |_| Some(body.to_string()));
        let sides = match hidden::shape(&drawn) {
            Shape::Sides(sides) => sides,
            shape => panic!("expected sides, got {shape:?}"),
        };
        assert_eq!(sides.len(), 1, "the embed added a side: {sides:?}");
    }

    #[test]
    fn expanding_twice_changes_nothing() {
        let resolve = |_: &str| Some("body".to_string());
        let once = expand("a ![[A]] b", resolve);
        assert_eq!(expand(&once, resolve), once);
    }
}

/// A card's text as a name read aloud: image references become their alt text, emphasis and code
/// marks come out, and the lines join into one sentence. A tile says "ser means to be", not the
/// asterisks around it.
pub fn plain(markdown: &str) -> String {
    let mut out = String::with_capacity(markdown.len());
    let mut rest = markdown;
    while let Some(at) = rest.find("![") {
        out.push_str(&rest[..at]);
        let after = &rest[at + 2..];
        let Some(close) = after.find("](") else {
            out.push_str(rest);
            return out;
        };
        out.push_str(&after[..close]);
        let Some(end) = after[close + 2..].find(')') else {
            out.push_str(rest);
            return out;
        };
        rest = &after[close + 2 + end + 1..];
    }
    out.push_str(rest);
    out.replace("**", "")
        .replace(['`', '*'], "")
        .replace("\n", " ")
        .trim()
        .to_owned()
}

#[cfg(test)]
mod plain_tests {
    use super::plain;

    #[test]
    fn an_image_reads_as_its_alt_text() {
        assert_eq!(plain("see ![the cat](cat.png) here"), "see the cat here");
    }

    #[test]
    fn emphasis_and_code_marks_come_out() {
        assert_eq!(plain("**ser** means `to be`"), "ser means to be");
    }

    #[test]
    fn lines_join_into_one_sentence() {
        assert_eq!(plain("first line\nsecond line"), "first line second line");
    }
}
