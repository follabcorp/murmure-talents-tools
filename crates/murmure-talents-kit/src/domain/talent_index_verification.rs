//! The signature of the index and the key that checks it (docs/murmure/talents.md, « Catalogue »).

use ed25519_dalek::{Signature, VerifyingKey};

use super::{TalentIndex, TalentIndexError};

pub struct TalentIndexVerification;

impl TalentIndexVerification {
    /// Authenticates the exact bytes (Ed25519), then reads them, then checks the times: not generated in the future,
    /// not expired (`now == expires_at` is expired), and a sequence not below the previous one (equal is allowed).
    pub fn verify(
        index_data: &[u8],
        signature: &[u8],
        public_key: &[u8],
        now: i64,
        previous_sequence: Option<u64>,
    ) -> Result<TalentIndex, TalentIndexError> {
        let index = Self::authenticate(index_data, signature, public_key)?;
        if now >= index.expires_at || index.generated_at > now {
            return Err(TalentIndexError::Expired);
        }
        if previous_sequence.is_some_and(|previous| index.sequence < previous) {
            return Err(TalentIndexError::SequenceRollback);
        }
        Ok(index)
    }

    /// Size first (4 MiB), then the signature on the exact bytes before any interpretation, then the reading. An
    /// expired index passes: its sequence still raises the anti-rollback threshold.
    pub fn authenticate(
        index_data: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<TalentIndex, TalentIndexError> {
        if index_data.len() > TalentIndex::MAXIMUM_BYTES {
            return Err(TalentIndexError::TooLarge);
        }
        let key = <[u8; 32]>::try_from(public_key)
            .ok()
            .and_then(|bytes| VerifyingKey::from_bytes(&bytes).ok());
        let signature = Signature::from_slice(signature).ok();
        match (key, signature) {
            (Some(key), Some(signature)) if key.verify_strict(index_data, &signature).is_ok() => {
                TalentIndex::parse(index_data)
            }
            _ => Err(TalentIndexError::InvalidSignature),
        }
    }
}

/// Which public key checks the Catalogue.
pub struct TalentCatalogKeyRule;

impl TalentCatalogKeyRule {
    /// The root public key of production; its private half stays out of the CI (ADR-0022). Rotating it takes a new
    /// version of the app.
    pub const PRODUCTION_KEY: [u8; 32] = [
        0x38, 0x04, 0x53, 0xcc, 0x2d, 0xa0, 0xf5, 0xc5, 0x85, 0xab, 0xfd, 0x46, 0xcb, 0x51, 0x46,
        0xae, 0x7a, 0xf7, 0x1c, 0xde, 0x18, 0xd6, 0x79, 0x18, 0xa2, 0xe3, 0x4e, 0x6a, 0x99, 0x56,
        0x17, 0xc5,
    ];

    /// The key of a launch that holds no key of production: a bench, which then has no remote Catalogue.
    pub const REPLACEMENT_KEY: [u8; 32] = [0; 32];

    /// The key of the bench when the bench gives one; otherwise the key of production, refused while it is the
    /// replacement key. No option or variable outside the bench can change it.
    pub fn select(
        production: [u8; 32],
        bench: Option<[u8; 32]>,
    ) -> Result<[u8; 32], TalentIndexError> {
        match bench {
            Some(key) => Ok(key),
            None if production == Self::REPLACEMENT_KEY => Err(TalentIndexError::ReplacementKey),
            None => Ok(production),
        }
    }
}
