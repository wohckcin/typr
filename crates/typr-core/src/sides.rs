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
mod tests {
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
}
