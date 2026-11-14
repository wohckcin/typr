//! A card's name: the short line a list shows and a reference resolves against.
//!
//! A name is a card's first side read down to one line, or what its name template renders when the
//! card has one. It is stored rather than derived on read so `[[name]]` resolves with one index
//! lookup and so the name a reference points at does not move when the card's fields change.

use crate::store::folded;
use crate::template::{self, Context, Fields};

/// The longest a name may be, in characters.
const LONGEST: usize = 120;

/// The name a card's text and name template imply.
///
/// A template that renders nothing — every field it names is empty — falls back to the first side,
/// so a card never loses its name because a field was cleared.
pub fn for_card(first_side: &str, template: Option<&str>, fields: &Fields<'_>) -> String {
    let first = line(first_side);
    let Some(template) = template else {
        return first;
    };
    // `<<Name>>` in a name template means the first side, and the first side is one line: a name
    // never wraps.
    let rendered = template::render(
        template,
        fields,
        &Context {
            name: Some(&first),
            deck: None,
        },
    );
    let named = line(&rendered);
    if named.is_empty() { first } else { named }
}

/// `name`, or `name` with the lowest free " (n)" suffix when another card already answers to it.
///
/// Comparison folds case and accents, the rule references resolve with, so two names the store
/// cannot tell apart never both exist in one deck.
pub fn unique(name: &str, taken: &[&str]) -> String {
    // An unnamed card is not given " (2)": an empty name is the absence of a name, not a name.
    if name.is_empty() {
        return String::new();
    }
    let wanted = folded(name);
    if !taken.iter().any(|other| folded(other) == wanted) {
        return name.to_string();
    }
    let mut suffix = 2;
    loop {
        let candidate = format!("{name} ({suffix})");
        if !taken.iter().any(|other| folded(other) == folded(&candidate)) {
            return candidate;
        }
        suffix += 1;
    }
}

/// The first non-empty line, trimmed and cut to [`LONGEST`].
fn line(text: &str) -> String {
    let line = text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("");
    cut(line)
}

/// `line` cut at a word boundary, with no marker: a name is the literal text `[[…]]` resolves
/// against, so it must not grow characters its author never typed.
fn cut(line: &str) -> String {
    if line.chars().count() <= LONGEST {
        return line.to_string();
    }
    let end = line
        .char_indices()
        .nth(LONGEST)
        .map_or(line.len(), |(at, _)| at);
    let head = &line[..end];
    // One word longer than the limit is cut where it must be rather than dropped.
    match head.rfind(char::is_whitespace) {
        Some(at) if at > 0 => head[..at].trim_end().to_string(),
        _ => head.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect()
    }

    #[test]
    fn a_card_without_a_template_is_named_for_its_first_side() {
        assert_eq!(
            for_card(
                "Krebs cycle\n---\nThe second stage\n",
                None,
                &Fields::new(&[])
            ),
            "Krebs cycle"
        );
    }

    #[test]
    fn a_name_skips_a_blank_line_and_trims() {
        assert_eq!(
            for_card("\n  \n  hablar  \n\n---\nhablo", None, &Fields::new(&[])),
            "hablar"
        );
    }

    #[test]
    fn a_template_names_the_card_from_its_fields() {
        let pairs = fields(&[("Word", "hablar"), ("Kind", "verb")]);
        assert_eq!(
            for_card(
                "hablar\n---\nhablo",
                Some("<<Word>> (<<Kind>>)"),
                &Fields::new(&pairs)
            ),
            "hablar (verb)"
        );
    }

    #[test]
    fn a_name_template_may_read_the_first_side() {
        let pairs = fields(&[("Kind", "verb")]);
        assert_eq!(
            for_card(
                "hablar\n---\nhablo",
                Some("<<Name>> (<<Kind>>)"),
                &Fields::new(&pairs)
            ),
            "hablar (verb)"
        );
    }

    #[test]
    fn a_template_that_renders_nothing_falls_back_to_the_first_side() {
        let pairs = fields(&[("Kind", "")]);
        assert_eq!(
            for_card("hablar\n---\nhablo", Some("<<Kind>>"), &Fields::new(&pairs)),
            "hablar"
        );
    }

    #[test]
    fn a_name_is_cut_at_a_word_boundary() {
        let long = "alpha ".repeat(60);
        let name = for_card(&long, None, &Fields::new(&[]));
        assert!(name.chars().count() <= LONGEST);
        assert!(name.ends_with("alpha"), "{name:?}");
        assert!(!name.ends_with(' '));
    }

    #[test]
    fn a_word_longer_than_the_limit_is_cut_at_the_limit() {
        let long = "x".repeat(LONGEST * 2);
        assert_eq!(
            for_card(&long, None, &Fields::new(&[])).chars().count(),
            LONGEST
        );
    }

    #[test]
    fn a_second_card_sharing_a_name_is_suffixed() {
        assert_eq!(unique("Krebs cycle", &["krebs CYCLE"]), "Krebs cycle (2)");
        assert_eq!(
            unique("Krebs cycle", &["Krebs cycle", "Krebs cycle (2)"]),
            "Krebs cycle (3)"
        );
    }

    #[test]
    fn a_name_no_one_else_has_is_left_alone() {
        assert_eq!(unique("Krebs cycle", &["Oxidation"]), "Krebs cycle");
    }

    #[test]
    fn an_unnamed_card_stays_unnamed() {
        assert_eq!(unique("", &[""]), "");
        assert_eq!(for_card("", None, &Fields::new(&[])), "");
    }
}
