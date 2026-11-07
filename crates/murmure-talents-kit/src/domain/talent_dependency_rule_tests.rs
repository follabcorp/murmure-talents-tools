//! Contract: « Règles pures » 4.

use std::collections::{BTreeMap, BTreeSet};

use super::fixtures::manifest;
use super::{
    TalentCapability, TalentDependencyError, TalentDependencyRule, TalentKind, TalentManifest,
};

fn catalog(talents: Vec<TalentManifest>) -> BTreeMap<String, TalentManifest> {
    talents
        .into_iter()
        .map(|talent| (talent.id.clone(), talent))
        .collect()
}

fn learned(ids: &[&str]) -> BTreeSet<String> {
    ids.iter().map(|id| (*id).to_owned()).collect()
}

#[test]
fn the_missing_talents_come_dependencies_first_and_skip_the_learned_ones() {
    let catalog = catalog(vec![
        manifest("root", "1.0.0", &[], &["middle", "leaf"]),
        manifest("middle", "1.0.0", &[], &["leaf"]),
        manifest("leaf", "1.0.0", &[], &[]),
    ]);
    assert_eq!(
        TalentDependencyRule::missing("root", &learned(&[]), &catalog),
        Ok(vec!["leaf".into(), "middle".into(), "root".into()])
    );
    assert_eq!(
        TalentDependencyRule::missing("root", &learned(&["leaf"]), &catalog),
        Ok(vec!["middle".into(), "root".into()])
    );
}

#[test]
fn an_unknown_requirement_a_cycle_and_a_chain_of_five_levels_are_refused() {
    let chain = |length: usize| {
        catalog(
            (0..length)
                .map(|index| {
                    let next = format!("t{}", index + 1);
                    let requires: Vec<&str> = if index + 1 < length {
                        vec![next.as_str()]
                    } else {
                        vec![]
                    };
                    manifest(&format!("t{index}"), "1.0.0", &[], &requires)
                })
                .collect(),
        )
    };
    assert_eq!(
        TalentDependencyRule::missing("t0", &learned(&[]), &chain(4)).map(|ids| ids.len()),
        Ok(4),
        "4 levels: the bound"
    );
    assert_eq!(
        TalentDependencyRule::missing("t0", &learned(&[]), &chain(5)),
        Err(TalentDependencyError::TooDeep)
    );
    assert_eq!(
        TalentDependencyRule::missing(
            "t0",
            &learned(&[]),
            &catalog(vec![manifest("t0", "1.0.0", &[], &["absent"])])
        ),
        Err(TalentDependencyError::UnknownRequirement)
    );
    assert_eq!(
        TalentDependencyRule::missing("nobody", &learned(&[]), &catalog(vec![])),
        Err(TalentDependencyError::UnknownRequirement)
    );
    let cycle = catalog(vec![
        manifest("a", "1.0.0", &[], &["b"]),
        manifest("b", "1.0.0", &[], &["a"]),
    ]);
    assert_eq!(
        TalentDependencyRule::missing("a", &learned(&[]), &cycle),
        Err(TalentDependencyError::Cycle)
    );
}

#[test]
fn a_textual_talent_must_have_its_capabilities_covered_by_its_direct_dependencies() {
    use TalentCapability::*;
    let catalog = catalog(vec![manifest("mes-mails", "1.0.0", &[ReadMail], &[])]);
    let talent = manifest("trier", "1.0.0", &[ReadMail, WebSearch], &["mes-mails"]);
    assert_eq!(
        TalentDependencyRule::uncovered_capabilities(&talent, &catalog),
        BTreeSet::from([WebSearch])
    );
    let mut with_code = talent.clone();
    with_code.kind = TalentKind::WithCode;
    assert!(
        TalentDependencyRule::uncovered_capabilities(&with_code, &catalog).is_empty(),
        "a Talent with code carries its own access"
    );
}

#[test]
fn the_dependents_are_transitive_listed_in_the_order_learned_and_never_include_the_talent() {
    let learned = vec![
        manifest("far", "1.0.0", &[], &["near"]),
        manifest("base", "1.0.0", &[], &[]),
        manifest("near", "1.0.0", &[], &["base"]),
        manifest("other", "1.0.0", &[], &[]),
    ];
    assert_eq!(
        TalentDependencyRule::dependents("base", &learned),
        ["far", "near"]
    );
    assert!(TalentDependencyRule::dependents("other", &learned).is_empty());
}
