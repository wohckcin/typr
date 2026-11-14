//! Turning a template and a card's fields into the card's text.
//!
//! A template is markdown with three extra forms:
//!
//! - `<<Field>>` is replaced by that field's value, or by nothing when it has none.
//! - `{{#Field}}…{{/Field}}` is kept only when that field has a non-empty value.
//! - everything else, including `{{1::hidden}}` and any other `{{…}}` the app does not own, is
//!   copied through untouched, so a template can carry hidden text.
//!
//! Rendering is pure: no I/O, no clock, no knowledge of the store. The store, the editor's preview
//! and its tests all call this one function, which is what keeps a card's stored text, its preview
//! and its sides from drifting apart.
//!
//! Two sigils rather than one: the cloze syntax already spends `{{…}}`, and a template that had to
//! share it could not contain hidden text.

/// A field whose value the card supplies rather than the user. These are never stored, so a card
/// renamed by editing its first side needs no migration.
pub const DYNAMIC: [&str; 2] = ["Name", "Deck"];

/// Whether `name` is computed from the card. A user value of that name is ignored, never shadowing
/// the computed one.
pub fn is_dynamic(name: &str) -> bool {
    DYNAMIC.contains(&name)
}

/// A card's stored fields, in the order the store lists them.
#[derive(Clone, Copy, Debug)]
pub struct Fields<'a>(&'a [(String, String)]);

impl<'a> Fields<'a> {
    pub fn new(pairs: &'a [(String, String)]) -> Self {
        Self(pairs)
    }

    /// The value of a field the user typed. A dynamic name answers `None` here even when a row
    /// exists for it: the card computes those.
    pub fn get(&self, name: &str) -> Option<&'a str> {
        if is_dynamic(name) {
            return None;
        }
        self.0
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

/// The values a card supplies itself.
#[derive(Clone, Copy, Debug, Default)]
pub struct Context<'a> {
    /// The card's first side, rendered before this pass.
    pub name: Option<&'a str>,
    /// The name of the deck the card sits in.
    pub deck: Option<&'a str>,
}

/// The card's text: `template` with its placeholders filled and its empty sections dropped.
pub fn render(template: &str, fields: &Fields<'_>, context: &Context<'_>) -> String {
    let mut out = String::with_capacity(template.len());
    write_range(template, fields, context, &mut out);
    out
}

/// Render a card from a template, its fields and its deck's name.
///
/// `<<Name>>` is the card's first side, and a first side can itself hold a field, so the name is
/// read from a render with nothing computed in it. One extra pass, and no fixed-point chase: a
/// template whose first side reads `<<Name>>` renders it empty, like any other unfilled field.
pub fn render_card(template: &str, fields: &[(String, String)], deck: Option<&str>) -> String {
    let fields = Fields::new(fields);
    let without_name = render(template, &fields, &Context { name: None, deck });
    let name = crate::sides::split(&without_name)
        .first()
        .map(|side| side.trim())
        .unwrap_or_default();
    render(
        template,
        &fields,
        &Context {
            name: Some(name),
            deck,
        },
    )
}

/// The value the card gives a computed field, or `None` when the name is not computed.
///
/// `Name` is the card's first side as read without it, which is the same value `render_card` fills
/// in, so a table showing it and a card rendering it cannot disagree.
pub fn computed(
    name: &str,
    template: &str,
    fields: &[(String, String)],
    deck: Option<&str>,
) -> Option<String> {
    if !is_dynamic(name) {
        return None;
    }
    if name == "Deck" {
        return Some(deck.unwrap_or_default().to_string());
    }
    let fields = Fields::new(fields);
    let without_name = render(template, &fields, &Context { name: None, deck });
    let first = crate::sides::split(&without_name)
        .first()
        .map(|side| side.trim())
        .unwrap_or_default()
        .to_string();
    Some(first)
}

/// The computed fields a template uses, in `DYNAMIC` order: a name the template never mentions is
/// not worth a row.
pub fn computed_used(template: &str) -> Vec<&'static str> {
    DYNAMIC
        .into_iter()
        .filter(|name| {
            template.contains(&format!("<<{name}>>"))
                || template.contains(&format!("{{{{#{name}}}}}"))
        })
        .collect()
}

