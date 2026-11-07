//! What the tests of the rules share: in-memory Catalogue sources, never a disk.

use murmure_talents_kit::{TalentManifest, TalentManifestRule, TalentSource};
use serde_json::{json, Map, Value};

use super::{CatalogFile, CatalogSource, CatalogValidationRule, FileKind, ValidationReport};

/// 2026-11-02T00:00:00Z
pub const NOW: i64 = 1_793_577_600;

pub fn skill() -> Vec<u8> {
    b"---\nname: travel-coach\ndescription: Plan a trip.\n---\nSuggest a local itinerary.\nReply in the language the person uses.\n".to_vec()
}

pub fn manifest(id: &str, changes: Value) -> Vec<u8> {
    let mut value = json!({
        "id": id, "version": "1.2.0", "kind": "textOnly",
        "localized": {
            "fr": {"name": "Préparer un voyage", "summary": "Prépare un itinéraire."},
            "en": {"name": "Plan a trip", "summary": "Prepare an itinerary."}
        },
        "needs": ["travel"], "capabilities": [], "requires": [],
        "minimumMurmureVersion": "1.0.0", "author": {"name": "Test Maintainer", "github": ["test-author"]}, "license": "MIT"
    });
    let overrides: &Map<String, Value> = changes.as_object().expect("an object");
    for (key, new) in overrides {
        value[key] = new.clone();
    }
    value.to_string().into_bytes()
}

pub fn file(path: &str, data: &[u8]) -> CatalogFile {
    CatalogFile { path: path.to_owned(), data: data.to_vec(), kind: FileKind::Regular, permissions: 0o644 }
}

pub fn talent(id: &str, changes: Value, files: Vec<CatalogFile>) -> CatalogSource {
    CatalogSource { directory_id: id.to_owned(), manifest_json: manifest(id, changes), files }
}

pub fn default_talent(changes: Value) -> CatalogSource {
    talent("travel-coach", changes, vec![file("SKILL.md", &skill())])
}

pub fn embedded(id: &str, capability: &str) -> TalentManifest {
    let bytes = manifest(id, json!({"kind": "withCode", "capabilities": [capability]}));
    TalentManifestRule::parse(&bytes, TalentSource::Embedded).expect("an embedded manifest")
}

pub fn validate(talent: CatalogSource, companions: Vec<CatalogSource>, built_in: &[TalentManifest]) -> ValidationReport {
    let all: Vec<CatalogSource> = std::iter::once(talent).chain(companions).collect();
    CatalogValidationRule::validate(&all, built_in)
}
