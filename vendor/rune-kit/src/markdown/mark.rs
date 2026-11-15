//! `==text==` highlights, which pulldown-cmark has no syntax for.
//!
//! The source is rewritten before it is parsed: each `==` that opens a highlight becomes [`OPEN`] and
//! each that closes it becomes [`CLOSE`]. Both are Unicode noncharacters, which conforming text does not
//! contain, so the parser can tell them from anything a writer typed. The parser turns a pair into a
//! [`Style::Mark`](super::Style::Mark) run and drops the characters from the text; a sentinel it does not
//! pair is written back as `==`, so nothing is lost.
//!
//! Code spans and fenced blocks are left as written. A highlight does not span a line.

use std::borrow::Cow;

/// The character that stands for an opening `==`.
pub(super) const OPEN: char = '\u{FDD0}';
/// The character that stands for a closing `==`.
pub(super) const CLOSE: char = '\u{FDD1}';

/// The source with its highlight markers rewritten, or the same string when there are none.
pub(super) fn rewrite_owned(src: String) -> String {
    let rewritten = match rewrite(&src) {
        Cow::Borrowed(_) => None,
        Cow::Owned(owned) => Some(owned),
    };
    rewritten.unwrap_or(src)
}

/// The source with each highlight's markers rewritten.
pub(super) fn rewrite(src: &str) -> Cow<'_, str> {
    if !src.contains("==") {
        return Cow::Borrowed(src);
    }
    let mut out = String::with_capacity(src.len() + 8);
    let mut changed = false;
    let mut fence: Option<(char, usize)> = None;
    for line in src.split_inclusive('\n') {
        if let Some((marker, width)) = fence {
            out.push_str(line);
            if closes_fence(line, marker, width) {
                fence = None;
            }
            continue;
        }
        if let Some(open) = fence_opens(line) {
            fence = Some(open);
            out.push_str(line);
            continue;
        }
        mark_line(line, &mut out, &mut changed);
    }
    if changed { Cow::Owned(out) } else { Cow::Borrowed(src) }
}

/// The marker and its width when a line opens a fenced block.
fn fence_opens(line: &str) -> Option<(char, usize)> {
    let trimmed = line.trim_start();
    ['`', '~'].into_iter().find_map(|marker| {
        let width = trimmed.chars().take_while(|c| *c == marker).count();
        (width >= 3).then_some((marker, width))
    })
}

fn closes_fence(line: &str, marker: char, width: usize) -> bool {
    let trimmed = line.trim_start();
    let run = trimmed.chars().take_while(|c| *c == marker).count();
    run >= width && trimmed[run * marker.len_utf8()..].trim().is_empty()
}

/// Rewrites one line's highlights, copying code spans whole.
fn mark_line(line: &str, out: &mut String, changed: &mut bool) {
    let mut rest = line;
    loop {
        let tick = rest.find('`');
        let equals = rest.find("==");
        match (tick, equals) {
            (Some(at), found) if found.is_none_or(|eq| at < eq) => {
                let width = rest[at..].chars().take_while(|c| *c == '`').count();
                let run = "`".repeat(width);
                let body = at + width;
                match rest[body..].find(&run) {
                    Some(end) => {
                        let stop = body + end + width;
                        out.push_str(&rest[..stop]);
                        rest = &rest[stop..];
                    }
                    None => {
                        out.push_str(rest);
                        return;
                    }
                }
            }
            (_, Some(at)) => {
                let body = at + 2;
                match closing_marker(&rest[body..]) {
                    Some(close) if is_highlight_body(&rest[body..body + close]) => {
                        out.push_str(&rest[..at]);
                        out.push(OPEN);
                        out.push_str(&rest[body..body + close]);
                        out.push(CLOSE);
                        *changed = true;
                        rest = &rest[body + close + 2..];
                    }
                    _ => {
                        out.push_str(&rest[..body]);
                        rest = &rest[body..];
                    }
                }
            }
            _ => {
                out.push_str(rest);
                return;
            }
        }
    }
}

/// The offset of the first `==` in `body` that is not inside a code span, or `None`.
///
/// A backtick run with no matching run after it is literal, as in the inline parser.
fn closing_marker(body: &str) -> Option<usize> {
    let mut i = 0;
    while i < body.len() {
        let rest = &body[i..];
        if rest.starts_with('`') {
            let width = rest.chars().take_while(|c| *c == '`').count();
            let run = "`".repeat(width);
            i += match rest[width..].find(&run) {
                Some(end) => width + end + width,
                None => width,
            };
            continue;
        }
        if rest.starts_with("==") {
            return Some(i);
        }
        i += rest.chars().next().map_or(1, char::len_utf8);
    }
    None
}

/// A highlight holds text: an empty or blank pair is two equals signs written as they are.
fn is_highlight_body(body: &str) -> bool {
    !body.trim().is_empty()
}

/// One piece of a parser text event that contains sentinels.
pub(super) enum Piece<'a> {
    Text(&'a str),
    Open,
    Close,
}

/// The text and sentinels of a parser text event, in order.
pub(super) fn pieces(text: &str) -> Vec<Piece<'_>> {
    let mut out = Vec::new();
    let mut start = 0;
    for (at, c) in text.char_indices() {
        if c == OPEN || c == CLOSE {
            if at > start {
                out.push(Piece::Text(&text[start..at]));
            }
            out.push(if c == OPEN { Piece::Open } else { Piece::Close });
            start = at + c.len_utf8();
        }
    }
    if start < text.len() {
        out.push(Piece::Text(&text[start..]));
    }
    out
}

/// The text with each sentinel written back as the `==` it stood for.
pub(super) fn restore(text: &str) -> Cow<'_, str> {
    if text.contains([OPEN, CLOSE]) {
        Cow::Owned(text.replace([OPEN, CLOSE], "=="))
    } else {
        Cow::Borrowed(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pair_of_markers_becomes_the_sentinels() {
        assert_eq!(rewrite("==the Krebs cycle==").as_ref(), format!("{OPEN}the Krebs cycle{CLOSE}"));
    }

    #[test]
    fn a_source_without_markers_is_borrowed() {
        assert!(matches!(rewrite("plain text"), Cow::Borrowed(_)));
    }

    #[test]
    fn equals_inside_a_code_span_are_left_alone() {
        let source = "the `a == b` comparison";
        assert_eq!(rewrite(source).as_ref(), source);
    }

    #[test]
    fn a_fence_is_left_alone() {
        let source = "```\na == b ==\n```\n";
        assert_eq!(rewrite(source).as_ref(), source);
    }

    #[test]
    fn a_blank_pair_is_not_a_highlight() {
        assert!(matches!(rewrite("a ==   == b"), Cow::Borrowed(_)));
    }

    #[test]
    fn an_unclosed_marker_stays_as_written() {
        assert_eq!(rewrite("a == b").as_ref(), "a == b");
    }

    #[test]
    fn a_closing_marker_inside_a_code_span_does_not_close() {
        assert_eq!(rewrite("==`a == b`==").as_ref(), format!("{OPEN}`a == b`{CLOSE}"));
    }

    #[test]
    fn restore_writes_a_sentinel_back_as_its_markers() {
        assert_eq!(restore(&format!("x{OPEN}y{CLOSE}")).as_ref(), "x==y==");
    }
}
