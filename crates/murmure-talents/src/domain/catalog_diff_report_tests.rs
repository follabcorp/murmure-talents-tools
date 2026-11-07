//! Contract: « Auteurs, versions et CODEOWNERS ». The report of the comparison, on in-memory sources.

use serde_json::json;

use super::test_fixtures::{manifest, skill};
use super::{CatalogDiffReport, CatalogFile, CatalogSource, FileKind};

fn source(id: &str, changes: serde_json::Value) -> CatalogSource {
    CatalogSource {
        directory_id: id.to_owned(),
        manifest_json: manifest(id, changes),
        files: vec![CatalogFile { path: "SKILL.md".to_owned(), data: skill(), kind: FileKind::Regular, permissions: 0o644 }],
    }
}

#[test]
fn a_new_capability_asks_for_two_approvals_and_is_listed_in_both_formats() {
    let base = [source("trip", json!({}))];
    let head = [source("trip", json!({"version": "1.3.0", "capabilities": ["read-mail"], "needs": ["mail"]}))];
    let report = CatalogDiffReport::build(&base, &head);
    assert!(report.is_valid());
    assert!(report.text().contains("Capacités ajoutées : read-mail"));
    assert!(report.text().contains("deux approbations de Mainteneurs requises"));
    let json: serde_json::Value = serde_json::from_str(&report.json()).unwrap();
    assert_eq!(json["talents"][0]["addedCapabilities"], json!(["read-mail"]));
}

#[test]
fn a_head_manifest_that_cannot_be_read_is_an_error_and_a_base_one_is_ignored() {
    let broken = CatalogSource { manifest_json: b"{".to_vec(), ..source("trip", json!({})) };
    let report = CatalogDiffReport::build(std::slice::from_ref(&broken), &[source("trip", json!({}))]);
    assert!(report.is_valid());
    let report = CatalogDiffReport::build(&[source("trip", json!({}))], &[broken]);
    assert!(!report.is_valid());
    assert!(report.text().contains("ERREUR [trip]"));
}

#[test]
fn a_changed_file_at_an_equal_version_makes_the_report_invalid_and_names_the_talent() {
    let base = [source("trip", json!({}))];
    let mut changed = source("trip", json!({}));
    changed.files[0].data = b"another text".to_vec();
    let report = CatalogDiffReport::build(&base, &[changed]);
    assert!(!report.is_valid());
    assert!(report.text().contains("ERREUR [trip]"));
}
