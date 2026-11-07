//! What the integration suites of the tool share: running the binary, a path as text, and the Skill of the test Talent.
#![allow(dead_code)]

use std::process::{Command, Output};
use std::path::Path;

pub const SKILL: &str = "---\nname: travel-coach\ndescription: Plan a trip.\n---\nSuggest a local itinerary.\n";

pub fn tool(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_murmure-talents")).args(arguments).output().expect("the tool runs")
}

pub fn path(path: &Path) -> &str {
    path.to_str().expect("a UTF-8 path")
}
