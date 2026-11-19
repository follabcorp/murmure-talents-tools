//! Contract: « Règles pures » 4.

use std::collections::{BTreeMap, BTreeSet};

use super::fixtures::{manifest, supplier};
use super::{
    CapabilitySupplierPool, TalentCapability, TalentDependencyError, TalentDependencyRule, TalentKind, TalentManifest,
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

fn text_talent(id: &str, capabilities: &[TalentCapability]) -> TalentManifest {
    manifest(id, "1.0.0", capabilities, &[])
}

#[test]
fn the_suppliers_of_a_capability_are_the_talents_with_code_that_declare_it_sorted_by_id() {
    use TalentCapability::*;
    let manifests = vec![
        supplier("mes-mails", &[ReadMail, SendMail]),
        supplier("ma-boite", &[ReadMail]),
        supplier("naviguer", &[BrowseWeb]),
        text_talent("trier", &[ReadMail]),
    ];
    let suppliers = TalentDependencyRule::suppliers(&manifests);
    assert_eq!(suppliers[&ReadMail], ["ma-boite", "mes-mails"]);
    assert_eq!(suppliers[&SendMail], ["mes-mails"]);
    assert_eq!(suppliers[&BrowseWeb], ["naviguer"]);
    assert!(!suppliers.contains_key(&WebSearch), "nobody declares it");
}

#[test]
fn a_capability_is_covered_by_one_supplier_by_two_and_uncovered_by_none() {
    use TalentCapability::*;
    let sorter = text_talent("trier", &[ReadMail]);
    let one = [supplier("mes-mails", &[ReadMail])];
    let two = [supplier("mes-mails", &[ReadMail]), supplier("ma-boite", &[ReadMail])];
    assert!(TalentDependencyRule::uncovered_by_suppliers(&sorter, &one).is_empty());
    assert!(TalentDependencyRule::uncovered_by_suppliers(&sorter, &two).is_empty());
    assert_eq!(
        TalentDependencyRule::uncovered_by_suppliers(&sorter, &[]),
        BTreeSet::from([ReadMail])
    );
    let both = text_talent("trier-et-chercher", &[ReadMail, WebSearch]);
    assert_eq!(
        TalentDependencyRule::uncovered_by_suppliers(&both, &one),
        BTreeSet::from([WebSearch]),
        "only the capability nobody supplies"
    );
    let textual_only = [text_talent("autre", &[ReadMail])];
    assert_eq!(
        TalentDependencyRule::uncovered_by_suppliers(&sorter, &textual_only),
        BTreeSet::from([ReadMail]),
        "a textual Talent supplies nothing"
    );
}

#[test]
fn a_manifest_with_requires_and_a_talent_with_code_have_no_capability_to_supply() {
    use TalentCapability::*;
    let old_form = manifest("trier", "1.0.0", &[ReadMail], &["mes-mails"]);
    assert!(TalentDependencyRule::uncovered_by_suppliers(&old_form, &[]).is_empty());
    assert!(TalentDependencyRule::uncovered_by_suppliers(&supplier("mes-mails", &[ReadMail]), &[]).is_empty());
    let catalog = catalog(vec![supplier("mes-mails", &[ReadMail]), old_form.clone()]);
    assert_eq!(
        TalentDependencyRule::uncovered_capabilities(&old_form, &catalog),
        BTreeSet::new(),
        "the direct dependencies still cover it by identifier"
    );
}

#[test]
fn the_proposed_suppliers_are_the_learned_then_the_built_in_then_the_catalogue_without_duplicates() {
    use TalentCapability::*;
    let pool = CapabilitySupplierPool {
        learned: vec![supplier("zeta-appris", &[ReadMail]), supplier("naviguer", &[BrowseWeb])],
        built_ins: vec![
            supplier("mes-mails", &[ReadMail]),
            supplier("alpha-integre", &[ReadMail]),
            supplier("zeta-appris", &[ReadMail]),
        ],
        catalog: vec![
            supplier("ma-boite", &[ReadMail]),
            supplier("mes-mails", &[ReadMail]),
            text_talent("pas-de-code", &[ReadMail]),
        ],
    };
    assert_eq!(
        TalentDependencyRule::proposed_suppliers(ReadMail, &pool),
        ["zeta-appris", "alpha-integre", "mes-mails", "ma-boite"]
    );
}

#[test]
fn a_capability_without_any_supplier_on_the_platform_proposes_nothing() {
    use TalentCapability::*;
    let pool = CapabilitySupplierPool {
        learned: vec![supplier("naviguer", &[BrowseWeb])],
        built_ins: vec![],
        catalog: vec![text_talent("pas-de-code", &[ReadMail])],
    };
    assert!(TalentDependencyRule::proposed_suppliers(ReadMail, &pool).is_empty());
}

#[test]
fn forgetting_one_of_two_suppliers_leaves_the_dependent_but_forgetting_the_last_one_does_not() {
    use TalentCapability::*;
    let sorter = text_talent("trier", &[ReadMail]);
    let both = vec![
        supplier("mes-mails", &[ReadMail]),
        supplier("ma-boite", &[ReadMail]),
        sorter.clone(),
    ];
    assert!(TalentDependencyRule::dependents("mes-mails", &both).is_empty());
    assert!(TalentDependencyRule::dependents("ma-boite", &both).is_empty());
    let last = vec![supplier("ma-boite", &[ReadMail]), sorter];
    assert_eq!(TalentDependencyRule::dependents("ma-boite", &last), ["trier"]);
}

#[test]
fn a_dependent_by_capability_drags_the_talents_that_require_it_by_identifier() {
    use TalentCapability::*;
    let learned = vec![
        supplier("mes-mails", &[ReadMail]),
        text_talent("trier", &[ReadMail]),
        manifest("rapport", "1.0.0", &[], &["trier"]),
    ];
    assert_eq!(
        TalentDependencyRule::dependents("mes-mails", &learned),
        ["trier", "rapport"]
    );
}
