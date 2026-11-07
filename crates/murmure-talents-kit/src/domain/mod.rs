//! Domain layer: the pure rules of the contract of the Talents (docs/murmure/talents.md, docs/murmure/catalogue-talents.md).

mod covered_apps_file;
mod iso_date;
mod semantic_version;
mod talent_built_in_resource;
mod talent_bypass_rule;
mod talent_catalog_delegation;
mod talent_catalog_diff_rule;
mod talent_dependency_rule;
mod talent_export_projection;
mod talent_file_verification;
mod talent_guard_events;
mod talent_index;
mod talent_index_verification;
mod talent_manifest;
mod talent_manifest_rule;
mod talent_operation_error;
mod talent_revocation_reason;
mod talent_vocabulary;

pub use covered_apps_file::CoveredAppsFile;
pub use semantic_version::SemanticVersion;
pub use talent_built_in_resource::{TalentBuiltInResource, BUILT_IN_MANIFESTS};
pub use talent_bypass_rule::{CoveredApp, TalentBypassRefusal, TalentBypassRule};
pub use talent_catalog_delegation::{
    TalentCatalogDelegation, TalentCatalogTrust, TalentCatalogTrustError, TalentDelegationError,
    TalentVerifiedCatalog,
};
pub use talent_catalog_diff_rule::{
    TalentCatalogDiff, TalentCatalogDiffRule, TalentChange, TalentChangeKind, TalentDiffError,
    TalentDiffErrorKind, TalentSnapshot,
};
pub use talent_dependency_rule::{TalentDependencyError, TalentDependencyRule};
pub use talent_export_projection::{TalentExportError, TalentExportFile, TalentExportProjection};
pub use talent_file_verification::{hex, SkillFiles, TalentBundleRule, TalentFileVerification};
pub use talent_guard_events::TalentGuardEvents;
pub use talent_index::{
    TalentIndex, TalentIndexEntry, TalentIndexError, TalentIndexFile, TalentIndexRevocation,
};
pub use talent_index_verification::{TalentCatalogKeyRule, TalentIndexVerification};
pub use talent_manifest::{
    LocalizedText, TalentAuthor, TalentKind, TalentManifest, TalentManifestError, TalentOrigin,
    TalentOriginKind, TalentPlatform, TalentSource,
};
pub use talent_manifest_rule::{is_valid_identifier, TalentManifestRule};
pub use talent_operation_error::TalentOperationError;
pub use talent_revocation_reason::TalentRevocationReason;
pub use talent_vocabulary::{CapabilityLevel, TalentCapability, TalentNeed};

#[cfg(test)]
mod covered_apps_file_tests;
#[cfg(any(test, feature = "test-support"))]
pub mod fixtures;
#[cfg(test)]
mod semantic_version_tests;
#[cfg(test)]
mod talent_built_in_resource_tests;
#[cfg(test)]
mod talent_bypass_rule_tests;
#[cfg(test)]
mod talent_catalog_delegation_tests;
#[cfg(test)]
mod talent_catalog_diff_rule_tests;
#[cfg(test)]
mod talent_dependency_rule_tests;
#[cfg(test)]
mod talent_export_projection_tests;
#[cfg(test)]
mod talent_file_verification_tests;
#[cfg(test)]
mod talent_guard_events_tests;
#[cfg(test)]
mod talent_index_tests;
#[cfg(test)]
mod talent_index_verification_tests;
#[cfg(test)]
mod talent_manifest_rule_tests;
#[cfg(test)]
mod talent_vocabulary_tests;
