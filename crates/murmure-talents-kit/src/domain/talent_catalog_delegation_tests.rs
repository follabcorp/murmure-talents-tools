//! Contract: « Catalogue » (`TalentCatalogDelegation`, `TalentCatalogTrust`): the root key signs the delegation, the
//! delegated publication key signs the index.

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use serde_json::{json, Value};

use super::fixtures::manifest_json;
use super::{
    TalentCatalogDelegation, TalentCatalogTrust, TalentCatalogTrustError, TalentDelegationError,
    TalentIndexError,
};

const NOT_BEFORE: i64 = 1_793_000_000;
const NOT_BEFORE_ISO: &str = "2026-10-26T07:33:20Z";
const NOT_AFTER: i64 = 1_800_000_000;
const NOT_AFTER_ISO: &str = "2027-01-15T08:00:00Z";
/// Inside the validity of the delegation and of `index_bytes`.
const NOW: i64 = 1_793_613_660;

fn key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

fn delegation_json(publication: &SigningKey, sequence: u64) -> Value {
    json!({
        "publicationKey": STANDARD.encode(publication.verifying_key().to_bytes()),
        "notBefore": NOT_BEFORE_ISO, "notAfter": NOT_AFTER_ISO, "delegationSequence": sequence,
    })
}

fn bytes(value: &Value) -> Vec<u8> {
    value.to_string().into_bytes()
}

fn index_bytes(sequence: u64) -> Vec<u8> {
    let mut manifest: Value = manifest_json();
    manifest["requires"] = json!([]);
    bytes(&json!({
        "format": 1, "sequence": sequence, "generatedAt": "2026-11-02T10:00:00Z", "expiresAt": "2026-11-16T10:00:00Z",
        "baseURL": "https://talents.example/v1/", "featured": [],
        "talents": [{"manifest": manifest, "files": [{"path": "SKILL.md", "size": 1, "sha256": "0".repeat(64)}]}], "revoked": []
    }))
}

/// The four served files: the root signs the delegation, the publication key signs the index.
struct Chain {
    delegation: Vec<u8>,
    delegation_signature: Vec<u8>,
    index: Vec<u8>,
    index_signature: Vec<u8>,
}

impl Chain {
    fn new(root: &SigningKey, publication: &SigningKey, delegation_sequence: u64) -> Self {
        let delegation = bytes(&delegation_json(publication, delegation_sequence));
        let index = index_bytes(7);
        Self {
            delegation_signature: root.sign(&delegation).to_bytes().to_vec(),
            index_signature: publication.sign(&index).to_bytes().to_vec(),
            delegation,
            index,
        }
    }

    fn verify(
        &self,
        root: &SigningKey,
        now: i64,
        previous_delegation: Option<u64>,
        previous_index: Option<u64>,
    ) -> Result<(u64, u64), TalentCatalogTrustError> {
        TalentCatalogTrust::verify(
            &self.delegation,
            &self.delegation_signature,
            &self.index,
            &self.index_signature,
            &root.verifying_key().to_bytes(),
            now,
            previous_delegation,
            previous_index,
        )
        .map(|verified| (verified.delegation.sequence, verified.index.sequence))
    }
}

fn delegation_refused(error: TalentDelegationError) -> TalentCatalogTrustError {
    TalentCatalogTrustError::Delegation(error)
}

#[test]
fn an_index_signed_by_the_delegated_key_under_a_delegation_signed_by_the_root_is_verified() {
    let chain = Chain::new(&key(1), &key(2), 3);
    assert_eq!(chain.verify(&key(1), NOW, None, None), Ok((3, 7)));
    assert_eq!(
        chain.verify(&key(1), NOW, Some(3), Some(7)),
        Ok((3, 7)),
        "equal sequences are allowed"
    );
}

#[test]
fn a_delegation_the_root_did_not_sign_is_refused_before_anything_else() {
    let mut chain = Chain::new(&key(1), &key(2), 3);
    chain.delegation_signature[10] ^= 1;
    assert_eq!(
        chain.verify(&key(1), NOW, None, None),
        Err(delegation_refused(TalentDelegationError::InvalidSignature))
    );
    let other = Chain::new(&key(9), &key(2), 3);
    assert_eq!(
        other.verify(&key(1), NOW, None, None),
        Err(delegation_refused(TalentDelegationError::InvalidSignature)),
        "signed by another key"
    );
}

#[test]
fn a_delegation_is_valid_from_not_before_included_to_not_after_excluded() {
    let chain = Chain::new(&key(1), &key(2), 0);
    assert_eq!(
        chain.verify(&key(1), NOT_BEFORE - 1, None, None),
        Err(delegation_refused(TalentDelegationError::NotYetValid))
    );
    // The index of the fixtures was generated later: at `notBefore` the delegation passes and the index speaks.
    assert_eq!(
        chain.verify(&key(1), NOT_BEFORE, None, None),
        Err(TalentCatalogTrustError::Index(TalentIndexError::Expired))
    );
    assert_eq!(
        chain.verify(&key(1), NOT_AFTER, None, None),
        Err(delegation_refused(TalentDelegationError::Expired))
    );
}

