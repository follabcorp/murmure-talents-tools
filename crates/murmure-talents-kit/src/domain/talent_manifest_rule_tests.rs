//! Contract: « Manifeste », « Nature ». One test per class of refusal; a bound is tested at the bound and above it.

use serde_json::{json, Value};

use super::fixtures::{bytes, manifest_json};
use super::{
    SemanticVersion, TalentCapability, TalentKind, TalentManifestError, TalentManifestRule,
    TalentNeed, TalentPlatform, TalentSource,
};

fn parse(value: &Value) -> Result<super::TalentManifest, TalentManifestError> {
    TalentManifestRule::parse(&bytes(value), TalentSource::Catalog)
}

fn with(change: impl FnOnce(&mut Value)) -> Value {
    let mut value = manifest_json();
    change(&mut value);
    value
}

fn invalid(field: &str) -> Result<super::TalentManifest, TalentManifestError> {
    Err(TalentManifestError::InvalidField(field.to_owned()))
}

#[test]
fn the_manifest_of_the_contract_is_read_field_by_field() {
    let manifest = parse(&manifest_json()).expect("a valid manifest");
    assert_eq!(
        (manifest.id.as_str(), manifest.kind, manifest.version),
        (
            "trier-mes-mails",
            TalentKind::TextOnly,
            SemanticVersion::new(1, 2, 0)
        )
    );
    assert_eq!(manifest.localized["en"].name, "Sort my email");
    assert_eq!(manifest.needs, [TalentNeed::Mail, TalentNeed::Organizing]);
    assert_eq!(manifest.capabilities, [TalentCapability::ReadMail]);
    assert_eq!(
        (
            manifest.requires.as_slice(),
            manifest.minimum_murmure_version
        ),
        (
            ["mes-mails".to_owned()].as_slice(),
            SemanticVersion::new(1, 0, 0)
        )
    );
    assert_eq!(manifest.platforms, Some(vec![TalentPlatform::Macos]));
    assert_eq!(
        (
            manifest.author.name.as_str(),
            manifest.author.url.as_deref()
        ),
        ("Camille Dupont", Some("https://example.org"))
    );
    assert_eq!(manifest.license, "MIT");
    assert_eq!(
        manifest.origin.map(|origin| origin.author),
        Some("jdoe".to_owned())
    );
}

#[test]
fn the_optional_keys_default_to_nothing_and_an_empty_list_of_capabilities_is_allowed() {
    let manifest = parse(&with(|value| {
        let object = value.as_object_mut().expect("an object");
        for key in ["requires", "platforms", "origin"] {
            object.remove(key);
        }
        object["capabilities"] = json!([]);
    }))
    .expect("a valid manifest");
    assert_eq!(
        (
            manifest.requires.is_empty(),
            manifest.platforms,
            manifest.origin,
            manifest.capabilities.is_empty()
        ),
        (true, None, None, true)
    );
}

#[test]
fn a_manifest_of_16_kib_is_read_and_one_byte_more_is_too_large_before_any_analysis() {
    let mut text = manifest_json().to_string().into_bytes();
    text.resize(16_384, b' ');
    assert!(TalentManifestRule::parse(&text, TalentSource::Catalog).is_ok());
    text.push(b' ');
    assert_eq!(
        TalentManifestRule::parse(&text, TalentSource::Catalog),
        Err(TalentManifestError::TooLarge)
    );
    assert_eq!(
        TalentManifestRule::parse(&vec![b'x'; 16_385], TalentSource::Catalog),
        Err(TalentManifestError::TooLarge),
        "not even JSON, still too large first"
    );
}

#[test]
fn a_text_that_is_not_strict_json_is_malformed_even_when_a_duplicate_key_is_escaped() {
    for text in [
        "pas du json",
        "",
        r#"{"id":"a","id":"b"}"#,
        r#"{"id":"a","id":"b"}"#,
    ] {
        assert_eq!(
            TalentManifestRule::parse(text.as_bytes(), TalentSource::Catalog),
            Err(TalentManifestError::MalformedJson),
            "{text:?}"
        );
    }
    assert_eq!(
        TalentManifestRule::parse(b"[]", TalentSource::Catalog),
        invalid("manifest"),
        "valid JSON, but not a manifest"
    );
}

