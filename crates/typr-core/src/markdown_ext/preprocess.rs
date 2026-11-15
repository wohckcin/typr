//! The constructs the markdown parser cannot see, written into forms it can.
//!
//! A card written for Mochi uses syntax `rune-kit`'s parser has no event for: `==highlight==`,
//! `{base}(reading)`, `$math$`, `20^th^`, `H~2~0` and definition lines. The renderer exposes no inline
//! hook — its style fields are `pub(super)` and its events are drawn inside a paragraph — so the only
//! way to keep those words on the page is to rewrite them into syntax it does render, before it looks.
//!
//! Nothing here invents text: a construct becomes the same characters in another shape, or it is left
//! exactly as written. A rewrite that fired inside a code span or a fence would corrupt what the author
//! meant literally, so the scan tracks both.
//!
//! Running the transform twice changes nothing, which the tests assert: the second pass sees only forms
//! it does not rewrite.

use std::borrow::Cow;

/// Rewrite the constructs the parser cannot see. The output is what the renderer is handed.
pub fn rewrite(text: &str) -> Cow<'_, str> {
    if !may_hold_a_construct(text) {
        return Cow::Borrowed(text);
    }
    let lines: Vec<&str> = text.split('\n').collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut fence: Option<(char, usize)> = None;
    let mut definition_list = false;
    let mut index = 0;

    while index < lines.len() {
        let line = lines[index];
        index += 1;
        if let Some((marker, width)) = fence {
            out.push(line.to_string());
            let trimmed = line.trim_start();
            let run = trimmed.chars().take_while(|c| *c == marker).count();
            if run >= width {
                fence = None;
            }
            continue;
        }
        if let Some(open) = fence_open(line) {
            fence = Some(open);
            out.push(line.to_string());
            continue;
        }
        // Display math is a block of its own: a fenced block keeps it on its own line, set apart, and
        // the dollars on the page.
        if starts_display_math(line) {
            out.push("```text".to_string());
            out.push(line.trim().to_string());
            while index < lines.len() {
                let part = lines[index];
                index += 1;
                out.push(part.to_string());
                if part.trim_end().ends_with("$$") && part.trim() != "$$" {
                    break;
                }
                if part.trim() == "$$" {
                    break;
                }
            }
            out.push("```".to_string());
            continue;
        }
        // A definition line belongs to the term above it: `: text` after a plain line.
        if let Some(definition) = definition_of(line) {
            // Every definition line after the first belongs to the same term.
            if definition_list {
                out.push(format!("- {definition}"));
                continue;
            }
            if let Some(term) = out.last_mut()
                && is_definition_term(term)
            {
                *term = format!("**{}**", term.trim());
                definition_list = true;
                out.push(format!("- {definition}"));
                continue;
            }
        }
        definition_list = false;
        out.push(inline(line));
    }

    if out.join("\n") == text {
        Cow::Borrowed(text)
    } else {
        Cow::Owned(out.join("\n"))
    }
}

/// A line that opens display math: `$$` alone, or `$$` with the formula after it.
fn starts_display_math(line: &str) -> bool {
    let trimmed = line.trim();
    // `$$x$$` on one line closes itself, and the loop below takes that line as both halves.
    trimmed.starts_with("$$")
}

/// Cheap answer to "could this text hold one of the constructs", so a card of plain prose is untouched.
fn may_hold_a_construct(text: &str) -> bool {
    text.contains('=')
        || text.contains('{')
        || text.contains('$')
        || text.contains('^')
        || text.contains('~')
        || text.contains("\n: ")
}

/// The marker and its width when a line opens a fenced block.
fn fence_open(line: &str) -> Option<(char, usize)> {
    let trimmed = line.trim_start();
    for marker in ['`', '~'] {
        let width = trimmed.chars().take_while(|c| *c == marker).count();
        if width >= 3 {
            return Some((marker, width));
        }
    }
    None
}

/// `: definition` on its own line, as a definition list writes it.
fn definition_of(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let rest = trimmed.strip_prefix(": ")?;
    (!rest.trim().is_empty()).then_some(rest)
}