/// The field names a template asks the user to fill, in the order they first appear.
///
/// A computed name is left out: `Name` and `Deck` are rendered from the card, so a table that let
/// someone type over them would be lying. A name repeated in the template appears once.
pub fn placeholders(template: &str) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    let mut cursor = 0;
    while let Some((at, token)) = next_token(template, cursor) {
        let rest = &template[at..];
        match token {
            Token::Placeholder => {
                let Some(end) = rest.find(">>") else {
                    break;
                };
                let name = rest[2..end].trim();
                if !name.is_empty() && !is_dynamic(name) && !names.iter().any(|own| own == name) {
                    names.push(name.to_string());
                }
                cursor = at + end + 2;
            }
            Token::SectionOpen => {
                let Some((name, body)) = section_name(rest, 3) else {
                    break;
                };
                if !is_dynamic(&name) && !names.contains(&name) {
                    names.push(name);
                }
                cursor = at + body;
            }
            _ => cursor = at + 2,
        }
    }
    names
}

/// What a marker at a position is.
enum Token {
    Placeholder,
    SectionOpen,
    SectionClose,
    Other,
}

/// The value a name answers with: the card's own first for a dynamic name, the field's otherwise.
fn value<'v>(name: &str, fields: &Fields<'v>, context: &Context<'v>) -> Option<&'v str> {
    match name {
        "Name" => context.name.filter(|value| !value.is_empty()),
        "Deck" => context.deck.filter(|value| !value.is_empty()),
        other => fields.get(other).filter(|value| !value.is_empty()),
    }
}

/// Render `text`, which may be a template or the body of a section.
fn write_range(text: &str, fields: &Fields<'_>, context: &Context<'_>, out: &mut String) {
    let mut cursor = 0;
    while cursor < text.len() {
        let Some((at, token)) = next_token(text, cursor) else {
            out.push_str(&text[cursor..]);
            return;
        };
        out.push_str(&text[cursor..at]);
        match token {
            Token::Placeholder => match text[at + 2..].find(">>") {
                Some(end) => {
                    let name = text[at + 2..at + 2 + end].trim();
                    // A placeholder with no value renders as nothing, which is what makes a
                    // template usable before every field is filled.
                    out.push_str(value(name, fields, context).unwrap_or_default());
                    cursor = at + 2 + end + 2;
                }
                // Never closed: the rest of the card is text, not a name.
                None => {
                    out.push_str("<<");
                    cursor = at + 2;
                }
            },
            Token::SectionOpen => match section_name(text, at + 3) {
                Some((name, body)) => match find_close(text, body, &name) {
                    Some((body_end, close_end)) => {
                        let inner = &text[body..body_end];
                        if value(&name, fields, context).is_some() {
                            write_range(inner, fields, context, out);
                        } else if inner.ends_with('\n') && text[close_end..].starts_with('\n') {
                            // The block owned its own line; leaving both newlines would open a
                            // gap where the section was.
                            cursor = close_end + 1;
                            continue;
                        }
                        cursor = close_end;
                    }
                    // An unclosed section is a typo, not an instruction to drop the rest of the
                    // card, so the marker stays as text.
                    None => {
                        out.push_str(&text[at..body]);
                        cursor = body;
                    }
                },
                None => {
                    out.push_str("{{#");
                    cursor = at + 3;
                }
            },
            // A close marker with nothing open is text, not an error.
            Token::SectionClose => {
                out.push_str("{{/");
                cursor = at + 3;
            }
            Token::Other => match text[at + 2..].find("}}") {
                Some(end) => {
                    let end = at + 2 + end + 2;
                    out.push_str(&text[at..end]);
                    cursor = end;
                }
                None => {
                    out.push_str("{{");
                    cursor = at + 2;
                }
            },
        }
    }
}

/// The next marker at or after `from`, and what it is.
fn next_token(text: &str, from: usize) -> Option<(usize, Token)> {
    let rest = &text[from..];
    let double = rest.find("{{").map(|at| (from + at, 2));
    let pair = rest.find("<<").map(|at| (from + at, 2));
    let (at, _) = match (double, pair) {
        (Some(a), Some(b)) => {
            if a.0 <= b.0 {
                a
            } else {
                b
            }
        }
        (Some(a), None) | (None, Some(a)) => a,
        (None, None) => return None,
    };
    let token = if text[at..].starts_with("<<") {
        Token::Placeholder
    } else if text[at..].starts_with("{{#") {
        Token::SectionOpen
    } else if text[at..].starts_with("{{/") {
        Token::SectionClose
    } else {
        Token::Other
    };
    Some((at, token))
}

