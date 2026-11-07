//! Domain layer: the strict reading of JSON.

mod strict_json;

pub use strict_json::{parse_strict_json, StrictJsonError};

#[cfg(test)]
mod strict_json_tests;
