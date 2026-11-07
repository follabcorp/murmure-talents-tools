use super::test_fixtures::{file, skill};
use super::{SkillClaim, SkillClaimRule};

fn claim(name: &str, fingerprint: &str) -> SkillClaim {
    SkillClaim { name: name.to_owned(), fingerprint: fingerprint.to_owned() }
}

#[test]
fn a_root_skill_is_named_after_its_folder() {
    let claims = SkillClaimRule::claims("travel-coach", &[file("SKILL.md", &skill())]).expect("claims");
    assert_eq!(claims.len(), 1);
    assert_eq!(claims[0].name, "travel-coach");
}

#[test]
fn refuses_files_outside_a_skill_folder() {
    assert!(SkillClaimRule::claims("travel-coach", &[file("skills/SKILL.md", &skill())]).is_err());
}

#[test]
fn the_same_name_conflicts_only_with_a_different_fingerprint() {
    assert!(SkillClaimRule::validate(&[claim("shared", "aa"), claim("shared", "aa"), claim("other", "bb")]).is_ok());
    assert!(SkillClaimRule::validate(&[claim("shared", "aa"), claim("shared", "bb")]).is_err());
}
