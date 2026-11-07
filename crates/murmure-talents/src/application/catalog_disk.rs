use std::collections::BTreeMap;
use std::path::Path;

use murmure_talents_kit::TalentIndexRevocation;

use crate::domain::{CatalogError, CatalogIndexArtifact, CatalogSource, SourceSnapshot};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogSnapshot {
    pub talents: Vec<CatalogSource>,
    pub featured: Vec<String>,
    pub revoked: Vec<TalentIndexRevocation>,
    /// `.github/CODEOWNERS` when present.
    pub codeowners: Option<String>,
}

/// The port of the disk (adapter: `adapters::fs_catalog_disk`). Every read and write is bounded and never follows a
/// link controlled by a contribution.
pub trait CatalogDisk {
    /// Only `talents/<id>/…` under `root` (what `diff` compares), within the same bounds.
    fn talents(&self, root: &Path) -> Result<Vec<CatalogSource>, CatalogError>;
    /// `talents/<id>/…`, `featured.json`, `revoked.json` under `root`, within the bounds of the contract.
    fn snapshot(&self, root: &Path) -> Result<CatalogSnapshot, CatalogError>;
    /// Writes the files, the `extra` ones (`path` → bytes) then `index.json` last, into a new folder distinct from
    /// `source`. A failure removes the folder it created.
    fn publish(
        &self,
        artifact: &CatalogIndexArtifact,
        extra: &BTreeMap<String, Vec<u8>>,
        output: &Path,
        source: &Path,
    ) -> Result<(), CatalogError>;
    /// Checks the key (facts → `SigningKeyRule`) before reading it, then reads the key and the index (parsed by the
    /// Kit). Returns the index bytes and the 32 private bytes.
    fn read_signing_input(
        &self,
        index: &Path,
        key: &Path,
        root: &Path,
    ) -> Result<(Vec<u8>, [u8; 32]), CatalogError>;
    /// Writes `<index>.sig` without replacing an existing file.
    fn write_signature(&self, signature: &[u8], index: &Path) -> Result<(), CatalogError>;
    /// Checks a private key (facts → `SigningKeyRule`, outside the repository holding `repository`) before reading
    /// its 32 bytes.
    fn read_private_key(&self, key: &Path, repository: &Path) -> Result<[u8; 32], CatalogError>;
    /// `delegation.json` and `delegation.json.sig` of a folder, as written by `write_delegation`.
    fn read_delegation(&self, folder: &Path) -> Result<(Vec<u8>, Vec<u8>), CatalogError>;
    /// Writes `delegation.json` and `delegation.json.sig` into a new folder; a failure removes the folder.
    fn write_delegation(&self, document: &[u8], signature: &[u8], output: &Path) -> Result<(), CatalogError>;
    /// Every file under a source folder (relative paths), within the bounds of the contract and without following
    /// any link.
    fn source_files(&self, source: &Path) -> Result<SourceSnapshot, CatalogError>;
    /// Writes the proposals as a Catalogue root in a new folder distinct from `source`: `talents/<id>/…`, empty
    /// `featured.json` and `revoked.json`. A failure removes the folder.
    fn write_proposals(&self, talents: &[CatalogSource], output: &Path, source: &Path) -> Result<(), CatalogError>;
}
