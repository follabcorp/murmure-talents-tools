//! The strict reading of `talent.json` (docs/murmure/talents.md, « Manifeste » and « Nature »). Pure: no file is read.

use std::collections::BTreeMap;

use serde_json::{Map, Value};
use strict_json::parse_strict_json;
use unicode_segmentation::UnicodeSegmentation;

use super::{
    LocalizedText, SemanticVersion, TalentAgent, TalentAgentIdentity, TalentAuthor, TalentBrain, TalentCapability,
    TalentKind, TalentManifest, TalentManifestError, TalentNeed, TalentOrigin, TalentOriginKind, TalentPlatform,
    TalentSecretScan, TalentSource,
};

const MANIFEST_BYTES_MAX: usize = 16_384;
const URL_BYTES_MAX: usize = 256;
const AGENT_NAME_GRAPHEMES_MAX: usize = 40;
const AGENT_ROLE_GRAPHEMES_MAX: usize = 120;
const AGENT_PERSONALITY_GRAPHEMES_MAX: usize = 4_000;
const AGENT_SKILLS_MAX: usize = 16;
const AGENT_CONNECTORS_MAX: usize = 16;
/// The keys that would let an Agent Talent act without asking: refused with their own error, anywhere they stand.
const AUTONOMY_KEYS: [&str; 3] = ["autonomous", "actWithoutAsking", "autonomy"];

pub struct TalentManifestRule;

type Failure = TalentManifestError;

impl TalentManifestRule {
    /// 16 KiB at most, checked before any analysis; strict JSON (no duplicate key, even escaped); no unknown key at
    /// any level, no type outside the schema, no value outside the closed vocabularies.
    pub fn parse(data: &[u8], source: TalentSource) -> Result<TalentManifest, TalentManifestError> {
        if data.len() > MANIFEST_BYTES_MAX {
            return Err(Failure::TooLarge);
        }
        let value = parse_strict_json(data).map_err(|_| Failure::MalformedJson)?;
        refuse_autonomy_keys(&value)?;
        let keys = [
            "id",
            "version",
            "kind",
            "localized",
            "needs",
            "capabilities",
            "requires",
            "minimumMurmureVersion",
            "platforms",
            "author",
            "license",
            "origin",
            "agent",
        ];
        let root = object(Some(&value), "manifest", &keys)?;
        let id = string(root.get("id"), "id")?;
        if !is_valid_identifier(id) {
            return Err(invalid("id"));
        }
        let version = read_version(root, "version")?;
        let kind = match string(root.get("kind"), "kind")? {
            "textOnly" => TalentKind::TextOnly,
            "withCode" => TalentKind::WithCode,
            "agent" => TalentKind::Agent,
            _ => return Err(invalid("kind")),
        };
        if kind == TalentKind::WithCode && source != TalentSource::Embedded {
            return Err(Failure::KindNotAllowed);
        }
        let localized = localized(root.get("localized"))?;
        let needs = list(root.get("needs"), "needs", 1..=2, TalentNeed::from_wire)?;
        let capabilities = list(
            root.get("capabilities"),
            "capabilities",
            0..=TalentCapability::ALL.len(),
            TalentCapability::from_wire,
        )?;
        let requires = match root.get("requires") {
            Some(value) => list(Some(value), "requires", 0..=8, |name| {
                (is_valid_identifier(name) && name != id).then(|| name.to_owned())
            })?,
            None => Vec::new(),
        };
        let minimum_murmure_version = read_version(root, "minimumMurmureVersion")?;
        let platforms = match root.get("platforms") {
            Some(value) => Some(list(Some(value), "platforms", 1..=2, platform)?),
            None => None,
        };
        let author = object(root.get("author"), "author", &["name", "url", "github"])?;
        let author = TalentAuthor {
            name: clean_text(author.get("name"), "author.name", 64)?,
            url: url(author.get("url"), "author.url")?,
            github: github_accounts(author.get("github"), source)?,
        };
        let license = string(root.get("license"), "license")?.to_owned();
        if !is_valid_license(&license) {
            return Err(invalid("license"));
        }
        let origin = match root.get("origin") {
            Some(value) => Some(origin(value)?),
            None => None,
        };
        let agent = match (kind, root.get("agent")) {
            (TalentKind::Agent, Some(value)) => Some(read_agent(value)?),
            (TalentKind::Agent, None) => return Err(invalid("agent")),
            (_, Some(_)) => return Err(invalid("agent")),
            (_, None) => None,
        };
        if kind == TalentKind::Agent {
            refuse_secret_in_public_texts(&localized, &author)?;
        }
        Ok(TalentManifest {
            id: id.to_owned(),
            version,
            kind,
            localized,
            needs,
            capabilities,
            requires,
            minimum_murmure_version,
            platforms,
            author,
            license,
            origin,
            agent,
        })
    }
}

