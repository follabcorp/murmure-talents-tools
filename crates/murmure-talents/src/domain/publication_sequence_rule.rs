use super::CatalogError;

/// The value of `--sequence` of `publish`.
pub struct PublicationSequenceRule;

impl PublicationSequenceRule {
    /// `auto` is the injected clock in seconds, so that each publication is higher than the previous one without
    /// state; otherwise an explicit unsigned integer. Anything else is refused.
    pub fn resolve(text: &str, now: i64) -> Result<u64, CatalogError> {
        if text == "auto" {
            return u64::try_from(now).map_err(|_| CatalogError::new("Horloge invalide pour la séquence automatique."));
        }
        if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(CatalogError::new("Séquence invalide : `auto` ou un entier positif ou nul."));
        }
        text.parse().map_err(|_| CatalogError::new("Séquence invalide : `auto` ou un entier positif ou nul."))
    }
}