/// The name a section opens with, and where its body begins.
fn section_name(text: &str, from: usize) -> Option<(String, usize)> {
    let end = text[from..].find("}}")?;
    let name = text[from..from + end].trim().to_string();
    if name.is_empty() {
        return None;
    }
    Some((name, from + end + 2))
}

/// Where the section named `name` closes, as the end of its body and the end of its close marker.
///
/// Nesting is matched by name, so an inner `{{#X}}…{{/X}}` does not close an outer `{{/X}}`. Names
/// are compared after trimming, so `{{# X }}` opens and `{{/X}}` closes the same section.
fn find_close(text: &str, body: usize, name: &str) -> Option<(usize, usize)> {
    let mut depth = 1;
    let mut cursor = body;
    while let Some(offset) = text[cursor..].find("{{") {
        let at = cursor + offset;
        let marker = &text[at..];
        let close_end = marker.find("}}")?;
        let inner = marker[2..close_end].trim();
        let end = at + close_end + 2;
        let opened = inner.strip_prefix('#').map(str::trim) == Some(name);
        let closed = inner.strip_prefix('/').map(str::trim) == Some(name);
        if opened {
            depth += 1;
        } else if closed {
            depth -= 1;
            if depth == 0 {
                return Some((at, end));
            }
        }
        // Any other marker is stepped over whole, so its braces cannot be read as ours.
        cursor = end;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{Context, Fields, render};

    fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
        list.iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect()
    }

    fn plain(template: &str, fields: &[(String, String)]) -> String {
        render(template, &Fields::new(fields), &Context::default())
    }

    #[test]
    fn a_placeholder_is_replaced_by_its_value() {
        let fields = pairs(&[("Word", "gato"), ("Meaning", "cat")]);
        assert_eq!(
            plain("<<Word>>\n---\n<<Meaning>>", &fields),
            "gato\n---\ncat"
        );
    }

    #[test]
    fn a_placeholder_without_a_value_renders_as_nothing() {
        let fields = pairs(&[("Word", "gato")]);
        assert_eq!(plain("<<Word>> <<Example>>", &fields), "gato ");
    }

    #[test]
    fn a_placeholder_shares_its_line_with_text() {
        let fields = pairs(&[("Word", "gato")]);
        assert_eq!(plain("the word is <<Word>>.", &fields), "the word is gato.");
    }

    #[test]
    fn a_filled_section_is_kept() {
        let fields = pairs(&[("Example", "el gato duerme")]);
        assert_eq!(
            plain("{{#Example}}e.g. <<Example>>{{/Example}}", &fields),
            "e.g. el gato duerme"
        );
    }

    #[test]
    fn an_empty_section_is_dropped_with_its_own_line() {
        let template = "above\n{{#Example}}\nsee <<Example>>\n{{/Example}}\nbelow";
        assert_eq!(plain(template, &pairs(&[])), "above\nbelow");
    }

    #[test]
    fn an_empty_section_inline_leaves_the_surrounding_text() {
        let template = "a {{#Example}}x{{/Example}}b";
        assert_eq!(plain(template, &pairs(&[])), "a b");
    }

    #[test]
    fn an_unclosed_section_is_left_as_text() {
        let template = "above {{#Example}} and the rest";
        assert_eq!(plain(template, &pairs(&[])), template);
    }

    #[test]
    fn a_section_nests_by_name() {
        let fields = pairs(&[("Outer", "yes")]);
        let template = "{{#Outer}}a{{#Inner}}b{{/Inner}}c{{/Outer}}";
        assert_eq!(plain(template, &fields), "ac");
    }

    #[test]
    fn a_marker_name_is_read_with_its_spaces() {
        let fields = pairs(&[("Example", "kept")]);
        assert_eq!(
            plain("{{# Example }}<<Example>>{{/Example}}", &fields),
            "kept"
        );
    }

    #[test]
    fn a_dynamic_field_renders_from_the_card() {
        let fields = pairs(&[]);
        let context = Context {
            name: Some("gato"),
            deck: Some("Spanish"),
        };
        assert_eq!(
            render("<<Deck>>: <<Name>>", &Fields::new(&fields), &context),
            "Spanish: gato"
        );
    }

    #[test]
    fn a_field_named_like_a_dynamic_one_is_ignored() {
        let fields = pairs(&[("Deck", "not this one")]);
        let context = Context {
            deck: Some("Spanish"),
            ..Context::default()
        };
        assert_eq!(
            render("<<Deck>>", &Fields::new(&fields), &context),
            "Spanish"
        );
        // With nothing to compute from, the stored value stays unreachable rather than winning.
        assert_eq!(
            render("<<Deck>>", &Fields::new(&fields), &Context::default()),
            ""
        );
    }

    #[test]
    fn hidden_text_survives_a_template() {
        let template = "<<Word>> {{1::hidden}} {{2::also}}";
        assert_eq!(plain(template, &pairs(&[])), " {{1::hidden}} {{2::also}}");
    }

    #[test]
    fn an_unknown_marker_is_copied_as_text() {
        assert_eq!(plain("{{unclosed", &pairs(&[])), "{{unclosed");
        assert_eq!(plain("stray {{/End}}", &pairs(&[])), "stray {{/End}}");
    }
}

