//! Why an operation on the Talents of a user is refused (docs/murmure/talents.md); `InvalidResources` is also what the
//! verification of the files of a Talent answers.

use super::{TalentDependencyError, UncoveredCapability};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TalentOperationError {
    LimitReached,
    ResourceConflict,
    LevelNotSupported,
    NotFound,
    InvalidResources,
    /// Forgetting is refused while these Talents depend on the one forgotten.
    Dependents(Vec<String>),
    /// Learning is refused until a Talent with code supplies each of these capabilities.
    CapabilitiesUncovered(Vec<UncoveredCapability>),
    RollbackFailed,
    Dependency(TalentDependencyError),
}
