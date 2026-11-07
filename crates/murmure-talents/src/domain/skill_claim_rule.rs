//! The Skills a Talent of the Catalogue claims, and whether two Talents claim the same name with different content.
//! A rule of the publication of the Catalogue: the Relay checks a Skill against the fingerprint it kept when it
//! learned it (`TalentSkillOwnershipRule`), not between two Talents of a Catalogue.

use std::collections::BTreeMap;

use murmure_talents_kit::{hex, is_valid_identifier, TalentFileVerification};
use sha2::{Digest, Sha256};

use super::{CatalogError, CatalogFile, CatalogValidationRule};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillClaim {
    pub name: String,
    pub fingerprint: String,
}

pub struct SkillClaimRule;

impl SkillClaimRule {
    /// A `SKILL.md` at the root makes one Skill named after the folder; otherwise each file is `skills/<name>/…`
    /// with a valid name. Each Skill is checked as a bundle of files of the contract.
    pub fn claims(directory_id: &str, files: &[CatalogFile]) -> Result<Vec<SkillClaim>, CatalogError> {
        let single = files.iter().any(|file| file.path == "SKILL.md");
        let mut groups: BTreeMap<String, Vec<CatalogFile>> = BTreeMap::new();
        for file in files {
            if single {
                groups.entry(directory_id.to_owned()).or_default().push(file.clone());
                continue;
            }
            let parts: Vec<&str> = file.path.split('/').collect();
            if parts.len() < 3 || parts[0] != "skills" || !is_valid_identifier(parts[1]) {
                return Err(CatalogError::new("Ressources hors d’une Compétence."));
            }
            let relative = CatalogFile { path: parts[2..].join("/"), ..file.clone() };
            groups.entry(parts[1].to_owned()).or_default().push(relative);
        }
        groups
            .into_iter()
            .map(|(name, mut group)| {
                TalentFileVerification::validate(&CatalogValidationRule::file_descriptors(&group))
                    .map_err(|_| CatalogError::new("Fichiers de Compétence invalides."))?;
                group.sort_by(|left, right| left.path.cmp(&right.path));
                let mut content = Vec::new();
                for file in &group {
                    content.extend_from_slice(file.path.as_bytes());
                    content.push(0);
                    content.extend_from_slice(hex(&Sha256::digest(&file.data)).as_bytes());
                    content.push(0);
                }
                Ok(SkillClaim { name, fingerprint: hex(&Sha256::digest(&content)) })
            })
            .collect()
    }

    /// The same name with two fingerprints is a conflict.
    pub fn validate(claims: &[SkillClaim]) -> Result<(), CatalogError> {
        let mut seen: BTreeMap<&str, &str> = BTreeMap::new();
        for claim in claims {
            if *seen.entry(&claim.name).or_insert(&claim.fingerprint) != claim.fingerprint {
                return Err(CatalogError::new("Deux Talents revendiquent une Compétence différente de même nom."));
            }
        }
        Ok(())
    }
}
