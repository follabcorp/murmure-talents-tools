//! The chain of trust of the Catalogue: the root key embedded in the Relay signs `delegation.json`, which names the
//! publication key that signs `index.json` (docs/murmure/catalogue-talents.md, ADR-0022).

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use ed25519_dalek::{Signature, VerifyingKey};
use serde_json::{Map, Value};
use strict_json::parse_strict_json;

use super::iso_date::parse_iso;
use super::{TalentIndex, TalentIndexError, TalentIndexVerification};

/// Why a delegation, or the chain it opens, is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TalentDelegationError {
    TooLarge,
    MalformedJson,
    UnknownKey,
    InvalidFormat,
    /// The root key did not sign these exact bytes.
    InvalidSignature,
    /// `now` is before `notBefore`.
    NotYetValid,
    /// `now` is at or after `notAfter`.
    Expired,
    /// The sequence is below the highest one known: how a leaked publication key is revoked.
    SequenceRollback,
}

/// Why the chain (delegation, then index) is refused: the stage that failed, and why.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TalentCatalogTrustError {
    Delegation(TalentDelegationError),
    Index(TalentIndexError),
}

/// `delegation.json`, once read. Times are Unix seconds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TalentCatalogDelegation {
    pub publication_key: [u8; 32],
    pub not_before: i64,
    pub not_after: i64,
    pub sequence: u64,
}

impl TalentCatalogDelegation {
    pub const MAXIMUM_BYTES: usize = 4096;

    /// Strict and bounded: size first, then strict JSON with the closed keys `publicationKey` (base64 of 32 bytes),
    /// `notBefore`, `notAfter` (ISO 8601, `notAfter` after `notBefore`) and `delegationSequence` (integer from 0).
    pub fn parse(data: &[u8]) -> Result<Self, TalentDelegationError> {
        use TalentDelegationError::{InvalidFormat, MalformedJson, TooLarge, UnknownKey};
        if data.len() > Self::MAXIMUM_BYTES {
            return Err(TooLarge);
        }
        let value = parse_strict_json(data).map_err(|_| MalformedJson)?;
        let object: &Map<String, Value> = value.as_object().ok_or(InvalidFormat)?;
        let keys = [
            "publicationKey",
            "notBefore",
            "notAfter",
            "delegationSequence",
        ];
        if !object.keys().all(|key| keys.contains(&key.as_str())) {
            return Err(UnknownKey);
        }
        if object.len() != keys.len() {
            return Err(InvalidFormat);
        }
        let date = |name: &str| {
            object
                .get(name)
                .and_then(Value::as_str)
                .and_then(parse_iso)
                .ok_or(InvalidFormat)
        };
        let publication_key = object
            .get("publicationKey")
            .and_then(Value::as_str)
            .and_then(|text| STANDARD.decode(text).ok())
            .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
            .ok_or(InvalidFormat)?;
        let (not_before, not_after) = (date("notBefore")?, date("notAfter")?);
        let sequence = object
            .get("delegationSequence")
            .and_then(Value::as_u64)
            .ok_or(InvalidFormat)?;
        if not_after <= not_before {
            return Err(InvalidFormat);
        }
        Ok(Self {
            publication_key,
            not_before,
            not_after,
            sequence,
        })
    }

    /// The root key signed these exact bytes, checked before any reading.
    fn authenticate(
        data: &[u8],
        signature: &[u8],
        root_key: &[u8],
    ) -> Result<Self, TalentDelegationError> {
        if data.len() > Self::MAXIMUM_BYTES {
            return Err(TalentDelegationError::TooLarge);
        }
        let key = <[u8; 32]>::try_from(root_key)
            .ok()
            .and_then(|bytes| VerifyingKey::from_bytes(&bytes).ok());
        let signature = Signature::from_slice(signature).ok();
        match (key, signature) {
            (Some(key), Some(signature)) if key.verify_strict(data, &signature).is_ok() => {
                Self::parse(data)
            }
            _ => Err(TalentDelegationError::InvalidSignature),
        }
    }
}

/// The two signatures that open the Catalogue.
pub struct TalentCatalogTrust;

/// A chain that passed every check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TalentVerifiedCatalog {
    pub delegation: TalentCatalogDelegation,
    pub index: TalentIndex,
}

impl TalentCatalogTrust {
    /// Root signature on the exact delegation bytes, then the delegation (size, reading, `notBefore <= now <
    /// notAfter`, sequence not below `previous_delegation_sequence`), then the index under the delegated key
    /// (`TalentIndexVerification::verify`: signature, expiry, sequence not below `previous_index_sequence`).
    #[allow(clippy::too_many_arguments)]
    pub fn verify(
        delegation: &[u8],
        delegation_signature: &[u8],
        index: &[u8],
        index_signature: &[u8],
        root_key: &[u8],
        now: i64,
        previous_delegation_sequence: Option<u64>,
        previous_index_sequence: Option<u64>,
    ) -> Result<TalentVerifiedCatalog, TalentCatalogTrustError> {
        use TalentCatalogTrustError::{Delegation, Index};
        let delegation =
            TalentCatalogDelegation::authenticate(delegation, delegation_signature, root_key)
                .map_err(Delegation)?;
        if now < delegation.not_before {
            return Err(Delegation(TalentDelegationError::NotYetValid));
        }
        if now >= delegation.not_after {
            return Err(Delegation(TalentDelegationError::Expired));
        }
        if previous_delegation_sequence.is_some_and(|previous| delegation.sequence < previous) {
            return Err(Delegation(TalentDelegationError::SequenceRollback));
        }
        let index = TalentIndexVerification::verify(
            index,
            index_signature,
            &delegation.publication_key,
            now,
            previous_index_sequence,
        )
        .map_err(Index)?;
        Ok(TalentVerifiedCatalog { delegation, index })
    }

    /// The signatures and the readings alone, no time and no sequence: an expired chain still raises the
    /// anti-rollback thresholds of both sequences.
    pub fn authenticate(
        delegation: &[u8],
        delegation_signature: &[u8],
        index: &[u8],
        index_signature: &[u8],
        root_key: &[u8],
    ) -> Result<TalentVerifiedCatalog, TalentCatalogTrustError> {
        use TalentCatalogTrustError::{Delegation, Index};
        let delegation =
            TalentCatalogDelegation::authenticate(delegation, delegation_signature, root_key)
                .map_err(Delegation)?;
        let index = TalentIndexVerification::authenticate(
            index,
            index_signature,
            &delegation.publication_key,
        )
        .map_err(Index)?;
        Ok(TalentVerifiedCatalog { delegation, index })
    }
}
