//! The contract of the Talents, alone (ADR-0023, ADR-0024): the manifest `talent.json`, its closed vocabularies and
//! bounds, the dependencies between Talents, the signed index of a Catalogue, the verification of its signature and of
//! the files it lists, and the manifests shipped with the Relay. Pure rules: no network, no disk, no runtime, and
//! nothing from the Relay. The Relay (`relay-talents`) and the tool of the Catalogue read and sign with the same code.
//! Contract: docs/murmure/talents.md and docs/murmure/catalogue-talents.md.

mod domain;

pub use domain::*;