/// An autonomy key at the root or in the `agent` object (the only places a manifest has free keys to try).
fn refuse_autonomy_keys(value: &Value) -> Result<(), Failure> {
    let Some(root) = value.as_object() else { return Ok(()) };
    let inside = root.get("agent").and_then(Value::as_object);
    let found = std::iter::once(root).chain(inside).any(|object| AUTONOMY_KEYS.iter().any(|key| object.contains_key(*key)));
    if found {
        Err(Failure::AutonomyField)
    } else {
        Ok(())
    }
}

fn read_agent(value: &Value) -> Result<TalentAgent, Failure> {
    let agent = object(Some(value), "agent", &["localized", "skills", "connectors", "brain"])?;
    let brain = match string(agent.get("brain"), "agent.brain")? {
        "reasoning" => TalentBrain::Reasoning,
        "fast" => TalentBrain::Fast,
        "local" => TalentBrain::Local,
        _ => return Err(invalid("agent.brain")),
    };
    Ok(TalentAgent {
        localized: agent_identities(agent.get("localized"))?,
        skills: list(agent.get("skills"), "agent.skills", 0..=AGENT_SKILLS_MAX, identifier)?,
        connectors: list(agent.get("connectors"), "agent.connectors", 0..=AGENT_CONNECTORS_MAX, identifier)?,
        brain,
    })
}

fn identifier(name: &str) -> Option<String> {
    is_valid_identifier(name).then(|| name.to_owned())
}

/// The same languages rules as `localized`; each entry holds a name, a one-line Role and a multi-line Personality,
/// none of them carrying a secret.
fn agent_identities(value: Option<&Value>) -> Result<BTreeMap<String, TalentAgentIdentity>, Failure> {
    let entries = value.and_then(Value::as_object).filter(|entries| {
        (2..=16).contains(&entries.len()) && entries.contains_key("fr") && entries.contains_key("en")
    });
    let entries = entries.ok_or_else(|| invalid("agent.localized"))?;
    let mut identities = BTreeMap::new();
    for (language, entry) in entries {
        if !is_valid_language(language) {
            return Err(invalid("agent.localized"));
        }
        let entry = object(Some(entry), "agent.localized", &["name", "role", "personality"])?;
        let name = clean_text(entry.get("name"), "agent.localized.name", AGENT_NAME_GRAPHEMES_MAX)?;
        refuse_secret(&name, "agent.localized.name")?;
        let role = clean_text(entry.get("role"), "agent.localized.role", AGENT_ROLE_GRAPHEMES_MAX)?;
        refuse_secret(&role, "agent.localized.role")?;
        let personality = multiline_text(entry.get("personality"), "agent.localized.personality", AGENT_PERSONALITY_GRAPHEMES_MAX)?;
        refuse_secret(&personality, "agent.localized.personality")?;
        identities.insert(language.clone(), TalentAgentIdentity { name, role, personality });
    }
    Ok(identities)
}

