//! Domain layer: the pure rules of the Catalogue tool. No disk, no clock, no process.

mod catalog_diff_report;
mod catalog_error;
mod codeowners_rule;
mod delegation_document_rule;
mod catalog_index_rule;
mod catalog_source;
mod catalog_validation_rule;
mod instruction_language_rule;
mod instruction_scan_rule;
mod skill_conversion_rule;
mod source_triage_rule;
mod publication_decision_rule;
mod publication_key_rule;
mod publication_sequence_rule;
mod signing_key_rule;
mod skill_claim_rule;

pub use catalog_diff_report::CatalogDiffReport;
pub use catalog_error::CatalogError;
pub use codeowners_rule::{CodeownersEntry, CodeownersRule};
pub use catalog_index_rule::{CatalogIndexArtifact, CatalogIndexRule};
pub use catalog_source::{
    CatalogFile, CatalogSource, Diagnostic, DiagnosticKind, FileKind, SourceSnapshot, ValidationReport,
};
pub use catalog_validation_rule::CatalogValidationRule;
pub use delegation_document_rule::DelegationDocumentRule;
pub use publication_decision_rule::PublicationDecisionRule;
pub use publication_key_rule::PublicationKeyRule;
pub use publication_sequence_rule::PublicationSequenceRule;
pub use instruction_language_rule::InstructionLanguageRule;
pub use instruction_scan_rule::InstructionScanRule;
pub use skill_conversion_rule::{ConversionReport, SkillConversionRule};
#[cfg(test)]
pub use skill_conversion_rule::ConversionStatus;
pub use source_triage_rule::SourceTriageRule;
#[cfg(test)]
pub use source_triage_rule::SourceTriage;
pub use signing_key_rule::{SigningKeyFacts, SigningKeyRule};
pub use skill_claim_rule::{SkillClaim, SkillClaimRule};

#[cfg(test)]
mod catalog_diff_report_tests;
#[cfg(test)]
mod catalog_index_rule_tests;
#[cfg(test)]
mod codeowners_rule_tests;
#[cfg(test)]
mod catalog_validation_rule_tests;
#[cfg(test)]
mod instruction_language_rule_tests;
#[cfg(test)]
mod instruction_scan_rule_tests;
#[cfg(test)]
mod skill_conversion_rule_tests;
#[cfg(test)]
mod source_triage_rule_tests;
#[cfg(test)]
mod delegation_document_rule_tests;
#[cfg(test)]
mod publication_decision_rule_tests;
#[cfg(test)]
mod publication_key_rule_tests;
#[cfg(test)]
mod publication_sequence_rule_tests;
#[cfg(test)]
mod signing_key_rule_tests;
#[cfg(test)]
mod skill_claim_rule_tests;
#[cfg(test)]
mod test_fixtures;
