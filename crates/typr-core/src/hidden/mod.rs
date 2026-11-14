//! Reading hidden text out of a card.
//!
//! `{{text}}` hides `text`; `{{1::text}}` hides it as part of group 1. Every unnumbered span belongs to
//! one group, keyed `cloze:0`, so the plain form masks a whole sentence at once and numbering is how a
//! card asks for separate reviews.
//!
//! Spans are read outside fenced blocks and outside inline code, and a `{{` with no `}}` after it on the
//! same line stays ordinary text — a card that explains the syntax can show it.
//!
//! Hidden text wins over the `---` separator: the card reports one shape, so a card never mixes sides
//! with groups.

use std::ops::Range;

const OPEN: &str = "{{";
const CLOSE: &str = "}}";
const MASK: &str = "[ … ]";

/// How a card is cut into prompts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shape {
    /// One face per side, revealed in order. A card with fewer than two sides has no prompt.
    Sides(Vec<String>),
    /// One face per hidden-text group, every face read out of one text.
    ///
    /// The text is kept with its delimiters because both the masked and the shown form are read out of
    /// it; copying it once per group would only cost memory.
    Hidden { text: String, groups: Vec<u32> },
}

impl Shape {
    /// The prompt keys this shape asks the store for, in review order.
    pub fn prompt_keys(&self) -> Vec<String> {
        match self {
            Shape::Sides(sides) if sides.len() >= 2 => vec!["fwd".to_string()],
            Shape::Sides(_) => Vec::new(),
            Shape::Hidden { groups, .. } => groups.iter().map(|group| key(*group)).collect(),
        }
    }

    /// The number of prompts a card of this shape carries.
    pub fn prompt_count(&self) -> usize {
        self.prompt_keys().len()
    }

    /// Whether the card is also asked from its last side.
    ///
    /// Two sides can be read in either order, so the reverse prompt is written beside `fwd`. Hidden
    /// text is already the question, so a group never reverses.
    /// Every prompt key this card claims: the shape's own keys plus `rev` for a card with two or
    /// more sides. The store writes exactly these and the app admits exactly these, so the two
    /// cannot drift apart.
    pub fn claimed_keys(&self) -> Vec<String> {
        let mut keys = self.prompt_keys();
        if self.claims_reverse() {
            keys.push("rev".to_string());
        }
        keys
    }

    pub fn claims_reverse(&self) -> bool {
        matches!(self, Shape::Sides(sides) if sides.len() >= 2)
    }

    /// Whether the card holds hidden text rather than sides.
    pub fn hides_text(&self) -> bool {
        matches!(self, Shape::Hidden { .. })
    }
}

/// The prompt key for a group number.
pub fn key(group: u32) -> String {
    format!("cloze:{group}")
}

/// The group a prompt key names, if it names a hidden-text group.
pub fn group_of(key: &str) -> Option<u32> {
    key.strip_prefix("cloze:")?.parse().ok()
}

/// A hidden span: where it sits with its delimiters, where its content sits, and its group.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Span {
    group: u32,
    whole: Range<usize>,
    content: Range<usize>,
}

/// How a card is cut into prompts.
pub fn shape(markdown: &str) -> Shape {
    let spans = spans(markdown);
    if spans.is_empty() {
        return Shape::Sides(
            crate::sides::split(markdown)
                .into_iter()
                .map(str::to_string)
                .collect(),
        );
    }
    let mut groups: Vec<u32> = spans.iter().map(|span| span.group).collect();
    groups.sort_unstable();
    groups.dedup();
    Shape::Hidden {
        text: markdown.to_string(),
        groups,
    }
}

/// The text with every span of `key` replaced by a mask and every other group left visible.
///
/// A card without hidden text is returned unchanged, so a caller never has to ask which shape it has.
pub fn masked(text: &str, shape: &Shape, key: &str) -> String {
    match shape {
        Shape::Sides(_) => text.to_string(),
        Shape::Hidden { .. } => match group_of(key) {
            Some(group) => render(text, &spans(text), &|span| {
                (span.group == group).then(|| MASK.to_string())
            }),
            None => shown(text, shape),
        },
    }
}

/// The text with every hidden span masked and its group named, which is what the editor previews: a
/// writer sees which answers are hidden and which prompt each belongs to, and sees none of them.
pub fn masked_all(text: &str, shape: &Shape) -> String {
    match shape {
        Shape::Sides(_) => text.to_string(),
        Shape::Hidden { .. } => render(text, &spans(text), &|span| Some(label(span.group))),
    }
}

/// The text with its delimiters removed and every span visible.
pub fn shown(text: &str, shape: &Shape) -> String {
    match shape {
        Shape::Sides(_) => text.to_string(),
        Shape::Hidden { .. } => render(text, &spans(text), &|_| None),
    }
}

