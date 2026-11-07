use murmure_talents_kit::TalentCatalogDelegation;

use super::CatalogError;

/// Whether `publish` may go on with this delegation and this publication key.
pub struct PublicationDecisionRule;

impl PublicationDecisionRule {
    pub const MINIMUM_REMAINING_SECONDS: i64 = 7 * 24 * 3600;

    /// Refuses a key other than the delegated one first, then a delegation that expires in less than 7 days (exactly
    /// 7 days is accepted).
    pub fn decide(
        delegation: &TalentCatalogDelegation,
        publication_public_key: &[u8; 32],
        now: i64,
    ) -> Result<(), CatalogError> {
        if &delegation.publication_key != publication_public_key {
            return Err(CatalogError::new("La clé de publication n’est pas celle déléguée par delegation.json."));
        }
        if delegation.not_after - now < Self::MINIMUM_REMAINING_SECONDS {
            return Err(CatalogError::new(
                "La délégation expire dans moins de 7 jours : en signer une nouvelle avec `delegate`.",
            ));
        }
        Ok(())
    }
}
