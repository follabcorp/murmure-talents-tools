//! Contract: « Auteurs, versions et CODEOWNERS ». Pure generation of the file, never a disk.

use super::{CodeownersEntry, CodeownersRule};

fn entry(id: &str, github: &[&str]) -> CodeownersEntry {
    CodeownersEntry { talent_id: id.to_owned(), github: github.iter().map(|handle| (*handle).to_owned()).collect() }
}

const EXPECTED: &str = "* @follabcorp/murmure\n\
/talents/alpha/ @follabcorp/murmure @alice @bob\n\
/talents/beta/ @follabcorp/murmure @carol\n\
/.github/ @follabcorp/murmure\n\
/featured.json @follabcorp/murmure\n\
/revoked.json @follabcorp/murmure\n";

#[test]
fn the_file_is_the_team_then_each_talent_then_the_team_only_paths() {
    let entries = [entry("alpha", &["alice", "bob"]), entry("beta", &["carol"])];
    assert_eq!(CodeownersRule::expected(&entries), EXPECTED);
}

#[test]
fn the_file_is_the_same_whatever_the_order_of_talents_and_handles_and_ignores_duplicates() {
    let shuffled = [entry("beta", &["carol", "carol"]), entry("alpha", &["bob", "alice"])];
    assert_eq!(CodeownersRule::expected(&shuffled), EXPECTED);
}

#[test]
fn a_catalogue_without_talent_still_protects_the_shared_paths() {
    assert_eq!(
        CodeownersRule::expected(&[]),
        "* @follabcorp/murmure\n/.github/ @follabcorp/murmure\n/featured.json @follabcorp/murmure\n/revoked.json @follabcorp/murmure\n"
    );
}

#[test]
fn a_present_file_is_current_only_when_it_is_the_expected_one() {
    assert!(CodeownersRule::is_current(EXPECTED, EXPECTED));
    let other_owner = EXPECTED.replace("@carol", "@mallory");
    assert!(!CodeownersRule::is_current(EXPECTED, &other_owner));
    assert!(!CodeownersRule::is_current(EXPECTED, ""));
}
