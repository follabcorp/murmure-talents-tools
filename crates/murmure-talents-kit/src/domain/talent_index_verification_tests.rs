//! Contract: « Catalogue » (`TalentIndexVerification`, `TalentCatalogKey`).

use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use serde_json::{json, Value};

use super::fixtures::manifest_json;
use super::{TalentCatalogKeyRule, TalentIndex, TalentIndexError, TalentIndexVerification};

const GENERATED: i64 = 1_793_613_600;
const EXPIRES: i64 = 1_794_823_200;

fn key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

fn index_bytes(sequence: u64) -> Vec<u8> {
    let mut manifest: Value = manifest_json();
    manifest["requires"] = json!([]);
    json!({
        "format": 1, "sequence": sequence, "generatedAt": "2026-11-02T10:00:00Z", "expiresAt": "2026-11-16T10:00:00Z",
        "baseURL": "https://talents.example/v1/", "featured": [],
        "talents": [{"manifest": manifest, "files": [{"path": "SKILL.md", "size": 1, "sha256": "0".repeat(64)}]}], "revoked": []
    })
    .to_string()
    .into_bytes()
}

fn verify(
    data: &[u8],
    signer: &SigningKey,
    now: i64,
    previous: Option<u64>,
) -> Result<TalentIndex, TalentIndexError> {
    let signature = signer.sign(data).to_bytes();
    TalentIndexVerification::verify(
        data,
        &signature,
        &signer.verifying_key().to_bytes(),
        now,
        previous,
    )
}

#[test]
fn an_index_signed_by_the_key_is_verified_while_it_is_valid() {
    let data = index_bytes(7);
    assert_eq!(
        verify(&data, &key(1), GENERATED + 60, None).map(|index| index.sequence),
        Ok(7)
    );
}

#[test]
fn the_signature_covers_the_exact_bytes_and_comes_from_the_key() {
    let signer = key(1);
    let data = index_bytes(7);
    let signature = signer.sign(&data).to_bytes();
    let public = signer.verifying_key().to_bytes();
    let mut tampered = data.clone();
    tampered.push(b' ');
    assert_eq!(
        TalentIndexVerification::verify(&tampered, &signature, &public, GENERATED + 60, None),
        Err(TalentIndexError::InvalidSignature)
    );
    assert_eq!(
        TalentIndexVerification::verify(
            &data,
            &signature,
            &key(2).verifying_key().to_bytes(),
            GENERATED + 60,
            None
        ),
        Err(TalentIndexError::InvalidSignature)
    );
    assert_eq!(
        TalentIndexVerification::verify(&data, &signature[..63], &public, GENERATED + 60, None),
        Err(TalentIndexError::InvalidSignature),
        "a short signature"
    );
    assert_eq!(
        TalentIndexVerification::verify(&data, &signature, &public[..31], GENERATED + 60, None),
        Err(TalentIndexError::InvalidSignature),
        "a short key"
    );
}

#[test]
fn the_signature_is_checked_before_the_bytes_are_interpreted_and_the_size_before_both() {
    let signer = key(1);
    let garbage = b"not json at all".to_vec();
    let other = key(2).sign(&garbage).to_bytes();
    assert_eq!(
        TalentIndexVerification::authenticate(&garbage, &other, &signer.verifying_key().to_bytes()),
        Err(TalentIndexError::InvalidSignature)
    );
    assert_eq!(
        verify(&garbage, &signer, GENERATED + 60, None),
        Err(TalentIndexError::MalformedJson),
        "well signed, but not an index"
    );
    let big = vec![b' '; TalentIndex::MAXIMUM_BYTES + 1];
    assert_eq!(
        TalentIndexVerification::authenticate(&big, &other, &signer.verifying_key().to_bytes()),
        Err(TalentIndexError::TooLarge)
    );
}

#[test]
fn an_index_generated_in_the_future_or_expired_equality_included_is_refused() {
    let data = index_bytes(1);
    assert!(
        verify(&data, &key(1), GENERATED, None).is_ok(),
        "generated exactly now"
    );
    assert_eq!(
        verify(&data, &key(1), GENERATED - 1, None),
        Err(TalentIndexError::Expired),
        "generated in the future"
    );
    assert!(verify(&data, &key(1), EXPIRES - 1, None).is_ok());
    assert_eq!(
        verify(&data, &key(1), EXPIRES, None),
        Err(TalentIndexError::Expired),
        "now == expires_at"
    );
    assert_eq!(
        verify(&data, &key(1), EXPIRES + 1, None),
        Err(TalentIndexError::Expired)
    );
}

#[test]
fn a_sequence_below_the_previous_one_is_a_rollback_and_an_equal_one_is_allowed() {
    let data = index_bytes(5);
    assert!(
        verify(&data, &key(1), GENERATED + 60, Some(5)).is_ok(),
        "equal"
    );
    assert!(verify(&data, &key(1), GENERATED + 60, Some(4)).is_ok());
    assert_eq!(
        verify(&data, &key(1), GENERATED + 60, Some(6)),
        Err(TalentIndexError::SequenceRollback)
    );
}

#[test]
fn an_expired_index_still_authenticates_to_keep_its_anti_rollback_threshold() {
    let signer = key(1);
    let data = index_bytes(9);
    let signature = signer.sign(&data).to_bytes();
    assert_eq!(
        TalentIndexVerification::authenticate(
            &data,
            &signature,
            &signer.verifying_key().to_bytes()
        )
        .map(|index| index.sequence),
        Ok(9)
    );
}

#[test]
fn the_bench_key_replaces_the_key_of_production_and_the_replacement_key_is_refused_without_it() {
    let production = [9u8; 32];
    let bench = [5u8; 32];
    assert_eq!(
        TalentCatalogKeyRule::select(production, None),
        Ok(production)
    );
    assert_eq!(
        TalentCatalogKeyRule::select(production, Some(bench)),
        Ok(bench)
    );
    assert_eq!(
        TalentCatalogKeyRule::select(TalentCatalogKeyRule::REPLACEMENT_KEY, None),
        Err(TalentIndexError::ReplacementKey)
    );
    assert_eq!(
        TalentCatalogKeyRule::select(TalentCatalogKeyRule::REPLACEMENT_KEY, Some(bench)),
        Ok(bench)
    );
}

#[test]
fn the_key_of_production_is_the_root_key_and_is_accepted() {
    let production = TalentCatalogKeyRule::PRODUCTION_KEY;
    assert_ne!(production, TalentCatalogKeyRule::REPLACEMENT_KEY);
    assert_eq!(
        base64::engine::general_purpose::STANDARD.encode(production),
        "OARTzC2g9cWFq/1Gy1FGrnr3HN4Y1nkYouNOaplWF8U="
    );
    assert_eq!(
        TalentCatalogKeyRule::select(production, None),
        Ok(production)
    );
}
