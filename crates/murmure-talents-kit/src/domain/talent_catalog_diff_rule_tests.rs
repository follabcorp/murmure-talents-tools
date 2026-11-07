//! Contract: « Auteurs, versions ». The comparison of two states of a Catalogue, on in-memory snapshots.

use std::collections::BTreeMap;

use super::{
    SemanticVersion, TalentCapability, TalentCatalogDiff, TalentCatalogDiffRule, TalentChangeKind,
    TalentDiffErrorKind, TalentSnapshot,
};

fn talent(id: &str, version: &str, capabilities: &[TalentCapability], requires: &[&str]) -> TalentSnapshot {
    TalentSnapshot {
        directory_id: id.to_owned(),
        id: id.to_owned(),
        version: SemanticVersion::parse(version).expect("a version"),
        capabilities: capabilities.to_vec(),
        requires: requires.iter().map(|name| (*name).to_owned()).collect(),
        files: BTreeMap::from([
            ("talent.json".to_owned(), format!("hash-manifest-{version}")),
            ("SKILL.md".to_owned(), "hash-skill".to_owned()),
        ]),
    }
}

fn with_file(mut snapshot: TalentSnapshot, path: &str, hash: &str) -> TalentSnapshot {
    snapshot.files.insert(path.to_owned(), hash.to_owned());
    snapshot
}

fn compare(base: Vec<TalentSnapshot>, head: Vec<TalentSnapshot>) -> TalentCatalogDiff {
    TalentCatalogDiffRule::compare(&base, &head)
}

fn error_kinds(diff: &TalentCatalogDiff) -> Vec<(&str, TalentDiffErrorKind)> {
    diff.errors.iter().map(|error| (error.directory_id.as_str(), error.kind)).collect()
}

#[test]
fn a_changed_file_at_an_equal_version_is_an_error() {
    let base = talent("trip", "1.0.0", &[], &[]);
    let head = with_file(base.clone(), "SKILL.md", "hash-edited");
    let diff = compare(vec![base], vec![head]);
    assert!(!diff.is_valid());
    assert_eq!(error_kinds(&diff), [("trip", TalentDiffErrorKind::FilesChangedWithoutVersionBump)]);
}

#[test]
fn an_added_or_removed_file_at_an_equal_version_is_an_error() {
    let base = talent("trip", "1.0.0", &[], &[]);
    let added = with_file(base.clone(), "notes.md", "hash-notes");
    let mut removed = base.clone();
    removed.files.remove("SKILL.md");
    for head in [added, removed] {
        let diff = compare(vec![base.clone()], vec![head]);
        assert_eq!(error_kinds(&diff), [("trip", TalentDiffErrorKind::FilesChangedWithoutVersionBump)]);
    }
}

#[test]
fn an_identical_talent_is_neither_an_error_nor_a_change() {
    let base = talent("trip", "1.0.0", &[], &[]);
    let diff = compare(vec![base.clone()], vec![base]);
    assert!(diff.is_valid());
    assert!(diff.changes.is_empty());
}

#[test]
fn a_version_that_goes_back_is_an_error() {
    let base = talent("trip", "1.2.0", &[], &[]);
    let head = with_file(talent("trip", "1.1.9", &[], &[]), "SKILL.md", "hash-edited");
    let diff = compare(vec![base], vec![head]);
    assert!(error_kinds(&diff).contains(&("trip", TalentDiffErrorKind::VersionDecreased)));
}

#[test]
fn a_changed_identifier_is_an_error() {
    let base = talent("trip", "1.0.0", &[], &[]);
    let mut head = talent("trip", "1.1.0", &[], &[]);
    head.id = "journey".to_owned();
    let diff = compare(vec![base], vec![head]);
    assert!(error_kinds(&diff).contains(&("trip", TalentDiffErrorKind::IdChanged)));
}

#[test]
fn a_version_raise_is_accepted_and_compared_as_semver_not_as_text() {
    let base = talent("trip", "1.9.0", &[], &[]);
    let head = with_file(talent("trip", "1.10.0", &[], &[]), "SKILL.md", "hash-edited");
    let diff = compare(vec![base], vec![head]);
    assert!(diff.is_valid());
    assert_eq!(diff.changes.len(), 1);
    assert_eq!(diff.changes[0].kind, TalentChangeKind::Modified);
}

#[test]
fn an_added_capability_is_listed_with_the_ones_the_head_gains_only() {
    let base = talent("trip", "1.0.0", &[TalentCapability::ReadMail], &[]);
    let head = talent("trip", "1.1.0", &[TalentCapability::ReadMail, TalentCapability::SendMail], &[]);
    let diff = compare(vec![base], vec![head]);
    assert!(diff.is_valid());
    assert_eq!(diff.changes[0].added_capabilities, [TalentCapability::SendMail]);
    assert!(diff.changes[0].added_requires.is_empty());
}

#[test]
fn a_removed_capability_is_not_reported_as_added() {
    let base = talent("trip", "1.0.0", &[TalentCapability::ReadMail, TalentCapability::SendMail], &[]);
    let head = talent("trip", "1.1.0", &[TalentCapability::ReadMail], &[]);
    let diff = compare(vec![base], vec![head]);
    assert!(diff.is_valid());
    assert!(diff.changes[0].added_capabilities.is_empty());
}

#[test]
fn an_added_dependency_is_listed() {
    let base = talent("trip", "1.0.0", &[], &["helper"]);
    let head = talent("trip", "1.1.0", &[], &["helper", "mapper"]);
    let diff = compare(vec![base], vec![head]);
    assert!(diff.is_valid());
    assert_eq!(diff.changes[0].added_requires, ["mapper".to_owned()]);
}

#[test]
fn a_new_talent_is_reported_with_all_its_capabilities_and_dependencies_as_added() {
    let head = talent("fresh", "1.0.0", &[TalentCapability::ReadMail], &["helper"]);
    let diff = compare(vec![], vec![head]);
    assert!(diff.is_valid());
    assert_eq!(diff.changes.len(), 1);
    assert_eq!(diff.changes[0].kind, TalentChangeKind::Added);
    assert_eq!(diff.changes[0].added_capabilities, [TalentCapability::ReadMail]);
    assert_eq!(diff.changes[0].added_requires, ["helper".to_owned()]);
}

#[test]
fn a_removed_talent_is_reported_without_error() {
    let base = talent("gone", "2.0.0", &[TalentCapability::ReadMail], &[]);
    let diff = compare(vec![base], vec![]);
    assert!(diff.is_valid());
    assert_eq!(diff.changes.len(), 1);
    assert_eq!(diff.changes[0].kind, TalentChangeKind::Removed);
    assert!(diff.changes[0].added_capabilities.is_empty());
}

#[test]
fn errors_and_changes_are_sorted_by_folder_whatever_the_input_order() {
    let base = vec![talent("zeta", "1.0.0", &[], &[]), talent("alpha", "1.0.0", &[], &[])];
    let head = vec![
        with_file(talent("zeta", "1.0.0", &[], &[]), "SKILL.md", "edited"),
        with_file(talent("alpha", "1.0.0", &[], &[]), "SKILL.md", "edited"),
        talent("middle", "1.0.0", &[], &[]),
    ];
    let diff = compare(base.clone(), head.clone());
    let reversed: Vec<_> = head.into_iter().rev().collect();
    assert_eq!(diff, compare(base.into_iter().rev().collect(), reversed));
    let folders: Vec<&str> = diff.errors.iter().map(|error| error.directory_id.as_str()).collect();
    assert_eq!(folders, ["alpha", "zeta"]);
}
