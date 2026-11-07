use murmure_talents_kit::TalentCatalogDelegation;

use super::PublicationDecisionRule;

const NOW: i64 = 1_793_577_600;
const DAY: i64 = 86_400;
const DELEGATED: [u8; 32] = [5; 32];

fn delegation(expires_in: i64) -> TalentCatalogDelegation {
    TalentCatalogDelegation { publication_key: DELEGATED, not_before: NOW - DAY, not_after: NOW + expires_in, sequence: 1 }
}

#[test]
fn accepts_a_delegation_with_seven_days_or_more_left() {
    assert!(PublicationDecisionRule::decide(&delegation(7 * DAY), &DELEGATED, NOW).is_ok());
    assert!(PublicationDecisionRule::decide(&delegation(300 * DAY), &DELEGATED, NOW).is_ok());
}

#[test]
fn refuses_a_delegation_with_less_than_seven_days_left() {
    assert!(PublicationDecisionRule::decide(&delegation(6 * DAY), &DELEGATED, NOW).is_err());
    assert!(PublicationDecisionRule::decide(&delegation(7 * DAY - 1), &DELEGATED, NOW).is_err());
    assert!(PublicationDecisionRule::decide(&delegation(0), &DELEGATED, NOW).is_err());
}

#[test]
fn refuses_a_publication_key_the_delegation_does_not_name() {
    let error = PublicationDecisionRule::decide(&delegation(300 * DAY), &[6; 32], NOW).unwrap_err();
    assert!(error.0.contains("déléguée"), "{}", error.0);
}
