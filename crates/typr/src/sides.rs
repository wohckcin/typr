//! Splitting a card into the sides review reveals in order.
//!
//! A line separates sides when, ignoring surrounding spaces and a trailing carriage return, its
//! content is exactly `---` — and never inside a fenced code block, so a fence can hold a rule.
//! `***` and `___` stay available to the user as thematic breaks.

const SEPARATOR: &str = "---";

/// The sides of `markdown`, in order, with sides that are empty after trimming dropped.
pub fn split(markdown: &str) -> Vec<&str> {
    let mut sides = Vec::new();
    let mut start = 0;
    let mut offset = 0;
    let mut fence: Option<&str> = None;

    for line in markdown.split_inclusive('\n') {
        let content = line.trim_end_matches(['\n', '\r']);
        let trimmed = content.trim();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            let marker = if trimmed.starts_with("```") {
                "```"
            } else {
                "~~~"
            };
            match fence {
                None => fence = Some(marker),
                Some(open) if open == marker => fence = None,
                Some(_) => {}
            }
        } else if fence.is_none() && trimmed == SEPARATOR {
            sides.push(&markdown[start..offset]);
            start = offset + line.len();
        }
        offset += line.len();
    }
    sides.push(&markdown[start..]);

    sides
        .into_iter()
        .filter(|side| !side.trim().is_empty())
        .collect()
}

#[cfg(test)]
mod tests;
