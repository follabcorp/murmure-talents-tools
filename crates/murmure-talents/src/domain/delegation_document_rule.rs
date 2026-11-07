use base64::{engine::general_purpose::STANDARD, Engine};

use super::catalog_index_rule::iso_date;
use super::CatalogError;

/// `delegation.json`: the document the root key signs to name the publication key (ADR-0022).
pub struct DelegationDocumentRule;

impl DelegationDocumentRule {
    pub const MINIMUM_VALID_DAYS: u32 = 1;
    pub const MAXIMUM_VALID_DAYS: u32 = 400;

    /// The exact bytes to sign: closed keys `publicationKey` (Base64 of the 32 public bytes), `notBefore` = `now`,
    /// `notAfter` = `now` + `valid_days` days (ISO 8601 UTC) and `delegationSequence`. Refuses a public key that is
    /// not the Base64 of 32 bytes and `valid_days` outside 1…400.
    pub fn build(
        publication_public_key: &str,
        sequence: u64,
        now: i64,
        valid_days: u32,
    ) -> Result<Vec<u8>, CatalogError> {
        if !(Self::MINIMUM_VALID_DAYS..=Self::MAXIMUM_VALID_DAYS).contains(&valid_days) {
            return Err(CatalogError::new("La validité de la délégation doit être de 1 à 400 jours."));
        }
        let key_is_valid = STANDARD
            .decode(publication_public_key)
            .is_ok_and(|bytes| bytes.len() == 32 && STANDARD.encode(&bytes) == publication_public_key);
        if !key_is_valid {
            return Err(CatalogError::new("La clé publique de publication doit être le base64 de 32 octets."));
        }
        let not_after = now + i64::from(valid_days) * 86_400;
        // Keys in alphabetical order, compact: the same shape as `index.json`.
        Ok(format!(
            "{{\"delegationSequence\":{sequence},\"notAfter\":\"{}\",\"notBefore\":\"{}\",\"publicationKey\":\"{publication_public_key}\"}}",
            iso_date(not_after),
            iso_date(now),
        )
        .into_bytes())
    }
}