/// ADR-0030: an Agent Talent never carries a secret, so its other public free texts are scanned too.
fn refuse_secret_in_public_texts(localized: &BTreeMap<String, LocalizedText>, author: &TalentAuthor) -> Result<(), Failure> {
    for text in localized.values() {
        refuse_secret(&text.name, "localized.name")?;
        refuse_secret(&text.summary, "localized.summary")?;
    }
    refuse_secret(&author.name, "author.name")
}

fn refuse_secret(text: &str, field: &str) -> Result<(), Failure> {
    match TalentSecretScan::find(text) {
        Some(_) => Err(Failure::SecretFound(field.to_owned())),
        None => Ok(()),
    }
}

/// Trimmed at the edges, 1 to `limit` graphemes; line breaks are allowed, any other control character is not.
fn multiline_text(value: Option<&Value>, field: &str, limit: usize) -> Result<String, Failure> {
    let original = string(value, field)?;
    let text = original.trim_matches(|character| is_edge_space(character) || character == '\n');
    let graphemes = text.graphemes(true).count();
    let forbidden = original
        .chars()
        .any(|character| (character.is_control() && character != '\n') || matches!(character, '\u{2028}' | '\u{2029}'));
    if (1..=limit).contains(&graphemes) && !forbidden {
        Ok(text.to_owned())
    } else {
        Err(invalid(field))
    }
}

/// 1 to 4 distinct GitHub accounts: required from the Catalogue, optional elsewhere.
fn github_accounts(value: Option<&Value>, source: TalentSource) -> Result<Vec<String>, Failure> {
    let Some(value) = value else {
        return if source == TalentSource::Catalog { Err(invalid("author.github")) } else { Ok(Vec::new()) };
    };
    list(Some(value), "author.github", 1..=4, |name| is_valid_github_account(name).then(|| name.to_owned()))
}

/// GitHub's syntax: 1 to 39 ASCII letters, digits or hyphens, no leading or trailing hyphen, no double hyphen.
fn is_valid_github_account(name: &str) -> bool {
    (1..=39).contains(&name.len())
        && name.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        && !name.starts_with('-')
        && !name.ends_with('-')
        && !name.contains("--")
}

fn invalid(field: &str) -> Failure {
    Failure::InvalidField(field.to_owned())
}

/// 1 to 64 bytes of `[a-z0-9-]`.
pub fn is_valid_identifier(name: &str) -> bool {
    (1..=64).contains(&name.len())
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn is_valid_license(license: &str) -> bool {
    (1..=64).contains(&license.len())
        && license
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.'))
}

fn object<'a>(
    value: Option<&'a Value>,
    field: &str,
    keys: &[&str],
) -> Result<&'a Map<String, Value>, Failure> {
    let object = value
        .and_then(Value::as_object)
        .ok_or_else(|| invalid(field))?;
    if object.keys().all(|key| keys.contains(&key.as_str())) {
        Ok(object)
    } else {
        Err(Failure::UnknownKey)
    }
}

fn string<'a>(value: Option<&'a Value>, field: &str) -> Result<&'a str, Failure> {
    value.and_then(Value::as_str).ok_or_else(|| invalid(field))
}

fn read_version(root: &Map<String, Value>, key: &str) -> Result<SemanticVersion, Failure> {
    SemanticVersion::parse(string(root.get(key), key)?).ok_or_else(|| invalid(key))
}

fn platform(value: &str) -> Option<TalentPlatform> {
    match value {
        "macos" => Some(TalentPlatform::Macos),
        "linux" => Some(TalentPlatform::Linux),
        _ => None,
    }
}

/// Distinct values of a closed list, in a bounded count; one value outside the list refuses the whole list.
fn list<T: PartialEq>(
    value: Option<&Value>,
    field: &str,
    range: std::ops::RangeInclusive<usize>,
    transform: impl Fn(&str) -> Option<T>,
) -> Result<Vec<T>, Failure> {
    let items = value
        .and_then(Value::as_array)
        .filter(|items| range.contains(&items.len()))
        .ok_or_else(|| invalid(field))?;
    let mut values: Vec<T> = Vec::with_capacity(items.len());
    for item in items {
        let value = item
            .as_str()
            .and_then(&transform)
            .ok_or_else(|| invalid(field))?;
        if values.contains(&value) {
            return Err(invalid(field));
        }
        values.push(value);
    }
    Ok(values)
}

