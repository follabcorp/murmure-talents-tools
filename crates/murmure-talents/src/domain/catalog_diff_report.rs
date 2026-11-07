//! The comparison of two Catalogue roots, as the pre-review shows it (docs/murmure/catalogue-talents.md): the Kit's rule
//! applied to snapshots built from the sources the disk adapter read. Pure.

use murmure_talents_kit::{
    hex, TalentCatalogDiff, TalentCatalogDiffRule, TalentChangeKind, TalentDiffErrorKind, TalentManifestRule,
    TalentSnapshot, TalentSource,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::CatalogSource;

pub struct CatalogDiffReport {
    pub diff: TalentCatalogDiff,
    /// Folders of the head whose manifest cannot be read: an error, and they take no part in the comparison.
    pub unreadable: Vec<String>,
}

impl CatalogDiffReport {
    /// The manifests are read without requiring `author.github`: the base may predate it. A base folder that cannot be
    /// read is ignored; a head folder that cannot be read is reported.
    pub fn build(base: &[CatalogSource], head: &[CatalogSource]) -> Self {
        let base: Vec<TalentSnapshot> = base.iter().filter_map(Self::snapshot).collect();
        let mut unreadable = Vec::new();
        let head: Vec<TalentSnapshot> = head
            .iter()
            .filter_map(|source| {
                let snapshot = Self::snapshot(source);
                if snapshot.is_none() {
                    unreadable.push(source.directory_id.clone());
                }
                snapshot
            })
            .collect();
        unreadable.sort();
        Self { diff: TalentCatalogDiffRule::compare(&base, &head), unreadable }
    }

    fn snapshot(source: &CatalogSource) -> Option<TalentSnapshot> {
        let manifest = TalentManifestRule::parse(&source.manifest_json, TalentSource::LocalFolder).ok()?;
        let mut files = std::collections::BTreeMap::new();
        files.insert("talent.json".to_owned(), hex(&Sha256::digest(&source.manifest_json)));
        for file in &source.files {
            files.insert(file.path.clone(), hex(&Sha256::digest(&file.data)));
        }
        Some(TalentSnapshot {
            directory_id: source.directory_id.clone(),
            id: manifest.id,
            version: manifest.version,
            capabilities: manifest.capabilities,
            requires: manifest.requires,
            files,
        })
    }

    pub fn is_valid(&self) -> bool {
        self.diff.is_valid() && self.unreadable.is_empty()
    }

    /// True when a Talent gains a capability: two approvals of Maintainers are then required.
    pub fn adds_capabilities(&self) -> bool {
        self.diff.changes.iter().any(|change| !change.added_capabilities.is_empty())
    }

    fn errors(&self) -> Vec<(String, &'static str, &'static str)> {
        let unreadable = self.unreadable.iter().map(|id| {
            (id.clone(), "invalidManifest", "Manifeste illisible dans la tête de la PR.")
        });
        let compared = self.diff.errors.iter().map(|error| {
            let (kind, message) = match error.kind {
                TalentDiffErrorKind::IdChanged => ("idChanged", "L’identifiant du Talent a changé."),
                TalentDiffErrorKind::VersionDecreased => ("versionDecreased", "La version recule."),
                TalentDiffErrorKind::FilesChangedWithoutVersionBump => (
                    "filesChangedWithoutVersionBump",
                    "Des fichiers changent sans que la version monte.",
                ),
            };
            (error.directory_id.clone(), kind, message)
        });
        let mut errors: Vec<_> = unreadable.chain(compared).collect();
        errors.sort_by(|left, right| left.0.cmp(&right.0));
        errors
    }

    /// « Comparaison du Catalogue », the verdict, `ERREUR [id] …`, then each changed Talent with what it adds; the
    /// approvals line when a capability is added.
    pub fn text(&self) -> String {
        let verdict = if self.is_valid() { "Comparaison : conforme." } else { "Comparaison : refusée." };
        let mut lines = vec!["Comparaison du Catalogue".to_owned(), verdict.to_owned()];
        lines.extend(self.errors().into_iter().map(|(id, _, message)| format!("ERREUR [{id}] {message}")));
        for change in &self.diff.changes {
            let label = match change.kind {
                TalentChangeKind::Added => "ajouté",
                TalentChangeKind::Removed => "retiré",
                TalentChangeKind::Modified => "modifié",
            };
            lines.push(format!("Talent {label} [{}]", change.directory_id));
            if !change.added_capabilities.is_empty() {
                let names: Vec<&str> = change.added_capabilities.iter().map(|capability| capability.wire()).collect();
                lines.push(format!("  Capacités ajoutées : {}", names.join(", ")));
            }
            if !change.added_requires.is_empty() {
                lines.push(format!("  Dépendances ajoutées : {}", change.added_requires.join(", ")));
            }
        }
        if self.adds_capabilities() {
            lines.push("Capacités ajoutées : deux approbations de Mainteneurs requises.".to_owned());
        }
        lines.join("\n") + "\n"
    }

    /// `{valid, errors:[{talentId, kind, message}], talents:[{talentId, change, addedCapabilities, addedDependencies}]}`
    pub fn json(&self) -> String {
        let errors: Vec<Value> = self
            .errors()
            .into_iter()
            .map(|(id, kind, message)| json!({"talentId": id, "kind": kind, "message": message}))
            .collect();
        let talents: Vec<Value> = self
            .diff
            .changes
            .iter()
            .map(|change| {
                let kind = match change.kind {
                    TalentChangeKind::Added => "added",
                    TalentChangeKind::Removed => "removed",
                    TalentChangeKind::Modified => "modified",
                };
                json!({
                    "talentId": change.directory_id,
                    "change": kind,
                    "addedCapabilities": change.added_capabilities.iter().map(|capability| capability.wire()).collect::<Vec<_>>(),
                    "addedDependencies": change.added_requires,
                })
            })
            .collect();
        json!({"valid": self.is_valid(), "errors": errors, "talents": talents}).to_string() + "\n"
    }
}
