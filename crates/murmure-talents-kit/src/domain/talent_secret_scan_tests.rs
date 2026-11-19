//! Contract: a free text of an Agent Talent carries no secret. One refused representative per main form, and legitimate
//! texts that merely look close.

use super::TalentSecretScan;

#[test]
fn the_main_secret_forms_are_found() {
    let samples = [
        format!("Use key sk-ant-api03-{} please", "a1B2".repeat(8)),
        format!("token ghp_{}", "x9Y8".repeat(9)),
        format!("export AWS=AKIA{}", "ABCDEFGHIJKLMNOP"),
        "-----BEGIN RSA PRIVATE KEY-----\nMIIE".to_owned(),
        format!(
            "jwt eyJ{}.eyJ{}.{}",
            "hbGciOiJIUzI1".repeat(2),
            "zdWIiOiIxMjM0".repeat(2),
            "SflKxwRJSMeKKF2QT4"
        ),
    ];
    for sample in &samples {
        assert!(TalentSecretScan::find(sample).is_some(), "{sample}");
    }
}

#[test]
fn legitimate_texts_are_accepted() {
    for text in [
        "Ask for the sk key name, never the key itself.",
        "See https://example.org/skills/sk-learn for details.",
        "Warm, concrete.\nAlways reply in the person's language.",
        "A private key is something to never share.",
    ] {
        assert_eq!(TalentSecretScan::find(text), None, "{text}");
    }
}