/// A line a definition can attach to: text, not a list, heading, quote or another definition.
fn is_definition_term(line: &str) -> bool {
    let trimmed = line.trim();
    !trimmed.is_empty()
        && !trimmed.starts_with('#')
        && !trimmed.starts_with('>')
        && !trimmed.starts_with('-')
        && !trimmed.starts_with('*')
        && !trimmed.starts_with('+')
        && !trimmed.starts_with("```")
        && !trimmed.starts_with("~~~")
        && !trimmed.starts_with('|')
        && definition_of(trimmed).is_none()
}

/// Rewrite the constructs that live inside one line, leaving code spans alone.
fn inline(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len());
    let mut i = 0;
    while i < chars.len() {
        // An inline code span runs to the matching run of backticks and is copied whole.
        if chars[i] == '`' {
            let width = chars[i..].iter().take_while(|c| **c == '`').count();
            if let Some(end) = code_end(&chars, i + width, width) {
                out.extend(&chars[i..=end]);
                i = end + 1;
                continue;
            }
        }
        if let Some((written, length)) = construct(&chars, i) {
            out.push_str(&written);
            i += length;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// The index of the closing backtick of a code span that opened at `from - width`.
fn code_end(chars: &[char], from: usize, width: usize) -> Option<usize> {
    let mut i = from;
    while i < chars.len() {
        if chars[i] == '`' {
            let run = chars[i..].iter().take_while(|c| **c == '`').count();
            if run == width {
                return Some(i + run - 1);
            }
            i += run;
            continue;
        }
        i += 1;
    }
    None
}

/// One construct starting at `at`, as the text to write and how many characters it consumed.
fn construct(chars: &[char], at: usize) -> Option<(String, usize)> {
    highlight(chars, at)
        .or_else(|| ruby(chars, at))
        .or_else(|| math(chars, at))
        .or_else(|| superscript(chars, at))
        .or_else(|| subscript(chars, at))
}

/// `==text==` becomes bold: the renderer has no inline background, and bold is what the design records
/// as the fallback. The markers are gone, which is what a reader sees.
fn highlight(chars: &[char], at: usize) -> Option<(String, usize)> {
    if chars.get(at) != Some(&'=') || chars.get(at + 1) != Some(&'=') {
        return None;
    }
    let body_at = at + 2;
    let mut i = body_at;
    while i + 1 < chars.len() {
        if chars[i] == '=' && chars[i + 1] == '=' {
            let body: String = chars[body_at..i].iter().collect();
            if body.trim().is_empty() {
                return None;
            }
            return Some((format!("**{}**", body.trim()), i + 2 - at));
        }
        i += 1;
    }
    None
}

/// `{base}(reading)` becomes `base (reading)`: both are kept, in reading order.
fn ruby(chars: &[char], at: usize) -> Option<(String, usize)> {
    if chars.get(at) != Some(&'{') {
        return None;
    }
    let mut close = at + 1;
    while close < chars.len() && chars[close] != '}' {
        if chars[close] == '{' || chars[close] == '\n' {
            return None;
        }
        close += 1;
    }
    if chars.get(close) != Some(&'}') || chars.get(close + 1) != Some(&'(') {
        return None;
    }
    let mut end = close + 2;
    while end < chars.len() && chars[end] != ')' {
        if chars[end] == '(' {
            return None;
        }
        end += 1;
    }
    if chars.get(end) != Some(&')') {
        return None;
    }
    let base: String = chars[at + 1..close].iter().collect();
    let reading: String = chars[close + 2..end].iter().collect();
    if base.trim().is_empty() || reading.trim().is_empty() {
        return None;
    }
    Some((
        format!("{} ({})", base.trim(), reading.trim()),
        end + 1 - at,
    ))
}

/// `$x$` becomes an inline code span: monospace, set apart, and the dollars stay on the page.
fn math(chars: &[char], at: usize) -> Option<(String, usize)> {
    if chars.get(at) != Some(&'$') {
        return None;
    }
    // `$$x$$` written inside a sentence is one run, not two empty inline spans.
    if chars.get(at + 1) == Some(&'$') {
        let mut end = at + 2;
        while end + 1 < chars.len() && !(chars[end] == '$' && chars[end + 1] == '$') {
            end += 1;
        }
        if end + 1 >= chars.len() || end == at + 2 {
            return None;
        }
        let body: String = chars[at..=end + 1].iter().collect();
        if body.contains('`') {
            return None;
        }
        return Some((format!("`{body}`"), end + 2 - at));
    }
    let mut end = at + 1;
    while end < chars.len() && chars[end] != '$' {
        end += 1;
    }
    if chars.get(end) != Some(&'$') || end == at + 1 {
        return None;
    }
    let body: String = chars[at..=end].iter().collect();
    // A span that already holds a backtick would break the one it is written into.
    if body.contains('`') {
        return None;
    }
    Some((format!("`{body}`"), end + 1 - at))
}

/// `20^th^` becomes `20ᵗʰ` when every character has a raised form, otherwise it is left as written.
fn superscript(chars: &[char], at: usize) -> Option<(String, usize)> {
    raised_or_lowered(chars, at, '^', '^', superscript_of)
}

/// `H~2~0` becomes `H₂0`. A doubled tilde is strikethrough and belongs to the parser.
fn subscript(chars: &[char], at: usize) -> Option<(String, usize)> {
    raised_or_lowered(chars, at, '~', '~', subscript_of)
}

fn raised_or_lowered(
    chars: &[char],
    at: usize,
    open: char,
    close: char,
    map: fn(char) -> Option<char>,
) -> Option<(String, usize)> {
    if chars.get(at) != Some(&open) {
        return None;
    }
    // A doubled marker is another construct's.
    if chars.get(at + 1) == Some(&open) {
        return None;
    }
    let mut end = at + 1;
    while end < chars.len() && chars[end] != close {
        end += 1;
    }
    if chars.get(end) != Some(&close) || end == at + 1 || chars.get(end + 1) == Some(&close) {
        return None;
    }
    let body = &chars[at + 1..end];
    let mut mapped = String::with_capacity(body.len());
    for c in body {
        mapped.push(map(*c)?);
    }
    Some((mapped, end + 1 - at))
}

/// The raised form of a character, when Unicode has one.
fn superscript_of(c: char) -> Option<char> {
    Some(match c {
        '0'..='9' => ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'][c as usize - '0' as usize],
        '+' => '⁺',
        '-' | '−' => '⁻',
        '=' => '⁼',
        '(' => '⁽',
        ')' => '⁾',
        'a' => 'ᵃ',
        'b' => 'ᵇ',
        'c' => 'ᶜ',
        'd' => 'ᵈ',
        'e' => 'ᵉ',
        'f' => 'ᶠ',
        'g' => 'ᵍ',
        'h' => 'ʰ',
        'i' => 'ⁱ',
        'j' => 'ʲ',
        'k' => 'ᵏ',
        'l' => 'ˡ',
        'm' => 'ᵐ',
        'n' => 'ⁿ',
        'o' => 'ᵒ',
        'p' => 'ᵖ',
        'r' => 'ʳ',
        's' => 'ˢ',
        't' => 'ᵗ',
        'u' => 'ᵘ',
        'v' => 'ᵛ',
        'w' => 'ʷ',
        'x' => 'ˣ',
        'y' => 'ʸ',
        'z' => 'ᶻ',
        _ => return None,
    })
}

/// The lowered form of a character, when Unicode has one.
fn subscript_of(c: char) -> Option<char> {
    Some(match c {
        '0'..='9' => ['₀', '₁', '₂', '₃', '₄', '₅', '₆', '₇', '₈', '₉'][c as usize - '0' as usize],
        '+' => '₊',
        '-' | '−' => '₋',
        '=' => '₌',
        '(' => '₍',
        ')' => '₎',
        'a' => 'ₐ',
        'e' => 'ₑ',
        'h' => 'ₕ',
        'i' => 'ᵢ',
        'j' => 'ⱼ',
        'k' => 'ₖ',
        'l' => 'ₗ',
        'm' => 'ₘ',
        'n' => 'ₙ',
        'o' => 'ₒ',
        'p' => 'ₚ',
        'r' => 'ᵣ',
        's' => 'ₛ',
        't' => 'ₜ',
        'u' => 'ᵤ',
        'v' => 'ᵥ',
        'x' => 'ₓ',
        _ => return None,
    })
}

#[cfg(test)]
#[path = "preprocess/tests.rs"]
mod tests;