#[test]
fn a_key_the_schema_does_not_know_is_refused_at_every_level() {
    for path in ["", "localized.fr", "author", "origin"] {
        let value = with(|value| {
            let target = path
                .split('.')
                .filter(|part| !part.is_empty())
                .fold(&mut *value, |node, key| &mut node[key]);
            target["unknown"] = json!(1);
        });
        assert_eq!(
            parse(&value),
            Err(TalentManifestError::UnknownKey),
            "{path:?}"
        );
    }
}

#[test]
fn a_capability_or_a_need_outside_the_vocabulary_makes_the_manifest_invalid_instead_of_being_ignored(
) {
    assert_eq!(
        parse(&with(
            |value| value["capabilities"] = json!(["read-mail", "teleport"])
        )),
        invalid("capabilities")
    );
    assert_eq!(
        parse(&with(|value| value["needs"] = json!(["mail", "shopping"]))),
        invalid("needs")
    );
}

#[test]
fn the_nature_with_code_is_admitted_for_the_embedded_manifests_only() {
    let with_code = with(|value| value["kind"] = json!("withCode"));
    assert_eq!(
        TalentManifestRule::parse(&bytes(&with_code), TalentSource::Catalog),
        Err(TalentManifestError::KindNotAllowed)
    );
    assert_eq!(
        TalentManifestRule::parse(&bytes(&with_code), TalentSource::LocalFolder),
        Err(TalentManifestError::KindNotAllowed)
    );
    assert_eq!(
        TalentManifestRule::parse(&bytes(&with_code), TalentSource::Embedded)
            .map(|manifest| manifest.kind),
        Ok(TalentKind::WithCode)
    );
    assert_eq!(
        parse(&with(|value| value["kind"] = json!("script"))),
        invalid("kind")
    );
}

#[test]
fn the_identifier_and_the_versions_are_strict() {
    assert_eq!(
        parse(&with(|value| value["id"] = json!("a".repeat(64)))).map(|manifest| manifest.id.len()),
        Ok(64)
    );
    for id in [
        json!("a".repeat(65)),
        json!("Mes_Mails"),
        json!(""),
        json!(7),
    ] {
        assert_eq!(
            parse(&with(|value| value["id"] = id.clone())),
            invalid("id"),
            "{id}"
        );
    }
    assert_eq!(
        parse(&with(|value| value["version"] = json!("1.0"))),
        invalid("version")
    );
    assert_eq!(
        parse(&with(
            |value| value["minimumMurmureVersion"] = json!("1.0.0-beta")
        )),
        invalid("minimumMurmureVersion")
    );
}

#[test]
fn two_to_sixteen_languages_with_french_and_english_are_required_and_their_tags_are_checked() {
    let languages = |count: usize| {
        with(|value| {
            let localized = value["localized"].as_object_mut().expect("an object");
            for index in 0..count.saturating_sub(2) {
                let tag: String = [
                    char::from(b'a' + (index / 26) as u8),
                    char::from(b'a' + (index % 26) as u8),
                    'x',
                ]
                .iter()
                .collect();
                localized.insert(tag, json!({"name": "Nom", "summary": "Résumé."}));
            }
        })
    };
    assert!(parse(&languages(16)).is_ok());
    assert_eq!(parse(&languages(17)), invalid("localized"));
    assert_eq!(
        parse(&with(|value| {
            value["localized"]
                .as_object_mut()
                .expect("an object")
                .remove("en");
        })),
        invalid("localized")
    );
    assert_eq!(
        parse(&with(|value| {
            value["localized"]
                .as_object_mut()
                .expect("an object")
                .remove("fr");
        })),
        invalid("localized")
    );
    let tagged = |tag: &str| {
        with(|value| value["localized"][tag] = json!({"name": "Nom", "summary": "Résumé."}))
    };
    assert!(parse(&tagged("zh-Hans")).is_ok(), "a script is optional");
    for tag in ["FR", "f", "zh-hans", "zh-Han", "fr_CA", "fr-CA"] {
        assert_eq!(parse(&tagged(tag)), invalid("localized"), "{tag}");
    }
}

