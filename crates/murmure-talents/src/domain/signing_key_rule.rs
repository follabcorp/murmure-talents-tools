use std::path::PathBuf;

use super::{CatalogError, FileKind};

/// The facts the disk adapter observed about the private key, never the key itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SigningKeyFacts {
    /// The key path with its links resolved, and the repository root, as canonical facts of the disk.
    pub resolved_key: PathBuf,
    pub repository: PathBuf,
    /// The nearest repository above the resolved key, when there is one: a key is never kept in any repository.
    pub key_repository: Option<PathBuf>,
    /// What the path is, without following a link at the last component.
    pub kind: FileKind,
    pub link_count: u32,
    pub permissions: u16,
    pub length: u64,
}

pub struct SigningKeyRule;

impl SigningKeyRule {
    /// A raw Ed25519 key: exactly 32 bytes, a regular file with one link, mode `0600`, outside the repository
    /// (a path that only shares a name prefix with it is outside).
    pub fn validate(facts: &SigningKeyFacts) -> Result<(), CatalogError> {
        if facts.kind != FileKind::Regular || facts.link_count != 1 {
            return Err(CatalogError::new("Clé privée non régulière ou liée."));
        }
        let in_key_repository = facts.key_repository.as_ref().is_some_and(|own| facts.resolved_key.starts_with(own));
        if facts.resolved_key.starts_with(&facts.repository) || in_key_repository || facts.permissions != 0o600 {
            return Err(CatalogError::new("La clé doit être un fichier 0600 hors du dépôt."));
        }
        if facts.length != 32 {
            return Err(CatalogError::new("Clé Ed25519 brute de 32 octets attendue."));
        }
        Ok(())
    }
}
