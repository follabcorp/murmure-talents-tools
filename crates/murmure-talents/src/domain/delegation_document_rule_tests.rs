use base64::{engine::general_purpose::STANDARD, Engine};
use murmure_talents_kit::TalentCatalogDelegation;

use super::DelegationDocumentRule;

const NOW: i64 = 1_793_577_600; // 2026-11-02T00:00:00Z
const DAY: i64 = 86_400;

fn key() -> String {
    STANDARD.encode([5u8; 32])
}

#[test]
fn builds_a_document_the_kit_reads_with_the_requested_window_and_sequence() {
    let bytes = DelegationDocumentRule::build(&key(), 3, NOW, 365).unwrap();
    let delegation = TalentCatalogDelegation::parse(&bytes).expect("the Kit accepts the document");
    assert_eq!(delegation.publication_key, [5u8; 32]);
    assert_eq!(delegation.not_before, NOW);
    assert_eq!(delegation.not_after, NOW + 365 * DAY);
    assert_eq!(delegation.sequence, 3);
}

#[test]
fn accepts_the_bounds_of_one_and_four_hundred_days() {
    for days in [1, 400] {
        let bytes = DelegationDocumentRule::build(&key(), 0, NOW, days).unwrap();
        let delegation = TalentCatalogDelegation::parse(&bytes).unwrap();
        assert_eq!(delegation.not_after - delegation.not_before, i64::from(days) * DAY);
    }
}

#[test]
fn refuses_a_validity_outside_one_to_four_hundred_days() {
    for days in [0, 401, 10_000] {
        assert!(DelegationDocumentRule::build(&key(), 0, NOW, days).is_err(), "{days} days");
    }
}

#[test]
fn refuses_a_public_key_that_is_not_base64_of_32_bytes() {
    for text in ["", "not base64!", &STANDARD.encode([1u8; 31]), &STANDARD.encode([1u8; 33])] {
        assert!(DelegationDocumentRule::build(text, 0, NOW, 365).is_err(), "{text}");
    }
}
