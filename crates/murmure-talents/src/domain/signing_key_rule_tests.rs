use std::path::PathBuf;

use super::{FileKind, SigningKeyFacts, SigningKeyRule};

// Paths are canonical facts supplied by the disk boundary, never resolved by the pure rule.
fn facts() -> SigningKeyFacts {
    SigningKeyFacts {
        resolved_key: PathBuf::from("/maintenance/murmure-talents-keys/catalog.key"),
        repository: PathBuf::from("/maintenance/murmure-talents"),
        key_repository: None,
        kind: FileKind::Regular,
        link_count: 1,
        permissions: 0o600,
        length: 32,
    }
}

#[test]
fn accepts_an_owner_only_key_outside_the_repository_even_with_a_shared_path_prefix() {
    assert!(SigningKeyRule::validate(&facts()).is_ok());
}

#[test]
fn refuses_an_owner_only_key_inside_the_repository() {
    let inside = SigningKeyFacts { resolved_key: PathBuf::from("/maintenance/murmure-talents/keys/catalog.key"), ..facts() };
    assert!(SigningKeyRule::validate(&inside).is_err());
}

#[test]
fn refuses_a_key_inside_its_own_repository_even_when_the_working_repository_is_elsewhere() {
    let inside = SigningKeyFacts {
        resolved_key: PathBuf::from("/elsewhere/repo/catalog.key"),
        key_repository: Some(PathBuf::from("/elsewhere/repo")),
        ..facts()
    };
    assert!(SigningKeyRule::validate(&inside).is_err());
}

#[test]
fn refuses_a_key_readable_by_others_even_outside_the_repository() {
    assert!(SigningKeyRule::validate(&SigningKeyFacts { permissions: 0o644, ..facts() }).is_err());
}

#[test]
fn refuses_a_key_more_permissive_than_owner_only_in_any_bit() {
    assert!(SigningKeyRule::validate(&SigningKeyFacts { permissions: 0o700, ..facts() }).is_err());
    assert!(SigningKeyRule::validate(&SigningKeyFacts { permissions: 0o400, ..facts() }).is_err());
}

#[test]
fn refuses_a_symbolic_link() {
    assert!(SigningKeyRule::validate(&SigningKeyFacts { kind: FileKind::SymbolicLink, ..facts() }).is_err());
}

#[test]
fn refuses_a_special_file() {
    assert!(SigningKeyRule::validate(&SigningKeyFacts { kind: FileKind::Other, ..facts() }).is_err());
}

#[test]
fn refuses_a_hard_linked_key() {
    assert!(SigningKeyRule::validate(&SigningKeyFacts { link_count: 2, ..facts() }).is_err());
}

#[test]
fn refuses_a_key_that_is_not_exactly_32_raw_bytes() {
    for length in [0, 31, 33, 64] {
        assert!(SigningKeyRule::validate(&SigningKeyFacts { length, ..facts() }).is_err(), "{length} bytes");
    }
}
