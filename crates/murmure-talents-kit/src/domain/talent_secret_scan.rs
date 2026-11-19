//! Detects the shape of a secret in a free text of a Talent (an Agent Talent is public: it must carry none).
//!
//! The forms are named and listed here, close to the ones the Relay masks, but this crate depends on nothing of the
//! Relay: the public tool `murmure-talents` shares it.

/// A character class for the body of a secret, after its prefix.
#[derive(Clone, Copy)]
enum Body {
    /// `[A-Za-z0-9_-]`
    Token,
    /// `[A-Za-z0-9]`
    Alphanumeric,
    /// `[A-Z0-9]`
    UpperAlphanumeric,
}

impl Body {
    fn accepts(self, character: char) -> bool {
        match self {
            Body::Token => character.is_ascii_alphanumeric() || matches!(character, '_' | '-'),
            Body::Alphanumeric => character.is_ascii_alphanumeric(),
            Body::UpperAlphanumeric => character.is_ascii_uppercase() || character.is_ascii_digit(),
        }
    }
}

/// One recognized form: the word starts with `prefix`, followed by at least `minimum` characters of `body`.
struct PrefixedForm {
    name: &'static str,
    prefix: &'static str,
    minimum: usize,
    body: Body,
}

const PREFIXED_FORMS: [PrefixedForm; 15] = [
    PrefixedForm { name: "API key (sk-…, sk-ant-…)", prefix: "sk-", minimum: 20, body: Body::Token },
    PrefixedForm { name: "Stripe key", prefix: "sk_live_", minimum: 16, body: Body::Alphanumeric },
    PrefixedForm { name: "Stripe key", prefix: "rk_live_", minimum: 16, body: Body::Alphanumeric },
    PrefixedForm { name: "GitHub token", prefix: "ghp_", minimum: 30, body: Body::Alphanumeric },
    PrefixedForm { name: "GitHub token", prefix: "gho_", minimum: 30, body: Body::Alphanumeric },
    PrefixedForm { name: "GitHub token", prefix: "ghu_", minimum: 30, body: Body::Alphanumeric },
    PrefixedForm { name: "GitHub token", prefix: "ghs_", minimum: 30, body: Body::Alphanumeric },
    PrefixedForm { name: "GitHub token", prefix: "ghr_", minimum: 30, body: Body::Alphanumeric },
    PrefixedForm { name: "GitHub fine-grained token", prefix: "github_pat_", minimum: 22, body: Body::Token },
    PrefixedForm { name: "Slack token", prefix: "xoxa-", minimum: 10, body: Body::Token },
    PrefixedForm { name: "Slack token", prefix: "xoxb-", minimum: 10, body: Body::Token },
    PrefixedForm { name: "Slack token", prefix: "xoxp-", minimum: 10, body: Body::Token },
    PrefixedForm { name: "AWS access key", prefix: "AKIA", minimum: 16, body: Body::UpperAlphanumeric },
    PrefixedForm { name: "AWS access key", prefix: "ASIA", minimum: 16, body: Body::UpperAlphanumeric },
    PrefixedForm { name: "Google API key", prefix: "AIza", minimum: 35, body: Body::Token },
];

const PRIVATE_KEY_NAME: &str = "private key header";
const JWT_NAME: &str = "JWT";
const JWT_SEGMENT_MINIMUM: usize = 10;

pub struct TalentSecretScan;

impl TalentSecretScan {
    /// The name of the first recognized secret form found in `text`, if any.
    pub fn find(text: &str) -> Option<&'static str> {
        if text.lines().any(Self::is_private_key_header) {
            return Some(PRIVATE_KEY_NAME);
        }
        text.split(|character: char| !(character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')))
            .filter(|word| !word.is_empty())
            .find_map(Self::word_form)
    }

    fn is_private_key_header(line: &str) -> bool {
        line.find("-----BEGIN ").is_some_and(|start| line[start..].contains("PRIVATE KEY-----"))
    }

    /// A form is recognized at the start of a word, so that `task-…` or `mask-…` never reads as `sk-…`.
    fn word_form(word: &str) -> Option<&'static str> {
        if Self::is_jwt(word) {
            return Some(JWT_NAME);
        }
        PREFIXED_FORMS.iter().find_map(|form| {
            let rest = word.strip_prefix(form.prefix)?;
            let length = rest.chars().take_while(|character| form.body.accepts(*character)).count();
            (length >= form.minimum).then_some(form.name)
        })
    }

    /// Three dot-separated base64url segments, the header and the payload starting with `eyJ` (`{"`).
    fn is_jwt(word: &str) -> bool {
        let segments: Vec<&str> = word.trim_end_matches('.').split('.').collect();
        segments.len() == 3
            && segments[..2].iter().all(|segment| segment.starts_with("eyJ"))
            && segments.iter().all(|segment| segment.len() >= JWT_SEGMENT_MINIMUM)
    }
}