/// Copy `text` into its rendered form, masking the spans of `hiding` when one is given.
fn render(text: &str, spans: &[Span], hide: &dyn Fn(&Span) -> Option<String>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut copied = 0;
    for span in spans {
        out.push_str(&text[copied..span.whole.start]);
        match hide(span) {
            Some(mask) => out.push_str(&mask),
            None => out.push_str(&text[span.content.clone()]),
        }
        copied = span.whole.end;
    }
    out.push_str(&text[copied..]);
    out
}

/// What a masked span of `group` reads as. Only a numbered group is labelled, because the plain form
/// is one group and has nothing to tell apart.
fn label(group: u32) -> String {
    match group {
        0 => MASK.to_string(),
        n => format!("[{n} … ]"),
    }
}

/// Every hidden span in `markdown`, outside fenced blocks and inline code.
fn spans(markdown: &str) -> Vec<Span> {
    let code = code_ranges(markdown);
    let mut found = Vec::new();
    let mut at = 0;
    while let Some(open) = markdown[at..].find(OPEN).map(|offset| at + offset) {
        let after = open + OPEN.len();
        if inside(&code, open) {
            at = after;
            continue;
        }
        let Some(close) = markdown[after..].find(CLOSE).map(|offset| after + offset) else {
            break;
        };
        let inner = &markdown[after..close];
        // A span reaches the end of its line at the furthest; braces across lines are a typo, not a card.
        if inner.contains('\n') || inside(&code, close) {
            at = after;
            continue;
        }
        let (group, prefix) = group_prefix(inner);
        found.push(Span {
            group,
            whole: open..close + CLOSE.len(),
            content: after + prefix..close,
        });
        at = close + CLOSE.len();
    }
    found
}

/// The group an inner span names, and how many bytes its `n::` prefix takes.
///
/// `1::text` is group 1. Anything else — no prefix, a non-number, a zero — is the unnumbered group.
fn group_prefix(inner: &str) -> (u32, usize) {
    let Some((prefix, rest)) = inner.split_once("::") else {
        return (0, 0);
    };
    let digits = prefix.trim();
    let numbered = !digits.is_empty()
        && digits.bytes().all(|byte| byte.is_ascii_digit())
        && digits.parse::<u32>().is_ok_and(|group| group > 0);
    if numbered {
        (digits.parse().unwrap_or(0), inner.len() - rest.len())
    } else {
        (0, 0)
    }
}

/// The code regions of `markdown`: whole fenced blocks and inline backtick spans.
fn code_ranges(markdown: &str) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut offset = 0;
    let mut fence: Option<Fence> = None;
    for line in markdown.split_inclusive('\n') {
        let content = line.trim_end_matches(['\n', '\r']);
        let trimmed = content.trim();
        let marker = marker_of(trimmed);
        match (&fence, marker) {
            (None, Some(marker)) => {
                fence = Some(Fence {
                    marker,
                    start: offset,
                })
            }
            (Some(open), Some(marker)) if open.marker == marker => {
                ranges.push(open.start..offset + line.len());
                fence = None;
            }
            _ => {}
        }
        if fence.is_none() && marker.is_none() {
            ranges.extend(inline_code(content, offset));
        }
        offset += line.len();
    }
    if let Some(open) = fence {
        ranges.push(open.start..markdown.len());
    }
    ranges
}

struct Fence {
    marker: &'static str,
    start: usize,
}

fn marker_of(trimmed: &str) -> Option<&'static str> {
    if trimmed.starts_with("```") {
        Some("```")
    } else if trimmed.starts_with("~~~") {
        Some("~~~")
    } else {
        None
    }
}

/// The spans inside one line that backticks made code.
fn inline_code(line: &str, offset: usize) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut at = 0;
    while let Some(open) = line[at..].find('`').map(|found| at + found) {
        let ticks = line[open..].chars().take_while(|c| *c == '`').count();
        let Some(close) = find_run(line, open + ticks, ticks) else {
            break;
        };
        ranges.push(offset + open..offset + close + ticks);
        at = close + ticks;
    }
    ranges
}

/// The start of the next run of exactly `ticks` backticks at or after `from`.
fn find_run(line: &str, from: usize, ticks: usize) -> Option<usize> {
    let mut at = from;
    while let Some(found) = line[at..].find('`').map(|offset| at + offset) {
        let run = line[found..].chars().take_while(|c| *c == '`').count();
        if run == ticks {
            return Some(found);
        }
        at = found + run;
    }
    None
}

fn inside(ranges: &[Range<usize>], at: usize) -> bool {
    ranges
        .iter()
        .any(|range| range.start <= at && at < range.end)
}

#[cfg(test)]
mod tests;
