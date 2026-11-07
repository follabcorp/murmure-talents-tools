//! Contract: « Catalogue » (`index.json`). Bounds are tested at the bound and above it.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::fixtures::manifest_json;
use super::{TalentIndex, TalentIndexError, TalentRevocationReason};

const GENERATED: &str = "2026-11-02T10:00:00Z";
const EXPIRES: &str = "2026-11-16T10:00:00Z";

pub fn file(path: &str, content: &str) -> Value {
    json!({"path": path, "size": content.len(), "sha256": format!("{:x}", Sha256::digest(content.as_bytes()))})
}

fn entry(id: &str) -> Value {
    let mut manifest = manifest_json();
    manifest["id"] = json!(id);
    manifest["requires"] = json!([]);
    json!({"manifest": manifest, "files": [file("SKILL.md", "---\nname: x\n---\n")]})
}

fn index() -> Value {
    json!({
        "format": 1, "sequence": 42, "generatedAt": GENERATED, "expiresAt": EXPIRES, "baseURL": "https://talents.example/v1/",
        "featured": ["trier-mes-mails"], "talents": [entry("trier-mes-mails")], "revoked": [{"id": "talent-retire", "reason": "malicious"}]
    })
}

fn parse(value: &Value) -> Result<TalentIndex, TalentIndexError> {
    TalentIndex::parse(value.to_string().as_bytes())
}

fn with(change: impl FnOnce(&mut Value)) -> Value {
    let mut value = index();
    change(&mut value);
    value
}

#[test]
fn the_index_of_the_contract_is_read_with_its_times_files_and_revocations() {
    let index = parse(&index()).expect("a valid index");
    assert_eq!(
        (index.format, index.sequence, index.featured.as_slice()),
        (1, 42, ["trier-mes-mails".to_owned()].as_slice())
    );
    assert_eq!(
        (index.generated_at, index.expires_at),
        (1_793_613_600, 1_794_823_200),
        "Unix seconds of the two ISO 8601 dates"
    );
    assert_eq!(index.base_url, "https://talents.example/v1/");
    assert_eq!(
        (
            index.talents.len(),
            index.talents[0].manifest.id.as_str(),
            index.talents[0].files[0].path.as_str()
        ),
        (1, "trier-mes-mails", "SKILL.md")
    );
    assert_eq!(
        (index.revoked[0].id.as_str(), index.revoked[0].reason),
        ("talent-retire", TalentRevocationReason::Malicious)
    );
    let kept: Value = serde_json::from_slice(&index.talents[0].manifest_json).expect("JSON");
    assert_eq!(kept["id"], "trier-mes-mails");
    assert!(
        !String::from_utf8_lossy(&index.talents[0].manifest_json).contains(": "),
        "compact, with sorted keys"
    );
}

#[test]
fn an_index_of_4_mib_is_read_and_one_byte_more_is_too_large_before_any_analysis() {
    let mut text = index().to_string().into_bytes();
    text.resize(TalentIndex::MAXIMUM_BYTES, b' ');
    assert!(TalentIndex::parse(&text).is_ok());
    text.push(b' ');
    assert_eq!(TalentIndex::parse(&text), Err(TalentIndexError::TooLarge));
}