#[test]
fn a_name_and_a_summary_are_trimmed_counted_in_graphemes_and_free_of_line_breaks_and_controls() {
    let named = |name: &str| with(|value| value["localized"]["fr"]["name"] = json!(name));
    assert_eq!(
        parse(&named("  Trier  ")).map(|manifest| manifest.localized["fr"].name.clone()),
        Ok("Trier".to_owned())
    );
    assert!(
        parse(&named(&"e\u{301}".repeat(40))).is_ok(),
        "40 graphemes, 80 scalars"
    );
    assert_eq!(
        parse(&named(&"e\u{301}".repeat(41))),
        invalid("localized.name")
    );
    for name in ["", "   ", "deux\nlignes", "tab\tulation", "nul\u{0}"] {
        assert_eq!(parse(&named(name)), invalid("localized.name"), "{name:?}");
    }
    let summarized =
        |summary: &str| with(|value| value["localized"]["en"]["summary"] = json!(summary));
    assert!(parse(&summarized(&"x".repeat(140))).is_ok());
    assert_eq!(
        parse(&summarized(&"x".repeat(141))),
        invalid("localized.summary")
    );
}

#[test]
fn needs_capabilities_requirements_and_platforms_are_bounded_distinct_lists() {
    assert_eq!(
        parse(&with(|value| value["needs"] = json!([]))),
        invalid("needs")
    );
    assert_eq!(
        parse(&with(
            |value| value["needs"] = json!(["mail", "home", "travel"])
        )),
        invalid("needs")
    );
    assert_eq!(
        parse(&with(|value| value["needs"] = json!(["mail", "mail"]))),
        invalid("needs")
    );
    let all: Vec<&str> = TalentCapability::ALL
        .iter()
        .map(|capability| capability.wire())
        .collect();
    assert_eq!(
        parse(&with(|value| value["capabilities"] = json!(all)))
            .map(|manifest| manifest.capabilities.len()),
        Ok(TalentCapability::ALL.len())
    );
    assert_eq!(
        parse(&with(
            |value| value["capabilities"] = json!(["read-mail", "read-mail"])
        )),
        invalid("capabilities")
    );
    let requires = |count: usize| {
        with(|value| {
            value["requires"] = json!((0..count)
                .map(|index| format!("dep-{index}"))
                .collect::<Vec<_>>())
        })
    };
    assert!(parse(&requires(8)).is_ok());
    assert_eq!(parse(&requires(9)), invalid("requires"));
    assert_eq!(
        parse(&with(|value| value["requires"] = json!(["trier-mes-mails"]))),
        invalid("requires"),
        "never its own id"
    );
    assert_eq!(
        parse(&with(|value| value["requires"] = json!(["Mauvais_Id"]))),
        invalid("requires")
    );
    assert_eq!(
        parse(&with(|value| value["platforms"] = json!(["macos", "linux"])))
            .map(|manifest| manifest.platforms),
        Ok(Some(vec![TalentPlatform::Macos, TalentPlatform::Linux]))
    );
    for platforms in [json!([]), json!(["windows"]), json!(["macos", "macos"])] {
        assert_eq!(
            parse(&with(|value| value["platforms"] = platforms.clone())),
            invalid("platforms"),
            "{platforms}"
        );
    }
}

#[test]
fn an_author_a_license_and_an_origin_follow_their_bounds() {
    assert_eq!(
        parse(&with(
            |value| value["author"]["name"] = json!("é".repeat(64))
        ))
        .map(|manifest| manifest.author.name.chars().count()),
        Ok(64)
    );
    assert_eq!(
        parse(&with(
            |value| value["author"]["name"] = json!("é".repeat(65))
        )),
        invalid("author.name")
    );
    let url = |text: String| with(|value| value["author"]["url"] = json!(text));
    assert!(
        parse(&url(format!(
            "https://example.org/{}",
            "a".repeat(256 - "https://example.org/".len())
        )))
        .is_ok(),
        "256 bytes"
    );
    assert_eq!(
        parse(&url(format!(
            "https://example.org/{}",
            "a".repeat(257 - "https://example.org/".len())
        ))),
        invalid("author.url")
    );
    for refused in [
        "http://example.org",
        "ftp://example.org",
        "example.org",
        "https://",
    ] {
        assert_eq!(
            parse(&url(refused.to_owned())),
            invalid("author.url"),
            "{refused}"
        );
    }
    assert!(parse(&with(|value| value["license"] = json!("Apache-2.0+"))).is_ok());
    assert!(parse(&with(|value| value["license"] = json!("a".repeat(64)))).is_ok());
    for license in [
        "a".repeat(65),
        "GPL v3".to_owned(),
        String::new(),
        "MIT/X".to_owned(),
    ] {
        assert_eq!(
            parse(&with(|value| value["license"] = json!(license))),
            invalid("license"),
            "{license:?}"
        );
    }
    assert_eq!(
        parse(&with(|value| value["origin"]["kind"] = json!("chatgpt"))),
        invalid("origin.kind")
    );
    assert_eq!(
        parse(&with(|value| {
            value["origin"]
                .as_object_mut()
                .expect("an object")
                .remove("author");
        })),
        invalid("origin.author")
    );
}

