//! JSON read strictly (ADR-0024): one UTF-8 value, no trailing bytes, no duplicate key at any depth, at most 32 levels.
//! A generic leaf crate: the contract of the channel (`murmure-protocol`) and the one of the Talents
//! (`murmure-talents-kit`) read their bodies with this one implementation.

mod domain;

pub use domain::*;