#[test]
fn a_duplicate_key_even_escaped_and_an_unknown_key_at_any_level_are_refused() {
    assert_eq!(
        TalentIndex::parse(br#"{"format":1,"format":1}"#),
        Err(TalentIndexError::MalformedJson)
    );
    assert_eq!(
        TalentIndex::parse(br#"{"format":1,"format":1}"#),
        Err(TalentIndexError::MalformedJson)
    );
    assert_eq!(
        TalentIndex::parse(b"nope"),
        Err(TalentIndexError::MalformedJson)
    );
    for place in ["", "talents.0", "talents.0.files.0", "revoked.0"] {
        let value = with(|value| {
            let target =
                place
                    .split('.')
                    .filter(|part| !part.is_empty())
                    .fold(&mut *value, |node, key| match key.parse::<usize>() {
                        Ok(position) => &mut node[position],
                        Err(_) => &mut node[key],
                    });
            target["extra"] = json!(1);
        });
        assert_eq!(
            parse(&value),
            Err(TalentIndexError::UnknownKey),
            "{place:?}"
        );
    }
}

#[test]
fn a_missing_key_a_format_other_than_1_and_a_sequence_that_is_not_a_natural_integer_are_invalid() {
    assert_eq!(
        parse(&with(|value| {
            value.as_object_mut().expect("an object").remove("revoked");
        })),
        Err(TalentIndexError::InvalidFormat)
    );
    assert_eq!(
        parse(&with(|value| value["format"] = json!(2))),
        Err(TalentIndexError::InvalidFormat)
    );
    assert_eq!(
        parse(&with(|value| value["sequence"] = json!(0))).map(|index| index.sequence),
        Ok(0)
    );
    for sequence in [json!(-1), json!(1.5), json!(true), json!("1")] {
        assert_eq!(
            parse(&with(|value| value["sequence"] = sequence.clone())),
            Err(TalentIndexError::InvalidFormat),
            "{sequence}"
        );
    }
}

#[test]
fn the_validity_is_positive_and_at_most_30_days() {
    let valid_until = |expires: &str| with(|value| value["expiresAt"] = json!(expires));
    assert!(
        parse(&valid_until("2026-12-02T10:00:00Z")).is_ok(),
        "30 days exactly"
    );
    assert_eq!(
        parse(&valid_until("2026-12-02T10:00:01Z")),
        Err(TalentIndexError::InvalidFormat)
    );
    assert_eq!(
        parse(&valid_until(GENERATED)),
        Err(TalentIndexError::InvalidFormat),
        "not strictly positive"
    );
    assert_eq!(
        parse(&valid_until("2026-11-01T10:00:00Z")),
        Err(TalentIndexError::InvalidFormat)
    );
    assert_eq!(
        parse(&valid_until("demain")),
        Err(TalentIndexError::InvalidFormat)
    );
}

#[test]
fn the_index_holds_at_most_2000_entries_with_distinct_identifiers() {
    let entries = |count: usize| {
        with(|value| {
            value["talents"] = json!((0..count)
                .map(|index| entry(&format!("talent-{index}")))
                .collect::<Vec<_>>());
            value["featured"] = json!([]);
        })
    };
    assert_eq!(
        parse(&entries(2000)).map(|index| index.talents.len()),
        Ok(2000)
    );
    assert_eq!(parse(&entries(2001)), Err(TalentIndexError::InvalidFormat));
    assert_eq!(
        parse(&with(
            |value| value["talents"] = json!([entry("a"), entry("a")])
        )),
        Err(TalentIndexError::InvalidFormat)
    );
}

#[test]
fn featured_holds_up_to_three_distinct_identifiers_that_are_in_the_raw_entries() {
    let featured = |ids: &[&str]| {
        with(|value| {
            value["talents"] = json!(["a", "b", "c", "d"]
                .iter()
                .map(|id| entry(id))
                .collect::<Vec<_>>());
            value["featured"] = json!(ids);
        })
    };
    assert_eq!(
        parse(&featured(&[])).map(|index| index.featured.len()),
        Ok(0)
    );
    assert_eq!(
        parse(&featured(&["a", "b", "c"])).map(|index| index.featured.len()),
        Ok(3)
    );
    assert_eq!(
        parse(&featured(&["a", "b", "c", "d"])),
        Err(TalentIndexError::InvalidFormat)
    );
    assert_eq!(
        parse(&featured(&["a", "a"])),
        Err(TalentIndexError::InvalidFormat)
    );
    assert_eq!(
        parse(&featured(&["absent"])),
        Err(TalentIndexError::InvalidFormat)
    );
}

#[test]
fn a_manifest_this_version_does_not_understand_hides_its_entry_and_not_the_index() {
    let mut newer = entry("nouveau");
    newer["manifest"]["capabilities"] = json!(["teleport"]);
    let mut with_code = entry("avec-code");
    with_code["manifest"]["kind"] = json!("withCode");
    let value = with(|value| {
        value["talents"] = json!([entry("connu"), newer, with_code]);
        value["featured"] = json!(["nouveau"]);
    });
    let index = parse(&value).expect("the signed envelope stays useful");
    assert_eq!(
        index
            .talents
            .iter()
            .map(|entry| entry.manifest.id.as_str())
            .collect::<Vec<_>>(),
        ["connu"]
    );
    assert_eq!(
        index.featured,
        ["nouveau"],
        "featured is checked against the raw entries"
    );
}

#[test]
fn the_files_of_an_entry_and_the_revocations_are_checked_with_the_index() {
    let mut no_skill = entry("sans-skill");
    no_skill["files"] = json!([file("README.md", "x")]);
    assert_eq!(
        parse(&with(|value| value["talents"] = json!([no_skill]))),
        Err(TalentIndexError::InvalidFiles)
    );
    assert_eq!(
        parse(&with(
            |value| value["revoked"] = json!([{"id": "a", "reason": "boring"}])
        )),
        Err(TalentIndexError::InvalidFormat)
    );
    assert_eq!(
        parse(&with(
            |value| value["revoked"] =
                json!([{"id": "a", "reason": "broken"}, {"id": "a", "reason": "policy"}])
        )),
        Err(TalentIndexError::InvalidFormat)
    );
    for reason in ["malicious", "broken", "authorRequest", "policy"] {
        assert!(
            parse(&with(
                |value| value["revoked"] = json!([{"id": "a", "reason": reason}])
            ))
            .is_ok(),
            "{reason}"
        );
    }
}

#[test]
fn the_base_url_is_https_or_loopback_http_without_credentials_query_or_fragment() {
    for safe in [
        "https://talents.example/v1/",
        "http://127.0.0.1:4000/files/",
        "http://localhost/x",
        "http://[::1]:8080/",
    ] {
        assert!(TalentIndex::is_safe_base_url(safe), "{safe}");
        assert!(
            parse(&with(|value| value["baseURL"] = json!(safe))).is_ok(),
            "{safe}"
        );
    }
    for unsafe_url in [
        "http://talents.example/",
        "https://user:pw@talents.example/",
        "https://talents.example/?a=1",
        "https://talents.example/#f",
        "ftp://talents.example/",
        "talents.example",
        "https://",
    ] {
        assert!(!TalentIndex::is_safe_base_url(unsafe_url), "{unsafe_url}");
        assert_eq!(
            parse(&with(|value| value["baseURL"] = json!(unsafe_url))),
            Err(TalentIndexError::InvalidFormat),
            "{unsafe_url}"
        );
    }
}