#[test]
fn a_delegation_sequence_that_goes_back_is_refused_even_while_the_old_delegation_is_valid() {
    let chain = Chain::new(&key(1), &key(2), 4);
    assert_eq!(
        chain.verify(&key(1), NOW, Some(5), None),
        Err(delegation_refused(TalentDelegationError::SequenceRollback))
    );
    assert!(chain.verify(&key(1), NOW, Some(4), None).is_ok());
    assert!(chain.verify(&key(1), NOW, Some(3), None).is_ok());
}

#[test]
fn the_index_must_be_signed_by_the_delegated_key_not_by_the_root() {
    let mut chain = Chain::new(&key(1), &key(2), 3);
    chain.index_signature = key(1).sign(&chain.index).to_bytes().to_vec();
    assert_eq!(
        chain.verify(&key(1), NOW, None, None),
        Err(TalentCatalogTrustError::Index(
            TalentIndexError::InvalidSignature
        ))
    );
}

#[test]
fn the_index_keeps_its_own_expiry_and_anti_rollback_under_a_valid_delegation() {
    let chain = Chain::new(&key(1), &key(2), 3);
    assert_eq!(
        chain.verify(&key(1), NOW, None, Some(8)),
        Err(TalentCatalogTrustError::Index(
            TalentIndexError::SequenceRollback
        ))
    );
    // The index expires on 2026-11-16T10:00:00Z (1_794_823_200), inside the validity of the delegation.
    assert_eq!(
        chain.verify(&key(1), 1_794_823_200, None, None),
        Err(TalentCatalogTrustError::Index(TalentIndexError::Expired))
    );
}

#[test]
fn an_old_publication_key_is_refused_once_the_delegation_names_another() {
    let old = Chain::new(&key(1), &key(2), 3);
    let rotated = Chain::new(&key(1), &key(5), 4);
    let mixed = Chain {
        index: old.index.clone(),
        index_signature: old.index_signature.clone(),
        ..rotated
    };
    assert_eq!(
        mixed.verify(&key(1), NOW, Some(4), None),
        Err(TalentCatalogTrustError::Index(
            TalentIndexError::InvalidSignature
        ))
    );
}

#[test]
fn an_expired_chain_still_authenticates_to_keep_both_anti_rollback_thresholds() {
    let chain = Chain::new(&key(1), &key(2), 3);
    let verified = TalentCatalogTrust::authenticate(
        &chain.delegation,
        &chain.delegation_signature,
        &chain.index,
        &chain.index_signature,
        &key(1).verifying_key().to_bytes(),
    )
    .expect("authenticated");
    assert_eq!(
        (verified.delegation.sequence, verified.index.sequence),
        (3, 7)
    );
}

#[test]
fn the_delegation_is_read_strictly_and_bounded() {
    let valid = delegation_json(&key(2), 3);
    let parsed = TalentCatalogDelegation::parse(&bytes(&valid)).expect("read");
    assert_eq!(parsed.publication_key, key(2).verifying_key().to_bytes());
    assert_eq!(
        (parsed.not_before, parsed.not_after, parsed.sequence),
        (NOT_BEFORE, NOT_AFTER, 3)
    );

    let mut unknown = valid.clone();
    unknown["extra"] = json!(1);
    assert_eq!(
        TalentCatalogDelegation::parse(&bytes(&unknown)),
        Err(TalentDelegationError::UnknownKey)
    );
    let mut padded = bytes(&valid);
    padded.resize(TalentCatalogDelegation::MAXIMUM_BYTES + 1, b' ');
    assert_eq!(
        TalentCatalogDelegation::parse(&padded),
        Err(TalentDelegationError::TooLarge)
    );
    let mut short_key = valid.clone();
    short_key["publicationKey"] = json!(STANDARD.encode([1u8; 31]));
    assert_eq!(
        TalentCatalogDelegation::parse(&bytes(&short_key)),
        Err(TalentDelegationError::InvalidFormat)
    );
    let mut negative = valid.clone();
    negative["delegationSequence"] = json!(-1);
    assert_eq!(
        TalentCatalogDelegation::parse(&bytes(&negative)),
        Err(TalentDelegationError::InvalidFormat)
    );
    let mut backwards = valid;
    backwards["notAfter"] = backwards["notBefore"].clone();
    assert_eq!(
        TalentCatalogDelegation::parse(&bytes(&backwards)),
        Err(TalentDelegationError::InvalidFormat),
        "an empty validity"
    );
    assert_eq!(
        TalentCatalogDelegation::parse(b"not json"),
        Err(TalentDelegationError::MalformedJson)
    );
}