#[cfg(test)]
mod card_tests {
    use super::render_card;

    /// `<<Name>>` is the card's first side, and the first side is read from the text rendered
    /// without it — one extra pass, never a loop, so a template that uses `<<Name>>` still has
    /// one answer.
    #[test]
    fn a_name_in_the_first_side_is_read_from_the_card_without_it() {
        let fields = [("Word".to_string(), "gato".to_string())];
        assert_eq!(
            render_card("<<Word>> (<<Name>>)\n---\n<<Meaning>>", &fields, None),
            "gato (gato ())\n---\n",
            "the name is the first side of the card as it reads without it"
        );
    }

    #[test]
    fn a_name_below_the_first_side_is_that_side() {
        let fields = [("Word".to_string(), "gato".to_string())];
        assert_eq!(
            render_card("<<Word>>\n---\n<<Name>>, and it means cat", &fields, None),
            "gato\n---\ngato, and it means cat",
            "a name that does not change the first side is simply filled"
        );
    }
}

#[cfg(test)]
mod computed_tests {
    use super::{computed, computed_used};

    #[test]
    fn only_the_computed_names_a_template_asks_for_are_listed() {
        assert_eq!(
            computed_used("<<Word>>\n---\nfrom <<Deck>>"),
            vec!["Deck"],
            "a template that never says <<Name>> asks for no name row"
        );
        assert_eq!(
            computed_used("{{#Name}}<<Name>>{{/Name}}"),
            vec!["Name"],
            "a section counts as asking for it"
        );
        assert!(computed_used("<<Word>>").is_empty());
    }

    #[test]
    fn a_computed_value_is_the_one_the_card_renders() {
        let fields = [("Word".to_string(), "gato".to_string())];
        assert_eq!(
            computed("Name", "<<Word>>\n---\n<<Meaning>>", &fields, None).as_deref(),
            Some("gato"),
            "the name is the first side of the card"
        );
        assert_eq!(
            computed("Deck", "<<Word>>", &fields, Some("Spanish")).as_deref(),
            Some("Spanish")
        );
        assert_eq!(
            computed("Deck", "<<Word>>", &fields, None).as_deref(),
            Some(""),
            "an unknown deck renders empty rather than failing"
        );
        assert_eq!(computed("Word", "<<Word>>", &fields, None), None);
    }
}

#[cfg(test)]
mod placeholder_tests {
    use super::placeholders;

    #[test]
    fn the_table_lists_the_fields_the_template_asks_for() {
        assert_eq!(
            placeholders("<<Word>>\n---\n<<Meaning>>\n\n{{#Example}}e.g. <<Example>>{{/Example}}"),
            vec!["Word", "Meaning", "Example"],
            "each name once, in the order the template uses it"
        );
        assert_eq!(
            placeholders("<< Word >> and <<Word>> and {{# Word }}x{{/Word}}"),
            vec!["Word"],
            "trimmed and de-duplicated"
        );
        assert!(
            placeholders("<<Name>> on <<Deck>> with <<Word>>") == vec!["Word"],
            "a computed name cannot be typed into"
        );
        assert!(placeholders("{{1::hidden}} and <<").is_empty());
    }
}
