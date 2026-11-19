//! What the tests of the rules share: manifests, as JSON and as values.

use std::collections::BTreeMap;

use serde_json::{json, Value};

use super::{
    LocalizedText, SemanticVersion, TalentAuthor, TalentCapability, TalentKind, TalentManifest,
    TalentNeed,
};

/// A valid manifest of the catalog, as JSON.
pub fn manifest_json() -> Value {
    json!({
        "id": "trier-mes-mails",
        "version": "1.2.0",
        "kind": "textOnly",
        "localized": {
            "fr": {"name": "Trier mes mails", "summary": "Range ta boîte mail et te signale ce qui compte."},
            "en": {"name": "Sort my email", "summary": "Tidies your inbox and points out what matters."}
        },
        "needs": ["mail", "organizing"],
        "capabilities": ["read-mail"],
        "requires": ["mes-mails"],
        "minimumMurmureVersion": "1.0.0",
        "platforms": ["macos"],
        "author": {"name": "Camille Dupont", "url": "https://example.org", "github": ["camille-dupont"]},
        "license": "MIT",
        "origin": {"kind": "openclaw", "author": "jdoe", "url": "https://clawhub.ai/skills/inbox-triage"}
    })
}

pub fn bytes(value: &Value) -> Vec<u8> {
    value.to_string().into_bytes()
}

/// A manifest value: textual, French and English, one need, no platform restriction.
pub fn manifest(
    id: &str,
    version: &str,
    capabilities: &[TalentCapability],
    requires: &[&str],
) -> TalentManifest {
    let text = |name: &str| LocalizedText {
        name: name.to_owned(),
        summary: format!("Résumé de {name}."),
    };
    TalentManifest {
        id: id.to_owned(),
        version: SemanticVersion::parse(version).expect("a version"),
        kind: TalentKind::TextOnly,
        localized: BTreeMap::from([
            ("fr".to_owned(), text(&format!("Talent {id}"))),
            ("en".to_owned(), text(&format!("Talent {id} (en)"))),
        ]),
        needs: vec![TalentNeed::Organizing],
        capabilities: capabilities.to_vec(),
        requires: requires.iter().map(|id| (*id).to_owned()).collect(),
        minimum_murmure_version: SemanticVersion::new(1, 0, 0),
        platforms: None,
        author: TalentAuthor {
            name: "Auteur".to_owned(),
            url: None,
            github: Vec::new(),
        },
        license: "MIT".to_owned(),
        origin: None,
        agent: None,
    }
}

/// A Talent with code that supplies `capabilities`.
pub fn supplier(id: &str, capabilities: &[TalentCapability]) -> TalentManifest {
    TalentManifest {
        kind: TalentKind::WithCode,
        ..manifest(id, "1.0.0", capabilities, &[])
    }
}

/// A valid Agent Talent manifest of the catalog, as JSON.
pub fn agent_manifest_json() -> Value {
    json!({
        "id": "coach-voyage",
        "version": "1.0.0",
        "kind": "agent",
        "localized": {
            "fr": {"name": "Coach voyage", "summary": "Un Agent qui prépare tes voyages."},
            "en": {"name": "Travel coach", "summary": "An Agent that plans your trips."}
        },
        "needs": ["travel"],
        "capabilities": [],
        "minimumMurmureVersion": "1.0.0",
        "author": {"name": "Camille Dupont", "github": ["camille-dupont"]},
        "license": "MIT",
        "agent": {
            "localized": {
                "fr": {"name": "Voyageur", "role": "Prépare des itinéraires.", "personality": "Chaleureux.\nToujours concret."},
                "en": {"name": "Traveler", "role": "Plans itineraries.", "personality": "Warm.\nAlways concrete."}
            },
            "skills": ["travel-coach"],
            "connectors": ["calendrier"],
            "brain": "reasoning"
        }
    })
}
