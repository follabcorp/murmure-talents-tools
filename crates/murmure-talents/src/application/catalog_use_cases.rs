use std::collections::BTreeMap;
use std::path::Path;

use base64::{engine::general_purpose::STANDARD, Engine};

use ed25519_dalek::{Signer, SigningKey};
use murmure_talents_kit::{TalentCatalogDelegation, TalentCatalogTrust, TalentManifest};

use super::{CatalogDisk, CatalogSnapshot};
use murmure_talents_kit::TalentSource;

use crate::domain::{
    CatalogDiffReport, CatalogError, CatalogIndexRule, CatalogSource, CatalogValidationRule, CodeownersEntry,
    CodeownersRule, ConversionReport, DelegationDocumentRule, Diagnostic, DiagnosticKind, PublicationDecisionRule, PublicationKeyRule,
    PublicationSequenceRule, SkillConversionRule, ValidationReport,
};

/// Base URL and sequence the index is rehearsed with when only `validate` runs.
const REHEARSAL_BASE_URL: &str = "https://catalogue.example.invalid/";

/// `validate <root>`: the report of the pre-review; a valid Catalogue must also make a readable index.
pub fn validate_catalog(
    disk: &dyn CatalogDisk,
    root: &Path,
    now: i64,
    built_in: &[TalentManifest],
) -> Result<ValidationReport, CatalogError> {
    validate_snapshot(&disk.snapshot(root)?, now, built_in)
}

fn validate_snapshot(
    snapshot: &CatalogSnapshot,
    now: i64,
    built_in: &[TalentManifest],
) -> Result<ValidationReport, CatalogError> {
    let mut report = CatalogValidationRule::validate(&snapshot.talents, built_in);
    if report.is_valid() {
        if let Some(present) = &snapshot.codeowners {
            if !CodeownersRule::is_current(&CodeownersRule::expected(&codeowners_entries(&snapshot.talents)?), present) {
                report.errors.push(Diagnostic {
                    kind: DiagnosticKind::CodeownersOutdated,
                    talent_id: "CODEOWNERS".to_owned(),
                    message: "Le CODEOWNERS présent diffère de celui des manifestes ; régénérer avec `murmure-talents codeowners`.".to_owned(),
                });
                return Ok(report);
            }
        }
        // A Talent still without GitHub account (converted or exported) cannot make an index yet: only its warning.
        let incomplete = report.warnings.iter().any(|warning| warning.kind == DiagnosticKind::MissingAuthorGithub);
        if !incomplete {
            CatalogIndexRule::build(
                &snapshot.talents, &snapshot.featured, &snapshot.revoked, REHEARSAL_BASE_URL, 0, now, built_in,
            )?;
        }
    }
    Ok(report)
}

/// `convert <source>`: triage and conversion of a local source folder into proposals to review; with `output`, the
/// proposals are also written there as a new Catalogue root. Nothing is fetched and nothing is executed.
pub fn convert_source(
    disk: &dyn CatalogDisk,
    source: &Path,
    id: Option<&str>,
    output: Option<&Path>,
) -> Result<ConversionReport, CatalogError> {
    let snapshot = disk.source_files(source)?;
    let report = SkillConversionRule::convert(&snapshot.files, &snapshot.oversized, id);
    if let Some(output) = output {
        disk.write_proposals(&report.talents, output, source)?;
    }
    Ok(report)
}

/// `build-index`: validates, builds the index and publishes it into `output`. An invalid catalog publishes nothing.
pub fn build_index(
    disk: &dyn CatalogDisk,
    root: &Path,
    output: &Path,
    base_url: &str,
    sequence: &str,
    now: i64,
    built_in: &[TalentManifest],
) -> Result<ValidationReport, CatalogError> {
    let snapshot = disk.snapshot(root)?;
    let report = CatalogValidationRule::validate(&snapshot.talents, built_in);
    if !report.is_valid() {
        return Ok(report);
    }
    let sequence: u64 = sequence.parse().map_err(|_| CatalogError::new("URL de base ou séquence invalide."))?;
    let artifact = CatalogIndexRule::build(
        &snapshot.talents, &snapshot.featured, &snapshot.revoked, base_url, sequence, now, built_in,
    )?;
    disk.publish(&artifact, &BTreeMap::new(), output, root)?;
    Ok(report)
}

/// `sign`: the detached Ed25519 signature (Base64 and a newline) of the exact bytes of the index, written beside it.
pub fn sign_index(
    disk: &dyn CatalogDisk,
    root: &Path,
    index: &Path,
    key: &Path,
) -> Result<(), CatalogError> {
    let (data, secret) = disk.read_signing_input(index, key, root)?;
    disk.write_signature(encoded_signature(&secret, &data).as_bytes(), index)
}

/// The Ed25519 signature of the exact bytes, in Base64 with a final newline (the format of every `.sig`).
fn encoded_signature(secret: &[u8; 32], data: &[u8]) -> String {
    STANDARD.encode(SigningKey::from_bytes(secret).sign(data).to_bytes()) + "\n"
}

