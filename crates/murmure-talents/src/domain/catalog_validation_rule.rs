use std::collections::BTreeMap;

use murmure_talents_kit::{
    hex, TalentCapability, TalentDependencyRule, TalentFileVerification, TalentIndexFile, TalentManifest,
    TalentManifestError, TalentManifestRule, TalentSource,
};
use sha2::{Digest, Sha256};

use super::{
    CatalogError, CatalogFile, CatalogSource, Diagnostic, DiagnosticKind, FileKind, InstructionLanguageRule, InstructionScanRule,
    SkillClaim, SkillClaimRule, ValidationReport,
};

const TALENTS_MAX: usize = 2000;
pub(super) const LICENSES: [&str; 9] =
    ["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC", "0BSD", "Unlicense", "CC0-1.0", "CC-BY-4.0"];
const EXECUTABLE_HEADERS: [[u8; 4]; 9] = [
    [0x7f, 0x45, 0x4c, 0x46],
    [0xfe, 0xed, 0xfa, 0xce],
    [0xce, 0xfa, 0xed, 0xfe],
    [0xfe, 0xed, 0xfa, 0xcf],
    [0xcf, 0xfa, 0xed, 0xfe],
    [0xca, 0xfe, 0xba, 0xbe],
    [0xbe, 0xba, 0xfe, 0xca],
    [0xca, 0xfe, 0xba, 0xbf],
    [0xbf, 0xba, 0xfe, 0xca],
];

/// What `convert` writes in a translation to fill by hand; it blocks the validation.
pub(super) const TRANSLATION_PLACEHOLDER: &str = "TODO-fr";

pub struct CatalogValidationRule;

impl CatalogValidationRule {
    /// The pre-review of a Catalogue: at most 2 000 Talents; each manifest read as a catalog one with the folder name as
    /// identifier, free in the Catalogue and the built-in ones; an admitted license; the files (Kit verification,
    /// regular, not executable, not a shebang or a binary header, UTF-8); same-named Skills that differ; dependencies
    /// present, acyclic and covering the capabilities. Suspicious instructions and mail sending without `send-mail`
    /// are warnings, as are a `SKILL.md` that seems not English and one without a reply-language instruction; `TODO-fr`
    /// in `localized` is an error.
    pub fn validate(talents: &[CatalogSource], built_in: &[TalentManifest]) -> ValidationReport {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut reject = |id: &str, message: &str| {
            errors.push(Diagnostic {
                kind: DiagnosticKind::InvalidContent,
                talent_id: id.to_owned(),
                message: message.to_owned(),
            });
        };
        let mut catalog: BTreeMap<String, TalentManifest> =
            built_in.iter().map(|manifest| (manifest.id.clone(), manifest.clone())).collect();
        let mut manifests: BTreeMap<&str, TalentManifest> = BTreeMap::new();
        let mut claims: Vec<SkillClaim> = Vec::new();
        if talents.len() > TALENTS_MAX {
            reject("Catalogue", "Plus de 2 000 Talents.");
        }
        for source in talents {
            let id = source.directory_id.as_str();
            let (parsed, github_missing) = Self::parse_manifest(&source.manifest_json);
            if github_missing && parsed.as_ref().is_ok_and(|manifest| manifest.id == id) {
                warnings.push(Diagnostic {
                    kind: DiagnosticKind::MissingAuthorGithub,
                    talent_id: id.to_owned(),
                    message: "author.github est vide : à compléter avant la PR (sans compte, les Mainteneurs relisent).".to_owned(),
                });
            }
            let manifest = match parsed {
                Ok(manifest) if manifest.id == id && !catalog.contains_key(id) => manifest,
                Ok(_) => {
                    reject(id, "Manifeste ou ressources invalides : identifiant de dossier différent ou déjà présent.");
                    continue;
                }
                Err(error) => {
                    reject(id, &format!("Manifeste ou ressources invalides : {error:?}."));
                    continue;
                }
            };
            if manifest.localized.values().any(|text| text.name == TRANSLATION_PLACEHOLDER || text.summary == TRANSLATION_PLACEHOLDER) {
                reject(id, "Traduction à compléter : TODO-fr reste dans localized.");
            }
            if !LICENSES.contains(&manifest.license.as_str()) {
                reject(id, "Licence non admise par le Catalogue.");
            }
            manifests.insert(id, manifest.clone());
            catalog.insert(id.to_owned(), manifest);
            match Self::check_files(source) {
                Ok(found) => claims.extend(found),
                Err(error) => reject(id, &format!("Manifeste ou ressources invalides : {}", error.0)),
            }
        }
        if SkillClaimRule::validate(&claims).is_err() {
            reject("Catalogue", "Deux Talents revendiquent une Compétence différente de même nom.");
        }
        for source in talents {
            let Some(manifest) = manifests.get(source.directory_id.as_str()) else { continue };
            let chain = match TalentDependencyRule::missing(&manifest.id, &Default::default(), &catalog) {
                Ok(chain) => {
                    if !TalentDependencyRule::uncovered_capabilities(manifest, &catalog).is_empty() {
                        reject(&manifest.id, "Capacités non couvertes par les dépendances.");
                    }
                    chain
                }
                Err(_) => {
                    reject(&manifest.id, "Dépendance absente, cyclique ou trop profonde.");
                    Vec::new()
                }
            };
            let mail_covered = chain.iter().filter_map(|id| catalog.get(id)).any(|dependency| {
                dependency.capabilities.contains(&TalentCapability::SendMail)
            });
            for file in &source.files {
                let Ok(text) = std::str::from_utf8(&file.data) else { continue };
                if InstructionScanRule::is_suspicious(text) {
                    warnings.push(Diagnostic {
                        kind: DiagnosticKind::SuspiciousInstructions,
                        talent_id: manifest.id.clone(),
                        message: format!("Consignes suspectes à relire dans {}.", file.path),
                    });
                }
                if file.path.ends_with("SKILL.md") {
                    if InstructionLanguageRule::seems_not_english(text) {
                        warnings.push(Diagnostic {
                            kind: DiagnosticKind::NotEnglishInstructions,
                            talent_id: manifest.id.clone(),
                            message: format!("Consignes qui semblent ne pas être en anglais dans {}.", file.path),
                        });
                    }
                    if !InstructionLanguageRule::has_reply_language_instruction(text) {
                        warnings.push(Diagnostic {
                            kind: DiagnosticKind::MissingReplyLanguageInstruction,
                            talent_id: manifest.id.clone(),
                            message: format!("Aucune consigne de répondre dans la langue de la personne dans {}.", file.path),
                        });
                    }
                }
                if file.path.ends_with("SKILL.md") && InstructionScanRule::evokes_mail_sending(text) && !mail_covered {
                    warnings.push(Diagnostic {
                        kind: DiagnosticKind::MailSendingWithoutCapability,
                        talent_id: manifest.id.clone(),
                        message: format!(
                            "Envoi de mail évoqué sans send-mail dans la chaîne de dépendances ({}).",
                            file.path
                        ),
                    });
                }
            }
        }
        ValidationReport { errors, warnings }
    }

