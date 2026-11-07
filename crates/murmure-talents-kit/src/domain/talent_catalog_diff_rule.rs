//! The comparison of two states of a Catalogue (docs/murmure/catalogue-talents.md): a Talent whose files change must
//! raise its `version`; capabilities and dependencies it gains are listed for the review. Pure: the snapshots are
//! already read.

use std::collections::{BTreeMap, BTreeSet};

use super::{SemanticVersion, TalentCapability};

/// One Talent of a Catalogue, as the comparison needs it: the folder it lives in, its manifest facts, and the SHA-256
/// (hex) of each of its files by path, `talent.json` included.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TalentSnapshot {
    pub directory_id: String,
    pub id: String,
    pub version: SemanticVersion,
    pub capabilities: Vec<TalentCapability>,
    pub requires: Vec<String>,
    pub files: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TalentDiffErrorKind {
    /// The manifest `id` of a folder differs between the two states.
    IdChanged,
    /// The `version` is lower in the head than in the base.
    VersionDecreased,
    /// A file changed, was added or removed while the `version` is equal.
    FilesChangedWithoutVersionBump,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TalentDiffError {
    pub directory_id: String,
    pub kind: TalentDiffErrorKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TalentChangeKind {
    Added,
    Removed,
    Modified,
}

/// What the head changes for one Talent. For an `Added` Talent, every capability and dependency counts as added.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TalentChange {
    pub directory_id: String,
    pub kind: TalentChangeKind,
    pub added_capabilities: Vec<TalentCapability>,
    pub added_requires: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TalentCatalogDiff {
    /// Sorted by folder, then kind.
    pub errors: Vec<TalentDiffError>,
    /// Only the Talents that are added, removed, or modified (a Talent identical in both states is absent), sorted
    /// by folder.
    pub changes: Vec<TalentChange>,
}

impl TalentCatalogDiff {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

pub struct TalentCatalogDiffRule;

impl TalentCatalogDiffRule {
    /// Compares the head with the base, folder by folder. A folder present in both states is an error when its `id`
    /// changes, when its `version` goes back, or when its files differ at an equal `version`; it is a change when its
    /// files differ. A folder only in the head is `Added`, only in the base is `Removed`.
    pub fn compare(base: &[TalentSnapshot], head: &[TalentSnapshot]) -> TalentCatalogDiff {
        let base: BTreeMap<&str, &TalentSnapshot> =
            base.iter().map(|talent| (talent.directory_id.as_str(), talent)).collect();
        let head: BTreeMap<&str, &TalentSnapshot> =
            head.iter().map(|talent| (talent.directory_id.as_str(), talent)).collect();
        let mut errors = Vec::new();
        let mut changes = Vec::new();
        let folders: BTreeSet<&str> = base.keys().chain(head.keys()).copied().collect();
        for folder in folders {
            let directory_id = folder.to_owned();
            match (base.get(folder), head.get(folder)) {
                (None, Some(after)) => changes.push(TalentChange {
                    directory_id,
                    kind: TalentChangeKind::Added,
                    added_capabilities: after.capabilities.clone(),
                    added_requires: after.requires.clone(),
                }),
                (Some(_), None) => changes.push(TalentChange {
                    directory_id,
                    kind: TalentChangeKind::Removed,
                    added_capabilities: Vec::new(),
                    added_requires: Vec::new(),
                }),
                (Some(before), Some(after)) => {
                    if before.id != after.id {
                        errors.push(TalentDiffError { directory_id: directory_id.clone(), kind: TalentDiffErrorKind::IdChanged });
                    }
                    let files_differ = before.files != after.files;
                    if after.version < before.version {
                        errors.push(TalentDiffError { directory_id: directory_id.clone(), kind: TalentDiffErrorKind::VersionDecreased });
                    } else if files_differ && after.version == before.version {
                        errors.push(TalentDiffError {
                            directory_id: directory_id.clone(),
                            kind: TalentDiffErrorKind::FilesChangedWithoutVersionBump,
                        });
                    }
                    if files_differ {
                        changes.push(TalentChange {
                            directory_id,
                            kind: TalentChangeKind::Modified,
                            added_capabilities: added(&before.capabilities, &after.capabilities),
                            added_requires: added(&before.requires, &after.requires),
                        });
                    }
                }
                (None, None) => {}
            }
        }
        TalentCatalogDiff { errors, changes }
    }
}

fn added<T: PartialEq + Clone>(before: &[T], after: &[T]) -> Vec<T> {
    after.iter().filter(|item| !before.contains(item)).cloned().collect()
}
