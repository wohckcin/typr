//! Ruby glosses and raised or lowered text, written into forms the renderer does draw.
//!
//! `{base}(reading)` becomes `base (reading)`; `20^th^` and `H~2~0` become Unicode raised and lowered
//! characters where every character has one, and are left as written otherwise. The toolkit has no
//! inline hook for stacked or shifted runs, so these are the only forms available here. Highlights,
//! math and definition lists are drawn by the vendored renderer and are not rewritten.
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
    let mut out: Vec<String> = Vec::new();
    let mut fence: Option<(char, usize)> = None;
    for line in text.split('\n') {
        if let Some((marker, width)) = fence {
            out.push(line.to_string());
            let run = line
                .trim_start()
                .chars()
                .take_while(|c| *c == marker)
                .count();
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
        out.push(inline(line));
    }
    let joined = out.join("\n");
    if joined == text {
        Cow::Borrowed(text)
    } else {
        Cow::Owned(joined)
    }
}

/// Cheap answer to "could this text hold one of the constructs", so a card of plain prose is untouched.
fn may_hold_a_construct(text: &str) -> bool {
    text.contains('{') || text.contains('^') || text.contains('~')
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
    ruby(chars, at)
        .or_else(|| superscript(chars, at))
        .or_else(|| subscript(chars, at))
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