fn parse_from(value: &Value, source: TalentSource) -> Result<super::TalentManifest, TalentManifestError> {
    TalentManifestRule::parse(&bytes(value), source)
}

fn with_github(github: Value) -> Value {
    with(|value| value["author"]["github"] = github)
}

#[test]
fn a_catalog_manifest_names_one_to_four_github_accounts() {
    for handles in [vec!["camille"], vec!["a", "b", "c", "d"]] {
        let manifest = parse(&with_github(json!(handles))).expect("1 to 4 accounts");
        assert_eq!(manifest.author.github, handles);
    }
    for handles in [json!([]), json!(["a", "b", "c", "d", "e"])] {
        assert_eq!(parse(&with_github(handles)), invalid("author.github"));
    }
}

#[test]
fn a_catalog_manifest_without_github_is_refused_but_other_sources_accept_it() {
    let legacy = with(|value| {
        value["author"].as_object_mut().expect("an author").remove("github");
    });
    assert_eq!(parse_from(&legacy, TalentSource::Catalog), invalid("author.github"));
    for source in [TalentSource::LocalFolder, TalentSource::Embedded] {
        let manifest = parse_from(&legacy, source).expect("the legacy author stays readable");
        assert!(manifest.author.github.is_empty());
    }
}

#[test]
fn a_non_catalog_manifest_may_carry_github_accounts() {
    let manifest = parse_from(&with_github(json!(["camille"])), TalentSource::LocalFolder).expect("accounts");
    assert_eq!(manifest.author.github, ["camille"]);
}

#[test]
fn a_github_account_follows_the_github_syntax() {
    let long_enough = "a".repeat(39);
    for valid in ["camille", "Camille-D", "a", "a1-b2", long_enough.as_str()] {
        assert!(parse(&with_github(json!([valid]))).is_ok(), "{valid:?}");
    }
    let too_long = "a".repeat(40);
    for refused in ["-camille", "camille-", "cam--ille", "cam_ille", "cam ille", "@camille", "", "é", too_long.as_str()] {
        assert_eq!(parse(&with_github(json!([refused]))), invalid("author.github"), "{refused:?}");
    }
    assert_eq!(parse(&with_github(json!(["camille", "camille"]))), invalid("author.github"));
    assert_eq!(parse(&with_github(json!("camille"))), invalid("author.github"));
}

#[test]
fn a_public_repository_origin_needs_its_url_while_the_ecosystem_ones_keep_theirs_optional() {
    let public = with(|value| {
        value["origin"] = json!({"kind": "public", "author": "JayRHa", "url": "https://github.com/JayRHa/agent-skills"});
    });
    let origin = parse(&public).expect("a public origin").origin.expect("an origin");
    assert_eq!(origin.kind, super::TalentOriginKind::Public);
    assert_eq!(origin.url.as_deref(), Some("https://github.com/JayRHa/agent-skills"));

    let without_url = with(|value| value["origin"] = json!({"kind": "public", "author": "JayRHa"}));
    assert_eq!(parse(&without_url), invalid("origin.url"));

    for kind in ["openclaw", "hermes"] {
        let value = with(|value| value["origin"] = json!({"kind": kind, "author": "jdoe"}));
        assert!(parse(&value).is_ok(), "{kind}");
    }
}
