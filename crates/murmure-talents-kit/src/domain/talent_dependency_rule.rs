//! The dependencies between Talents (docs/murmure/talents.md, « Règles pures » 4).

use std::collections::{BTreeMap, BTreeSet};

use super::{TalentCapability, TalentKind, TalentManifest};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TalentDependencyError {
    UnknownRequirement,
    Cycle,
    TooDeep,
}

/// The Talents among which the suppliers of a capability are looked for, each group already compatible with the
/// platform and the version of the Relay.
#[derive(Debug, Clone, Default)]
pub struct CapabilitySupplierPool {
    /// The Talents learned (connected).
    pub learned: Vec<TalentManifest>,
    /// The built-in « Par Murmure » Talents available on the platform.
    pub built_ins: Vec<TalentManifest>,
    /// The Talents of the Catalogue compatible with this Relay.
    pub catalog: Vec<TalentManifest>,
}

/// A capability that no learned Talent supplies, and who could.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UncoveredCapability {
    pub capability: TalentCapability,
    /// The suppliers to connect, in the order of `TalentDependencyRule::proposed_suppliers`.
    pub suppliers: Vec<String>,
}

pub struct TalentDependencyRule;

impl TalentDependencyRule {
    /// The Talents to learn in one gesture, dependencies first, without those already learned. A chain is at most 4
    /// levels deep, the Talent itself being the first.
    pub fn missing(
        id: &str,
        learned: &BTreeSet<String>,
        catalog: &BTreeMap<String, TalentManifest>,
    ) -> Result<Vec<String>, TalentDependencyError> {
        let mut visit = Visit {
            learned,
            catalog,
            visiting: Vec::new(),
            result: Vec::new(),
        };
        visit.talent(id, 1)?;
        Ok(visit.result)
    }

    /// The capabilities of a textual Talent that its direct dependencies do not declare; nothing for a Talent with
    /// code.
    pub fn uncovered_capabilities(
        manifest: &TalentManifest,
        catalog: &BTreeMap<String, TalentManifest>,
    ) -> BTreeSet<TalentCapability> {
        if manifest.kind != TalentKind::TextOnly {
            return BTreeSet::new();
        }
        let covered: BTreeSet<TalentCapability> = manifest
            .requires
            .iter()
            .filter_map(|id| catalog.get(id))
            .flat_map(|dependency| dependency.capabilities.iter().copied())
            .collect();
        manifest
            .capabilities
            .iter()
            .copied()
            .filter(|capability| !covered.contains(capability))
            .collect()
    }

    /// The Talents with code that declare each capability: identifiers sorted, a capability nobody declares absent.
    pub fn suppliers(manifests: &[TalentManifest]) -> BTreeMap<TalentCapability, Vec<String>> {
        let mut suppliers: BTreeMap<TalentCapability, Vec<String>> = BTreeMap::new();
        for manifest in manifests
            .iter()
            .filter(|manifest| manifest.kind == TalentKind::WithCode)
        {
            for capability in &manifest.capabilities {
                let ids = suppliers.entry(*capability).or_default();
                if !ids.contains(&manifest.id) {
                    ids.push(manifest.id.clone());
                }
            }
        }
        suppliers.values_mut().for_each(|ids| ids.sort());
        suppliers
    }

    /// The capabilities of a Talent declared by capability (textual, no `requires`) that no Talent with code among
    /// `learned` declares; nothing for any other Talent.
    pub fn uncovered_by_suppliers(
        manifest: &TalentManifest,
        learned: &[TalentManifest],
    ) -> BTreeSet<TalentCapability> {
        if !Self::is_declared_by_capability(manifest) {
            return BTreeSet::new();
        }
        let suppliers = Self::suppliers(learned);
        manifest
            .capabilities
            .iter()
            .copied()
            .filter(|capability| !suppliers.contains_key(capability))
            .collect()
    }

    /// The suppliers of `capability` to offer: learned first, then built-in, then Catalogue, Talents with code only,
    /// without duplicate, sorted by identifier in each group. Empty when none exists.
    pub fn proposed_suppliers(
        capability: TalentCapability,
        pool: &CapabilitySupplierPool,
    ) -> Vec<String> {
        let mut proposed: Vec<String> = Vec::new();
        for group in [&pool.learned, &pool.built_ins, &pool.catalog] {
            let mut ids: Vec<String> = Self::suppliers(group)
                .remove(&capability)
                .unwrap_or_default();
            ids.retain(|id| !proposed.contains(id));
            proposed.extend(ids);
        }
        proposed
    }

    /// A textual Talent without `requires`: its capabilities must be supplied by Talents with code.
    pub fn is_declared_by_capability(manifest: &TalentManifest) -> bool {
        manifest.kind == TalentKind::TextOnly && manifest.requires.is_empty()
    }

    /// The learned Talents that stop working when `id` is forgotten, transitively, in the order learned.
    pub fn dependents(id: &str, learned: &[TalentManifest]) -> Vec<String> {
        let mut affected: BTreeSet<&str> = BTreeSet::from([id]);
        let supplied_before = Self::suppliers(learned);
        loop {
            let before = affected.len();
            let remaining: Vec<(usize, TalentManifest)> = learned
                .iter()
                .enumerate()
                .filter(|(_, talent)| !affected.contains(talent.id.as_str()))
                .map(|(position, talent)| (position, talent.clone()))
                .collect();
            let only: Vec<TalentManifest> = remaining.iter().map(|(_, talent)| talent.clone()).collect();
            let supplied_after = Self::suppliers(&only);
            let mut newly: Vec<usize> = Vec::new();
            for (position, talent) in &remaining {
                let by_requires = talent
                    .requires
                    .iter()
                    .any(|required| affected.contains(required.as_str()));
                let by_capability = Self::is_declared_by_capability(talent)
                    && talent.capabilities.iter().any(|capability| {
                        supplied_before.contains_key(capability)
                            && !supplied_after.contains_key(capability)
                    });
                if by_requires || by_capability {
                    newly.push(*position);
                }
            }
            affected.extend(newly.into_iter().map(|position| learned[position].id.as_str()));
            if affected.len() == before {
                break;
            }
        }
        learned
            .iter()
            .filter(|talent| talent.id != id && affected.contains(talent.id.as_str()))
            .map(|talent| talent.id.clone())
            .collect()
    }
}

/// The deepest chain accepted, the Talent itself being the first level.
const DEPTH_MAX: usize = 4;

struct Visit<'a> {
    learned: &'a BTreeSet<String>,
    catalog: &'a BTreeMap<String, TalentManifest>,
    visiting: Vec<String>,
    result: Vec<String>,
}

impl Visit<'_> {
    fn talent(&mut self, id: &str, depth: usize) -> Result<(), TalentDependencyError> {
        let talent = self
            .catalog
            .get(id)
            .ok_or(TalentDependencyError::UnknownRequirement)?;
        if self.visiting.iter().any(|visiting| visiting == id) {
            return Err(TalentDependencyError::Cycle);
        }
        if depth > DEPTH_MAX {
            return Err(TalentDependencyError::TooDeep);
        }
        self.visiting.push(id.to_owned());
        for dependency in &talent.requires {
            self.talent(dependency, depth + 1)?;
        }
        self.visiting.pop();
        if !self.learned.contains(id) && !self.result.iter().any(|added| added == id) {
            self.result.push(id.to_owned());
        }
        Ok(())
    }
}