fn localized(value: Option<&Value>) -> Result<BTreeMap<String, LocalizedText>, Failure> {
    let entries = value.and_then(Value::as_object).filter(|entries| {
        (2..=16).contains(&entries.len())
            && entries.contains_key("fr")
            && entries.contains_key("en")
    });
    let entries = entries.ok_or_else(|| invalid("localized"))?;
    let mut localized = BTreeMap::new();
    for (language, entry) in entries {
        if !is_valid_language(language) {
            return Err(invalid("localized"));
        }
        let entry = object(Some(entry), "localized", &["name", "summary"])?;
        let text = LocalizedText {
            name: clean_text(entry.get("name"), "localized.name", 40)?,
            summary: clean_text(entry.get("summary"), "localized.summary", 140)?,
        };
        localized.insert(language.clone(), text);
    }
    Ok(localized)
}

/// `[a-z]{2,3}` and an optional script `-[A-Z][a-z]{3}`.
fn is_valid_language(tag: &str) -> bool {
    let mut parts = tag.split('-');
    let language = parts.next().unwrap_or("");
    let script = parts.next();
    if parts.next().is_some()
        || !(2..=3).contains(&language.len())
        || !language.bytes().all(|byte| byte.is_ascii_lowercase())
    {
        return false;
    }
    script.is_none_or(|script| {
        let bytes = script.as_bytes();
        bytes.len() == 4
            && bytes[0].is_ascii_uppercase()
            && bytes[1..].iter().all(u8::is_ascii_lowercase)
    })
}

/// Trimmed at the edges (spaces and tab), 1 to `limit` graphemes, no control character and no line break.
fn clean_text(value: Option<&Value>, field: &str, limit: usize) -> Result<String, Failure> {
    let original = string(value, field)?;
    let text = original.trim_matches(is_edge_space);
    let graphemes = text.graphemes(true).count();
    let forbidden = original
        .chars()
        .any(|character| character.is_control() || matches!(character, '\u{2028}' | '\u{2029}'));
    if (1..=limit).contains(&graphemes) && !forbidden {
        Ok(text.to_owned())
    } else {
        Err(invalid(field))
    }
}

/// Foundation's `.whitespaces`: the tab and the space separators.
fn is_edge_space(character: char) -> bool {
    matches!(
        character,
        '\t' | ' ' | '\u{A0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200A}' | '\u{202F}' | '\u{205F}' | '\u{3000}'
    )
}

fn url(value: Option<&Value>, field: &str) -> Result<Option<String>, Failure> {
    let Some(value) = value else { return Ok(None) };
    let text = string(Some(value), field)?;
    let parsed = url::Url::parse(text).ok();
    let valid = text.starts_with("https://")
        && text.len() <= URL_BYTES_MAX
        && parsed.is_some_and(|parsed| {
            parsed.scheme() == "https" && parsed.host_str().is_some_and(|host| !host.is_empty())
        });
    if valid {
        Ok(Some(text.to_owned()))
    } else {
        Err(invalid(field))
    }
}

fn origin(value: &Value) -> Result<TalentOrigin, Failure> {
    let item = object(Some(value), "origin", &["kind", "author", "url"])?;
    let kind = match string(item.get("kind"), "origin.kind")? {
        "openclaw" => TalentOriginKind::Openclaw,
        "hermes" => TalentOriginKind::Hermes,
        "public" => TalentOriginKind::Public,
        _ => return Err(invalid("origin.kind")),
    };
    let url = url(item.get("url"), "origin.url")?;
    if kind == TalentOriginKind::Public && url.is_none() {
        return Err(invalid("origin.url"));
    }
    Ok(TalentOrigin { kind, author: clean_text(item.get("author"), "origin.author", 64)?, url })
}
