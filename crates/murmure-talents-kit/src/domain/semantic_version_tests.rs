//! Contract: « Manifeste », key `version`; « Règles pures » 2 (numeric comparison).

use super::SemanticVersion;

#[test]
fn a_version_is_three_numbers_up_to_999999_and_compares_numerically() {
    assert_eq!(
        SemanticVersion::parse("1.2.3"),
        Some(SemanticVersion::new(1, 2, 3))
    );
    assert_eq!(
        SemanticVersion::parse("0.0.0"),
        Some(SemanticVersion::new(0, 0, 0))
    );
    assert_eq!(
        SemanticVersion::parse("999999.999999.999999"),
        Some(SemanticVersion::new(999_999, 999_999, 999_999))
    );
    assert!(
        SemanticVersion::parse("1.10.0") > SemanticVersion::parse("1.9.0"),
        "not a text comparison"
    );
    assert_eq!(SemanticVersion::new(1, 2, 3).to_string(), "1.2.3");
}

#[test]
fn a_version_with_a_suffix_a_leading_zero_a_missing_part_or_a_number_above_the_bound_is_refused() {
    for refused in [
        "1.0",
        "1.0.0.0",
        "1.0.0-beta",
        "1.0.0+7",
        "01.0.0",
        "1.00.0",
        "1.0.-1",
        "1.0.",
        "",
        "a.b.c",
        "1000000.0.0",
        "1.0.0 ",
        "+1.0.0",
    ] {
        assert_eq!(SemanticVersion::parse(refused), None, "{refused:?}");
    }
}
