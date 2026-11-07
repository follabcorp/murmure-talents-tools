#![cfg(unix)]
//! The tool end to end, on a temporary Catalogue: `validate` → `build-index` → `sign` → the Kit verifies.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output};

use base64::{engine::general_purpose::STANDARD, Engine};
use ed25519_dalek::SigningKey;
use murmure_talents_kit::TalentIndexVerification;

const SKILL: &str = "---\nname: travel-coach\ndescription: Plan a trip.\n---\nSuggest a local itinerary.\n";
const MANIFEST: &str = r#"{"id":"travel-coach","version":"1.2.0","kind":"textOnly",
"localized":{"fr":{"name":"Préparer un voyage","summary":"Prépare un itinéraire."},"en":{"name":"Plan a trip","summary":"Prepare an itinerary."}},
"needs":["travel"],"capabilities":["browse-web"],"requires":["naviguer-sur-le-web"],
"minimumMurmureVersion":"1.0.0","author":{"name":"Test Maintainer","github":["test-author"]},"license":"MIT"}"#;

fn tool(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_murmure-talents")).args(arguments).output().expect("the tool runs")
}

fn path(path: &Path) -> &str {
    path.to_str().expect("a UTF-8 path")
}

/// `<temp>/murmure-talents` holds the Catalogue; `<temp>` is outside it.
fn populate(temp: &Path) -> std::path::PathBuf {
    let repository = temp.join("murmure-talents");
    let talent = repository.join("talents/travel-coach");
    fs::create_dir_all(&talent).unwrap();
    fs::write(talent.join("talent.json"), MANIFEST).unwrap();
    fs::write(talent.join("SKILL.md"), SKILL).unwrap();
    fs::write(repository.join("featured.json"), r#"["travel-coach"]"#).unwrap();
    fs::write(repository.join("revoked.json"), r#"[{"id":"retired-coach","reason":"policy"}]"#).unwrap();
    repository
}

fn write_key(location: &Path, bytes: &[u8; 32]) {
    fs::write(location, bytes).unwrap();
    fs::set_permissions(location, fs::Permissions::from_mode(0o600)).unwrap();
}

#[test]
fn validates_builds_and_signs_an_index_the_kit_accepts() {
    let temp = tempfile::tempdir().unwrap();
    let repository = populate(temp.path());
    let output = temp.path().join("published");
    assert_eq!(tool(&["validate", path(&repository)]).status.code(), Some(0));
    let build = tool(&["build-index", path(&repository), "--output", path(&output),
        "--base-url", "https://talents.example.invalid/v1/", "--sequence", "42"]);
    assert_eq!(build.status.code(), Some(0));

    // Ephemeral test material only; never the human Catalogue key.
    let secret = [7u8; 32];
    let key = SigningKey::from_bytes(&secret);
    let key_path = temp.path().join("catalog.key");
    write_key(&key_path, &secret);
    let index_path = output.join("index.json");
    let original = fs::read(&index_path).unwrap();
    let sign = tool(&["sign", path(&repository), "--index", path(&index_path), "--key", path(&key_path)]);
    assert_eq!(sign.status.code(), Some(0));
    assert_eq!(fs::read(&index_path).unwrap(), original);

    let encoded = fs::read_to_string(output.join("index.json.sig")).unwrap();
    let signature = STANDARD.decode(encoded.trim()).unwrap();
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
    let index = TalentIndexVerification::verify(&original, &signature, key.verifying_key().as_bytes(), now, Some(41))
        .expect("the Kit accepts the index and its signature");
    assert_eq!(index.featured, vec!["travel-coach".to_owned()]);
    assert_eq!(index.revoked.first().map(|revoked| revoked.id.as_str()), Some("retired-coach"));
    assert_eq!(fs::read(output.join("travel-coach/1.2.0/SKILL.md")).unwrap(), SKILL.as_bytes());
}

#[test]
fn sign_refuses_a_key_alias_resolving_inside_the_repository_and_writes_no_signature() {
    let temp = tempfile::tempdir().unwrap();
    let repository = populate(temp.path());
    let output = temp.path().join("published");
    let build = tool(&["build-index", path(&repository), "--output", path(&output),
        "--base-url", "https://talents.example.invalid/", "--sequence", "1"]);
    assert_eq!(build.status.code(), Some(0));
    let secret = [9u8; 32];
    let inside = repository.join("catalog.key");
    write_key(&inside, &secret);
    let alias = temp.path().join("outside.key");
    std::os::unix::fs::symlink(&inside, &alias).unwrap();
    let index_path = output.join("index.json");
    let result = tool(&["sign", path(&repository), "--index", path(&index_path), "--key", path(&alias)]);
    assert_ne!(result.status.code(), Some(0));
    assert!(!output.join("index.json.sig").exists());
    let shown = [result.stdout, result.stderr].concat();
    assert!(!String::from_utf8_lossy(&shown).contains(&STANDARD.encode(secret)));
}

/// A Catalogue root holding one Talent `travel-coach` at `version`, with the given capabilities and Skill text.
fn catalog_at(root: &Path, version: &str, capabilities: &str, requires: &str, skill: &str) {
    let talent = root.join("talents/travel-coach");
    fs::create_dir_all(&talent).unwrap();
    let manifest = MANIFEST
        .replace(r#""version":"1.2.0""#, &format!(r#""version":"{version}""#))
        .replace(r#""capabilities":["browse-web"]"#, &format!(r#""capabilities":{capabilities}"#))
        .replace(r#""requires":["naviguer-sur-le-web"]"#, &format!(r#""requires":{requires}"#))
        .replace(r#""author":{"name":"Test Maintainer"}"#, r#""author":{"name":"Test Maintainer","github":["test-author"]}"#);
    fs::write(talent.join("talent.json"), manifest).unwrap();
    fs::write(talent.join("SKILL.md"), skill).unwrap();
}

#[test]
fn diff_refuses_a_changed_file_without_a_version_raise() {
    let temp = tempfile::tempdir().unwrap();
    let (base, head) = (temp.path().join("base"), temp.path().join("head"));
    catalog_at(&base, "1.2.0", "[]", "[]", SKILL);
    catalog_at(&head, "1.2.0", "[]", "[]", &SKILL.replace("local", "global"));
    let result = tool(&["diff", path(&base), path(&head)]);
    assert_ne!(result.status.code(), Some(0));
    let text = String::from_utf8_lossy(&result.stdout);
    assert!(text.contains("ERREUR [travel-coach]"), "{text}");
}

#[test]
fn diff_json_lists_an_added_capability_on_a_correct_version_raise() {
    let temp = tempfile::tempdir().unwrap();
    let (base, head) = (temp.path().join("base"), temp.path().join("head"));
    catalog_at(&base, "1.2.0", "[]", "[]", SKILL);
    catalog_at(&head, "1.3.0", r#"["read-mail"]"#, "[]", SKILL);
    let result = tool(&["diff", path(&base), path(&head), "--json"]);
    assert_eq!(result.status.code(), Some(0));
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).expect("JSON on stdout");
    assert_eq!(report["valid"], true);
    assert_eq!(report["errors"], serde_json::json!([]));
    assert_eq!(report["talents"][0]["talentId"], "travel-coach");
    assert_eq!(report["talents"][0]["change"], "modified");
    assert_eq!(report["talents"][0]["addedCapabilities"], serde_json::json!(["read-mail"]));
    assert_eq!(report["talents"][0]["addedDependencies"], serde_json::json!([]));
}
