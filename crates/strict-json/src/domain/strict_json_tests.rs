use super::parse_strict_json as parse;

fn nested(levels: usize) -> Vec<u8> {
    format!("{}{}", "[".repeat(levels), "]".repeat(levels)).into_bytes()
}

#[test]
fn thirty_two_levels_of_nesting_are_read_and_the_thirty_third_is_refused() {
    assert!(parse(&nested(32)).is_ok());
    assert!(parse(&nested(33)).is_err());
}

#[test]
fn an_escaped_spelling_of_a_key_is_that_key() {
    assert!(parse(br#"{"a":1,"a":2}"#).is_err());
    assert!(parse(br#"{"a":{"b":1},"c":{"b":2}}"#).is_ok());
}
