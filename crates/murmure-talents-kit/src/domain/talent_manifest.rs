//! The manifest `talent.json`, once read (docs/murmure/talents.md, « Manifeste »).

use std::collections::BTreeMap;

use super::{SemanticVersion, TalentCapability, TalentNeed};

/// Where the bytes of a manifest come from: `WithCode` is admitted for `Embedded` only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TalentSource {
    Embedded,
    Catalog,
    LocalFolder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TalentKind {
    TextOnly,
    WithCode,
    /// An Agent Talent: no code, carries an `agent` object (ADR-0030, decision 8).
    Agent,
}

/// The platform of the Relay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TalentPlatform {
    Macos,
    Linux,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TalentOriginKind {
    Openclaw,
    Hermes,
    /// A skill from a public repository outside the OpenClaw and Hermes ecosystems; the `url` is mandatory.
    Public,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalizedText {
    pub name: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TalentAuthor {
    pub name: String,
    pub url: Option<String>,
    /// GitHub accounts of the Authors: 1 to 4 for a Catalog manifest, optional otherwise (empty when absent).
    pub github: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TalentOrigin {
    pub kind: TalentOriginKind,
    pub author: String,
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TalentManifest {
    pub id: String,
    pub version: SemanticVersion,
    pub kind: TalentKind,
    /// By language tag (`fr`, `en`, `zh-Hans`).
    pub localized: BTreeMap<String, LocalizedText>,
    pub needs: Vec<TalentNeed>,
    pub capabilities: Vec<TalentCapability>,
    pub requires: Vec<String>,
    pub minimum_murmure_version: SemanticVersion,
    /// None: every platform.
    pub platforms: Option<Vec<TalentPlatform>>,
    pub author: TalentAuthor,
    pub license: String,
    pub origin: Option<TalentOrigin>,
    /// Present exactly when `kind` is `Agent`.
    pub agent: Option<TalentAgent>,
}

/// The advised type of Brain of an Agent Talent (closed list).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TalentBrain {
    Reasoning,
    Fast,
    Local,
}

/// Localized identity of an Agent Talent: proposed name, Role (one line), Personality (multi-line).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TalentAgentIdentity {
    pub name: String,
    pub role: String,
    pub personality: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TalentAgent {
    /// By language tag, `fr` and `en` at least.
    pub localized: BTreeMap<String, TalentAgentIdentity>,
    /// Included Skills (folder names).
    pub skills: Vec<String>,
    /// Requested Connectors (identifiers).
    pub connectors: Vec<String>,
    pub brain: TalentBrain,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TalentManifestError {
    TooLarge,
    MalformedJson,
    UnknownKey,
    InvalidField(String),
    KindNotAllowed,
    /// An autonomy key (`autonomous`, `actWithoutAsking`, `autonomy`): an Agent Talent never sets it.
    AutonomyField,
    /// A free text of an Agent Talent contains a secret; the field is named.
    SecretFound(String),
}