/// What `delegate` needs besides the disk and the clock.
pub struct DelegateRequest<'a> {
    pub root_key: &'a Path,
    /// Where the human works: the root key must be outside this repository.
    pub repository: &'a Path,
    pub publication_public_key: &'a str,
    pub sequence: u64,
    pub valid_days: u32,
    pub output: &'a Path,
}

/// `delegate`: signs a `delegation.json` for the publication key with the root key and writes both files into
/// `output`.
pub fn delegate(disk: &dyn CatalogDisk, request: &DelegateRequest<'_>, now: i64) -> Result<(), CatalogError> {
    let document =
        DelegationDocumentRule::build(request.publication_public_key, request.sequence, now, request.valid_days)?;
    let secret = disk.read_private_key(request.root_key, request.repository)?;
    disk.write_delegation(&document, encoded_signature(&secret, &document).as_bytes(), request.output)
}

/// `publication-key --public`: the Base64 public key of a private key file.
pub fn publication_public_key(disk: &dyn CatalogDisk, key: &Path, repository: &Path) -> Result<String, CatalogError> {
    Ok(PublicationKeyRule::public_key(&disk.read_private_key(key, repository)?))
}

/// What `publish` needs besides the disk and the clock.
pub struct PublishRequest<'a> {
    pub root: &'a Path,
    pub output: &'a Path,
    pub base_url: &'a str,
    pub sequence: &'a str,
    pub key: &'a Path,
    pub delegation: &'a Path,
    pub root_public_key: &'a str,
}

/// `publish`: validate, build the index, sign it, add the delegation, then check the whole chain with the Kit's
/// `TalentCatalogTrust` against the root key. Everything is computed in memory first: nothing is written unless the
/// chain is trusted, and a failed write removes the folder.
pub fn publish_catalog(
    disk: &dyn CatalogDisk,
    request: &PublishRequest<'_>,
    now: i64,
    built_in: &[TalentManifest],
) -> Result<ValidationReport, CatalogError> {
    let snapshot = disk.snapshot(request.root)?;
    let report = validate_snapshot(&snapshot, now, built_in)?;
    if !report.is_valid() {
        return Ok(report);
    }
    let sequence = PublicationSequenceRule::resolve(request.sequence, now)?;
    let root_key = STANDARD
        .decode(request.root_public_key)
        .map_err(|_| CatalogError::new("La clé racine publique doit être en base64."))?;
    let secret = disk.read_private_key(request.key, request.root)?;
    let (delegation_bytes, delegation_signature) = disk.read_delegation(request.delegation)?;
    let delegation = TalentCatalogDelegation::parse(&delegation_bytes)
        .map_err(|_| CatalogError::new("delegation.json invalide."))?;
    PublicationDecisionRule::decide(&delegation, &PublicationKeyRule::public_bytes(&secret), now)?;

    let artifact = CatalogIndexRule::build(
        &snapshot.talents, &snapshot.featured, &snapshot.revoked, request.base_url, sequence, now, built_in,
    )?;
    let index_signature = encoded_signature(&secret, &artifact.index_data);
    let decode = |bytes: &[u8]| {
        std::str::from_utf8(bytes)
            .ok()
            .and_then(|text| STANDARD.decode(text.trim()).ok())
            .ok_or_else(|| CatalogError::new("Signature de délégation illisible."))
    };
    TalentCatalogTrust::verify(
        &delegation_bytes,
        &decode(&delegation_signature)?,
        &artifact.index_data,
        &decode(index_signature.as_bytes())?,
        &root_key,
        now,
        None,
        None,
    )
    .map_err(|error| CatalogError::new(format!("La chaîne de confiance n’est pas vérifiée ({error:?}).")))?;

    let extra = BTreeMap::from([
        ("delegation.json".to_owned(), delegation_bytes),
        ("delegation.json.sig".to_owned(), delegation_signature),
        ("index.json.sig".to_owned(), index_signature.into_bytes()),
    ]);
    disk.publish(&artifact, &extra, request.output, request.root)?;
    Ok(report)
}

fn codeowners_entries(talents: &[CatalogSource]) -> Result<Vec<CodeownersEntry>, CatalogError> {
    talents
        .iter()
        .map(|source| {
            murmure_talents_kit::TalentManifestRule::parse(&source.manifest_json, TalentSource::Catalog)
                .map(|manifest| CodeownersEntry { talent_id: source.directory_id.clone(), github: manifest.author.github })
                .map_err(|_| CatalogError::new("Manifeste invalide : CODEOWNERS non généré."))
        })
        .collect()
}

/// `codeowners <root>`: the expected `CODEOWNERS`, from the Authors of the manifests.
pub fn codeowners(disk: &dyn CatalogDisk, root: &Path) -> Result<String, CatalogError> {
    Ok(CodeownersRule::expected(&codeowners_entries(&disk.snapshot(root)?.talents)?))
}

/// `diff <base> <head>`: the two roots are read by the disk, the Kit's rule compares them.
pub fn diff_catalogs(disk: &dyn CatalogDisk, base: &Path, head: &Path) -> Result<CatalogDiffReport, CatalogError> {
    Ok(CatalogDiffReport::build(&disk.talents(base)?, &disk.talents(head)?))
}
