//! The dependencies between Talents (docs/murmure/talents.md, « Règles pures » 4).

use std::collections::{BTreeMap, BTreeSet};

use super::{TalentCapability, TalentKind, TalentManifest};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TalentDependencyError {
    UnknownRequirement,
    Cycle,
    TooDeep,
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

    /// The learned Talents that stop working when `id` is forgotten, transitively, in the order learned.
    pub fn dependents(id: &str, learned: &[TalentManifest]) -> Vec<String> {
        let mut affected: BTreeSet<&str> = BTreeSet::from([id]);
        loop {
            let before = affected.len();
            for talent in learned {
                if !affected.contains(talent.id.as_str())
                    && talent
                        .requires
                        .iter()
                        .any(|required| affected.contains(required.as_str()))
                {
                    affected.insert(&talent.id);
                }
            }
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
