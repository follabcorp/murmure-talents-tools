//! The tool of the Catalogue of the Talents (ADR-0023, ADR-0024): `validate`, `build-index`, `sign`, `delegate`, `publication-key` and `publish`, on the contract
//! the Relay reads (`murmure-talents-kit`). Contract: docs/murmure/catalogue-talents.md.

mod adapters;
mod application;
mod domain;

pub use adapters::{run, CliResult};
pub use application::{CatalogDisk, CatalogSnapshot};
pub use domain::{
    CatalogError, CatalogFile, CatalogIndexArtifact, CatalogIndexRule, CatalogSource,
    CatalogValidationRule, CodeownersEntry, DelegationDocumentRule, PublicationDecisionRule, PublicationKeyRule,
    PublicationSequenceRule, CodeownersRule, Diagnostic, DiagnosticKind, FileKind, SigningKeyFacts, SigningKeyRule,
    ValidationReport,
};
