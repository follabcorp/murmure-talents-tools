use super::PublicationSequenceRule;

#[test]
fn auto_is_the_clock_in_seconds_and_grows_with_it() {
    let first = PublicationSequenceRule::resolve("auto", 1_793_577_600).unwrap();
    let later = PublicationSequenceRule::resolve("auto", 1_793_577_601).unwrap();
    assert_eq!(first, 1_793_577_600);
    assert!(later > first);
}

#[test]
fn an_explicit_integer_is_kept_whatever_the_clock() {
    assert_eq!(PublicationSequenceRule::resolve("42", 1_793_577_600).unwrap(), 42);
    assert_eq!(PublicationSequenceRule::resolve("0", 1_793_577_600).unwrap(), 0);
}

#[test]
fn refuses_what_is_neither_auto_nor_an_unsigned_integer() {
    for text in ["", "AUTO", "-1", "1.5", "12a", " 7", "99999999999999999999999"] {
        assert!(PublicationSequenceRule::resolve(text, 1_793_577_600).is_err(), "{text:?}");
    }
}
