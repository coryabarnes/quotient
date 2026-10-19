//! Syntax highlighting and autocomplete for the editor. Kept free of GUI types
//! so it can be tested.

use std::ops::Range;

use crate::catalog::{self, ENTRIES};
use crate::eval::{CONSTANTS, FORMATS};
use crate::parser::is_keyword;
use crate::units;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    Plain,
    Number,
    /// A user variable.
    Variable,
    Function,
    Unit,
    Constant,
    Keyword,
    Operator,
    Comment,
    Text,
}

/// Classifies every byte range of `text`; ranges are contiguous and cover it all.
pub fn highlight(text: &str, variables: &[String]) -> Vec<(Range<usize>, Class)> {
    let mut spans: Vec<(Range<usize>, Class)> = Vec::new();
    let mut push = |range: Range<usize>, class: Class| {
        if range.is_empty() {
            return;
        }
        match spans.last_mut() {
            Some((last, c)) if *c == class && last.end == range.start => last.end = range.end,
            _ => spans.push((range, class)),
        }
    };
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let end_of = |i: usize| chars.get(i).map_or(text.len(), |(b, _)| *b);
    let mut i = 0;
    while i < chars.len() {
        let (start, c) = chars[i];
        // Comments run to the end of the line.
        if c == '/' && chars.get(i + 1).is_some_and(|(_, n)| *n == '/') {
            let mut j = i;
            while j < chars.len() && chars[j].1 != '\n' {
                j += 1;
            }
            push(start..end_of(j), Class::Comment);
            i = j;
            continue;
        }
        if c == '"' || c == '\'' {
            let mut j = i + 1;
            while j < chars.len() && chars[j].1 != c && chars[j].1 != '\n' {
                j += 1;
            }
            let j = (j + 1).min(chars.len());
            push(start..end_of(j), Class::Text);
            i = j;
            continue;
        }
        if c.is_ascii_digit() || (c == '.' && chars.get(i + 1).is_some_and(|(_, n)| n.is_ascii_digit())) {
            // Mirrors the tokenizer: 0x/0b/0o literals, decimals, and an exponent
            // only when digits follow (so the `e` in `2e` is the constant).
            let at = |k: usize| chars.get(k).map(|(_, ch)| *ch);
            let mut j = i;
            if c == '0' && matches!(at(i + 1), Some('x' | 'b' | 'o')) {
                j = i + 2;
                while at(j).is_some_and(|d| d.is_ascii_hexdigit() || d == '_') {
                    j += 1;
                }
            } else {
                while at(j).is_some_and(|d| d.is_ascii_digit() || d == '.' || d == '_') {
                    j += 1;
                }
                if matches!(at(j), Some('e' | 'E')) {
                    let mut k = j + 1;
                    if matches!(at(k), Some('+' | '-')) {
                        k += 1;
                    }
                    if at(k).is_some_and(|d| d.is_ascii_digit()) {
                        j = k;
                        while at(j).is_some_and(|d| d.is_ascii_digit()) {
                            j += 1;
                        }
                    }
                }
            }
            push(start..end_of(j), Class::Number);
            i = j;
            continue;
        }
        if c.is_alphabetic() || c == '_' || c == 'π' || c == '°' {
            let mut j = i + 1;
            while j < chars.len() && (chars[j].1.is_alphanumeric() || chars[j].1 == '_') {
                j += 1;
            }
            let word = &text[start..end_of(j)];
            let next_non_space = chars[j..].iter().find(|(_, c)| *c != ' ').map(|(_, c)| *c);
            let class = if is_keyword(word) {
                Class::Keyword
            } else if next_non_space == Some('(') && (catalog::get(word).is_some() || variables.iter().any(|v| v == word)) {
                Class::Function
            } else if variables.iter().any(|v| v == word) || word == "ans" || is_line_ref(word) {
                Class::Variable
            } else if CONSTANTS.contains(&word) || word == "π" {
                Class::Constant
            } else if FORMATS.contains(&word) {
                Class::Keyword
            } else if units::lookup(word).is_some() || word.starts_with('°') {
                Class::Unit
            } else if catalog::get(word).is_some() {
                Class::Function
            } else {
                Class::Plain
            };
            push(start..end_of(j), class);
            i = j;
            continue;
        }
        let class = if "+-*/^%!=<>?:,;()[]×÷·−√".contains(c) { Class::Operator } else { Class::Plain };
        push(start..end_of(i + 1), class);
        i += 1;
    }
    spans
}

fn is_line_ref(word: &str) -> bool {
    word.strip_prefix("line").is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completion {
    pub label: String,
    /// Text inserted in place of the typed prefix.
    pub insert: String,
    pub detail: String,
}

/// The identifier being typed immediately before byte offset `cursor`.
pub fn word_before(text: &str, cursor: usize) -> (usize, &str) {
    let before = &text[..cursor];
    let start = before
        .char_indices()
        .rev()
        .take_while(|(_, c)| c.is_alphanumeric() || *c == '_')
        .last()
        .map_or(cursor, |(i, _)| i);
    let word = &text[start..cursor];
    // A word that starts with a digit is a number, not a name.
    if word.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        return (cursor, "");
    }
    (start, word)
}

/// Suggestions for a prefix: user variables, then functions, then constants.
/// Exact-case prefix matches come before case-insensitive ones.
pub fn complete(prefix: &str, variables: &[String], limit: usize) -> Vec<Completion> {
    if prefix.is_empty() {
        return Vec::new();
    }
    let lower = prefix.to_lowercase();
    let mut candidates: Vec<(u8, Completion)> = Vec::new();
    let mut add = |name: &str, insert: String, detail: String| {
        let rank = if name.starts_with(prefix) {
            0
        } else if name.to_lowercase().starts_with(&lower) {
            1
        } else {
            return;
        };
        if name == prefix || candidates.iter().any(|(_, c)| c.label == name) {
            return;
        }
        candidates.push((rank, Completion { label: name.to_string(), insert, detail }));
    };
    for v in variables {
        add(v, v.clone(), "variable".into());
    }
    for e in ENTRIES {
        add(e.name, format!("{}(", e.name), format!("{} — {}", e.signature, e.description));
    }
    for c in CONSTANTS {
        add(c, c.to_string(), "constant".into());
    }
    candidates.sort_by(|(ra, a), (rb, b)| ra.cmp(rb).then(a.label.len().cmp(&b.label.len())).then(a.label.cmp(&b.label)));
    candidates.into_iter().take(limit).map(|(_, c)| c).collect()
}
