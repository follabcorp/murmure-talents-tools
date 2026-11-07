//! The static, defensive search of the instructions of a Talent: text that looks like an exfiltration, and the
//! evocation of a mail sending. Case and diacritics are ignored, and a gap never crosses a line break.

use unicode_normalization::{char::is_combining_mark, UnicodeNormalization};

pub struct InstructionScanRule;

impl InstructionScanRule {
    pub fn is_suspicious(text: &str) -> bool {
        let text = fold(text);
        text_contains(&text, "exfiltrat")
            || gap(&text, &["ignore"], &["rules", "instructions", "regles", "consignes"], 50)
            || gap(&text, &["ignorer", "ignore", "ignorez"], &["regles", "consignes", "instructions"], 50)
            || gap(&text, &["forward", "transfer", "transfere"], &["mail", "email", "courriel"], 50)
            || gap(&text, &["send", "upload", "post", "envo", "transmet"], &["http://", "https://"], 100)
            || link_then_keyword(&text)
    }

    pub fn evokes_mail_sending(text: &str) -> bool {
        let text = fold(text);
        gap(&text, &["send", "forward", "envo", "transfer", "transfere"], &["mail", "email", "courriel"], 60)
    }
}

fn fold(text: &str) -> Vec<char> {
    text.nfd().filter(|character| !is_combining_mark(*character)).flat_map(char::to_lowercase).collect()
}

fn text_contains(text: &[char], word: &str) -> bool {
    (0..text.len()).any(|at| starts_with(text, at, word))
}

fn starts_with(text: &[char], at: usize, word: &str) -> bool {
    word.chars().enumerate().all(|(offset, expected)| text.get(at + offset) == Some(&expected))
}

fn is_line_break(character: char) -> bool {
    matches!(character, '\n' | '\u{b}' | '\u{c}' | '\r' | '\u{85}' | '\u{2028}' | '\u{2029}')
}

/// `(starts).{0,max}(ends)`: an end follows a start after at most `max` characters of the same line.
fn gap(text: &[char], starts: &[&str], ends: &[&str], max: usize) -> bool {
    (0..text.len()).any(|at| {
        starts.iter().filter(|start| starts_with(text, at, start)).any(|start| {
            let from = at + start.chars().count();
            (0..=max).any(|skipped| {
                ends.iter().any(|end| starts_with(text, from + skipped, end))
                    && text[from..from + skipped].iter().all(|character| !is_line_break(*character))
            })
        })
    })
}

/// `https?://\S+.{0,60}(upload|collect|exfiltrat)`, in one pass: the end of the non-blank run, the next line break
/// and the keyword occurrences are indexed once, so every link only costs a lookup.
fn link_then_keyword(text: &[char]) -> bool {
    let len = text.len();
    let mut next_blank = vec![len; len + 1];
    let mut next_break = vec![len; len + 1];
    for at in (0..len).rev() {
        next_blank[at] = if text[at].is_whitespace() { at } else { next_blank[at + 1] };
        next_break[at] = if is_line_break(text[at]) { at } else { next_break[at + 1] };
    }
    // keywords_before[at] = number of keyword starts strictly before `at`.
    let mut keywords_before = vec![0usize; len + 2];
    for at in 0..=len {
        let is_keyword = ["upload", "collect", "exfiltrat"].iter().any(|word| starts_with(text, at, word));
        keywords_before[at + 1] = keywords_before[at] + usize::from(is_keyword);
    }
    (0..len).any(|at| {
        ["http://", "https://"].iter().filter(|scheme| starts_with(text, at, scheme)).any(|scheme| {
            let from = at + scheme.chars().count();
            let run = next_blank[from.min(len)] - from.min(len);
            if run == 0 {
                return false;
            }
            let last = (from + run + 60).min(next_break[from.min(len)]).min(len);
            from < last && keywords_before[last + 1] - keywords_before[from + 1] > 0
        })
    })
}