    /// The manifest read as a catalog one. A manifest of an exported Talent has no GitHub account yet (absent or
    /// empty list): it is read without the key and the second result tells it, for a warning instead of an error.
    fn parse_manifest(bytes: &[u8]) -> (Result<TalentManifest, TalentManifestError>, bool) {
        let first = TalentManifestRule::parse(bytes, TalentSource::Catalog);
        if !matches!(&first, Err(TalentManifestError::InvalidField(field)) if field == "author.github") {
            return (first, false);
        }
        let Ok(mut value) = serde_json::from_slice::<serde_json::Value>(bytes) else { return (first, false) };
        let Some(author) = value.get_mut("author").and_then(serde_json::Value::as_object_mut) else { return (first, false) };
        let empty = match author.get("github") {
            None => true,
            Some(github) => github.as_array().is_some_and(Vec::is_empty),
        };
        if !empty {
            return (first, false);
        }
        author.remove("github");
        let Ok(rewritten) = serde_json::to_vec(&value) else { return (first, false) };
        (TalentManifestRule::parse(&rewritten, TalentSource::LocalFolder), true)
    }

    /// Path, size and SHA-256 of each file, as the index lists them.
    pub fn file_descriptors(files: &[CatalogFile]) -> Vec<TalentIndexFile> {
        files
            .iter()
            .map(|file| TalentIndexFile {
                path: file.path.clone(),
                size: file.data.len(),
                sha256: hex(&Sha256::digest(&file.data)),
            })
            .collect()
    }

    fn check_files(source: &CatalogSource) -> Result<Vec<SkillClaim>, CatalogError> {
        TalentFileVerification::validate(&Self::file_descriptors(&source.files))
            .map_err(|_| CatalogError::new("fichiers hors du contrat."))?;
        for file in &source.files {
            let plain = file.kind == FileKind::Regular
                && file.permissions & 0o111 == 0
                && !is_executable(&file.data)
                && std::str::from_utf8(&file.data).is_ok();
            if !plain {
                return Err(CatalogError::new("lien, fichier spécial, exécutable ou contenu non UTF-8."));
            }
        }
        SkillClaimRule::claims(&source.directory_id, &source.files)
    }
}

fn is_executable(data: &[u8]) -> bool {
    data.starts_with(b"#!") || data.get(..4).is_some_and(|prefix| EXECUTABLE_HEADERS.iter().any(|header| header == prefix))
}
