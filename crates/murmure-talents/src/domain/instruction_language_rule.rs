//! The language of the instructions of a Talent: they are written in English and tell the Agent to reply in the
//! language of the person. Pure heuristics; the results are warnings, never errors.

use unicode_normalization::{char::is_combining_mark, UnicodeNormalization};

/// Function words of French that English does not share (`a`, `on`, `son`… are left out on purpose).
const FRENCH_WORDS: [&str; 33] = [
    "le", "la", "les", "des", "du", "un", "une", "et", "est", "pour", "avec", "dans", "que", "qui", "tu", "il", "elle",
    "sa", "ses", "quand", "ou", "au", "aux", "sur", "pas", "ne", "ce", "cette", "vous", "nous", "sont", "mais", "par",
];
const MINIMUM_FRENCH_WORDS: usize = 3;
/// Share of French function words from which a text is taken for French (percent of its words).
const FRENCH_SHARE_PERCENT: usize = 12;

const REPLY_PHRASES: [&str; 8] = [
    "language the person",
    "language of the person",
    "person's language",
    "language the user",
    "language of the user",
    "user's language",
    "same language as the person",
    "same language as the user",
];

pub struct InstructionLanguageRule;

impl InstructionLanguageRule {
    /// True when the text seems written in French: a high share of French function words.
    pub fn seems_not_english(text: &str) -> bool {
        let folded = fold(text);
        let words: Vec<&str> = folded.split(|character: char| !character.is_alphabetic()).filter(|word| !word.is_empty()).collect();
        let french = words.iter().filter(|word| FRENCH_WORDS.contains(word)).count();
        french >= MINIMUM_FRENCH_WORDS && french * 100 >= words.len() * FRENCH_SHARE_PERCENT
    }

    /// True when the text tells the Agent to reply in the language the person uses (not in one fixed language).
    pub fn has_reply_language_instruction(text: &str) -> bool {
        let folded = fold(text).replace('’', "'");
        REPLY_PHRASES.iter().any(|phrase| folded.contains(phrase))
    }
}

fn fold(text: &str) -> String {
    text.nfd().filter(|character| !is_combining_mark(*character)).flat_map(char::to_lowercase).collect()
}
