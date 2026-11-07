//! Contract: « Talents "Par Murmure" ».

use serde_json::json;

use super::fixtures::manifest_json;
use super::{TalentBuiltInResource, TalentKind, TalentManifestError, BUILT_IN_MANIFESTS};

fn envelope(connector: &str) -> serde_json::Value {
    let mut manifest = manifest_json();
    manifest["kind"] = json!("withCode");
    manifest["requires"] = json!([]);
    json!({"connector": connector, "manifest": manifest})
}

#[test]
fn an_envelope_holds_a_connector_and_a_manifest_with_code_read_as_embedded() {
    let resource = TalentBuiltInResource::parse(envelope("chrome").to_string().as_bytes())
        .expect("a built-in Talent");
    assert_eq!(
        (resource.connector.as_str(), resource.manifest.kind),
        ("chrome", TalentKind::WithCode)
    );
    let text = String::from_utf8(resource.manifest_json).expect("UTF-8");
    assert!(
        text.starts_with(r#"{"author":"#)
            && text.contains("https://example.org")
            && !text.contains(": "),
        "sorted, compact, slashes unescaped: {text}"
    );
}

#[test]
fn an_envelope_with_another_set_of_keys_or_a_bad_connector_or_too_big_is_refused() {
    let mut extra = envelope("chrome");
    extra["extra"] = json!(1);
    let mut missing = envelope("chrome");
    missing
        .as_object_mut()
        .expect("an object")
        .remove("connector");
    for refused in [extra, missing, envelope("Chrome_Mail")] {
        assert_eq!(
            TalentBuiltInResource::parse(refused.to_string().as_bytes()),
            Err(TalentManifestError::InvalidField("envelope".into())),
            "{refused}"
        );
    }
    assert!(
        TalentBuiltInResource::parse(br#"{"connector":"a","connector":"a"}"#).is_err(),
        "duplicate key"
    );
    assert_eq!(
        TalentBuiltInResource::parse(&vec![b' '; TalentBuiltInResource::MAXIMUM_BYTES + 1]),
        Err(TalentManifestError::TooLarge)
    );
}

#[test]
fn a_built_in_talent_is_available_when_its_connector_is_defined() {
    let definitions = vec!["chrome".to_owned()];
    assert!(TalentBuiltInResource::is_available("chrome", &definitions));
    assert!(!TalentBuiltInResource::is_available("mail", &definitions));
}

fn my_mail() -> TalentBuiltInResource {
    let text = BUILT_IN_MANIFESTS
        .iter()
        .find(|text| text.contains(r#""id": "mes-mails""#))
        .expect("the built-in Talent mes-mails");
    TalentBuiltInResource::parse(text.as_bytes()).expect("a built-in Talent")
}

#[test]
fn my_mail_is_at_1_1_0_and_organizes_mail_after_reading_and_sending_it() {
    let resource = my_mail();
    assert_eq!(resource.connector, "mail");
    assert_eq!(resource.manifest.version.to_string(), "1.1.0");
    let wires: Vec<&str> = resource.manifest.capabilities.iter().map(|capability| capability.wire()).collect();
    assert_eq!(wires, ["read-mail", "send-mail", "organize-mail"]);
}
