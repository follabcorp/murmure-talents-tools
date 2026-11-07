use murmure_talents_kit::{SemanticVersion, TalentFileVerification, TalentIndex};
use serde_json::json;
use std::collections::BTreeMap;

use super::test_fixtures::{default_talent, skill, NOW};
use super::CatalogIndexRule;

const BASE_URL: &str = "https://talents.example.invalid/v1/";

#[test]
fn builds_the_shared_index_contract_and_versioned_files_from_an_injected_date() {
    let artifact = CatalogIndexRule::build(
        &[default_talent(json!({}))], &["travel-coach".to_owned()], &[], BASE_URL, 42, NOW, &[],
    ).expect("an index");
    let index = TalentIndex::parse(&artifact.index_data).expect("the Kit reads it");
    assert_eq!(index.sequence, 42);
    assert_eq!(index.generated_at, NOW);
    assert_eq!(index.expires_at, 1_794_787_200); // 2026-11-16T00:00:00Z
    assert_eq!(index.base_url, BASE_URL);
    assert_eq!(index.featured, vec!["travel-coach".to_owned()]);
    let entry = index.talents.first().expect("an entry");
    assert_eq!(entry.manifest.id, "travel-coach");
    assert_eq!(entry.manifest.version, SemanticVersion::parse("1.2.0").unwrap());
    assert_eq!(artifact.files["travel-coach/1.2.0/SKILL.md"], skill());
    let downloaded = BTreeMap::from([("SKILL.md".to_owned(), skill())]);
    TalentFileVerification::verify(&entry.files, &downloaded).expect("the files match the index");
}

#[test]
fn refuses_an_invalid_catalog() {
    let invalid = default_talent(json!({"license": "GPL-3.0"}));
    assert!(CatalogIndexRule::build(&[invalid], &[], &[], BASE_URL, 1, NOW, &[]).is_err());
}

#[test]
fn refuses_an_unsafe_base_url() {
    let source = default_talent(json!({}));
    assert!(CatalogIndexRule::build(&[source], &[], &[], "http://talents.example.invalid/", 1, NOW, &[]).is_err());
}
