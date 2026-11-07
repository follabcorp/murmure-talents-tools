#![cfg(unix)]
//! The delegation and the publication end to end, with ephemeral keys: what `publish` writes is verified by the
//! Kit's `TalentCatalogTrust`, the code the Relay runs.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use base64::{engine::general_purpose::STANDARD, Engine};
use murmure_talents_kit::TalentCatalogTrust;

mod support;
use support::{tool, path as text, SKILL};

const MANIFEST: &str = r#"{"id":"travel-coach","version":"1.2.0","kind":"textOnly",
"localized":{"fr":{"name":"Préparer un voyage","summary":"Prépare un itinéraire."},"en":{"name":"Plan a trip","summary":"Prepare an itinerary."}},
"needs":["travel"],"capabilities":[],"requires":[],
"minimumMurmureVersion":"1.0.0","author":{"name":"Test Maintainer","github":["test-author"]},"license":"MIT"}"#;

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64
}

fn catalog(temp: &Path) -> PathBuf {
    let repository = temp.join("murmure-talents");
    let talent = repository.join("talents/travel-coach");
    fs::create_dir_all(&talent).unwrap();
    fs::write(talent.join("talent.json"), MANIFEST).unwrap();
    fs::write(talent.join("SKILL.md"), SKILL).unwrap();
    fs::write(repository.join("featured.json"), "[]").unwrap();
    fs::write(repository.join("revoked.json"), "[]").unwrap();
    repository
}

fn write_key(location: &Path, secret: u8, mode: u32) {
    fs::write(location, [secret; 32]).unwrap();
    fs::set_permissions(location, fs::Permissions::from_mode(mode)).unwrap();
}

/// The Base64 public key the tool prints for the ephemeral private key at `key`.
fn public_key(key: &Path) -> String {
    let printed = tool(&["publication-key", "--public", text(key)]);
    assert_eq!(printed.status.code(), Some(0), "{}", stderr(&printed));
    String::from_utf8(printed.stdout).unwrap().trim().to_owned()
}

fn delegate(root_key: &Path, publication_key: &str, valid_days: &str, output: &Path) -> Output {
    tool(&["delegate", "--root-key", text(root_key), "--publication-public-key", publication_key,
        "--sequence", "1", "--valid-days", valid_days, "--output", text(output)])
}

fn publish(repository: &Path, output: &Path, key: &Path, delegation: &Path, root_public: &str) -> Output {
    tool(&["publish", text(repository), "--output", text(output), "--base-url", "https://talents.example.invalid/v1/",
        "--sequence", "auto", "--key", text(key), "--delegation", text(delegation), "--root-public-key", root_public])
}

#[test]
fn a_delegation_then_a_publication_make_a_chain_the_kit_trusts() {
    let temp = tempfile::tempdir().unwrap();
    let repository = catalog(temp.path());
    let (root_key, publication_key) = (temp.path().join("root.key"), temp.path().join("publication.key"));
    write_key(&root_key, 1, 0o600);
    write_key(&publication_key, 2, 0o600);
    let (root_public, publication_public) = (public_key(&root_key), public_key(&publication_key));
    assert_ne!(root_public, publication_public);

    let delegation = temp.path().join("delegation");
    let signed = delegate(&root_key, &publication_public, "365", &delegation);
    assert_eq!(signed.status.code(), Some(0), "{}", stderr(&signed));
    let before = now();
    let output = temp.path().join("published");
    let result = publish(&repository, &output, &publication_key, &delegation, &root_public);
    assert_eq!(result.status.code(), Some(0), "{}", stderr(&result));

    let decode = |name: &str| STANDARD.decode(fs::read_to_string(output.join(name)).unwrap().trim()).unwrap();
    let verified = TalentCatalogTrust::verify(
        &fs::read(output.join("delegation.json")).unwrap(), &decode("delegation.json.sig"),
        &fs::read(output.join("index.json")).unwrap(), &decode("index.json.sig"),
        &STANDARD.decode(&root_public).unwrap(), now(), None, None,
    )
    .expect("the Kit trusts the published chain");
    assert_eq!(verified.delegation.sequence, 1);
    assert!((before..=now()).contains(&(verified.index.sequence as i64)), "auto sequence is the clock");
    assert_eq!(fs::read(output.join("travel-coach/1.2.0/SKILL.md")).unwrap(), SKILL.as_bytes());
}

#[test]
fn delegate_refuses_an_open_root_key_and_an_existing_output_and_writes_nothing() {
    let temp = tempfile::tempdir().unwrap();
    let (open_key, owner_key) = (temp.path().join("open.key"), temp.path().join("owner.key"));
    write_key(&open_key, 1, 0o644);
    write_key(&owner_key, 1, 0o600);
    let publication_public = STANDARD.encode([2u8; 32]);

    let output = temp.path().join("delegation");
    let open = delegate(&open_key, &publication_public, "365", &output);
    assert_eq!(open.status.code(), Some(1));
    assert!(stderr(&open).contains("La clé doit être un fichier 0600 hors du dépôt."), "{}", stderr(&open));
    assert!(!output.exists());

    fs::create_dir(&output).unwrap();
    let existing = delegate(&owner_key, &publication_public, "365", &output);
    assert_eq!(existing.status.code(), Some(1));
    assert!(stderr(&existing).contains("La sortie doit être un dossier neuf sous un parent existant."), "{}", stderr(&existing));
    assert_eq!(fs::read_dir(&output).unwrap().count(), 0);

    // A key kept in another repository is refused although the command runs from elsewhere.
    let other_repository = temp.path().join("other-repository");
    fs::create_dir_all(other_repository.join(".git")).unwrap();
    let kept_key = other_repository.join("kept.key");
    write_key(&kept_key, 1, 0o600);
    let elsewhere = temp.path().join("elsewhere");
    fs::create_dir(&elsewhere).unwrap();
    let kept = Command::new(env!("CARGO_BIN_EXE_murmure-talents"))
        .current_dir(&elsewhere)
        .args(["publication-key", "--public", text(&kept_key)])
        .output()
        .unwrap();
    assert_eq!(kept.status.code(), Some(1));
    assert!(stderr(&kept).contains("La clé doit être un fichier 0600 hors du dépôt."), "{}", stderr(&kept));
}

#[test]
fn publish_refuses_an_undelegated_key_or_a_delegation_ending_within_a_week_and_leaves_no_output() {
    let temp = tempfile::tempdir().unwrap();
    let repository = catalog(temp.path());
    let (root_key, publication_key, other_key) =
        (temp.path().join("root.key"), temp.path().join("publication.key"), temp.path().join("other.key"));
    write_key(&root_key, 1, 0o600);
    write_key(&publication_key, 2, 0o600);
    write_key(&other_key, 3, 0o600);
    let (root_public, publication_public) = (public_key(&root_key), public_key(&publication_key));

    let delegation = temp.path().join("delegation");
    assert_eq!(delegate(&root_key, &publication_public, "365", &delegation).status.code(), Some(0));
    let output = temp.path().join("published");
    let undelegated = publish(&repository, &output, &other_key, &delegation, &root_public);
    assert_eq!(undelegated.status.code(), Some(1));
    assert!(stderr(&undelegated).contains("déléguée"), "{}", stderr(&undelegated));
    assert!(!output.exists());

    let short = temp.path().join("short-delegation");
    assert_eq!(delegate(&root_key, &publication_public, "6", &short).status.code(), Some(0));
    let expiring = publish(&repository, &output, &publication_key, &short, &root_public);
    assert_eq!(expiring.status.code(), Some(1));
    assert!(stderr(&expiring).contains("7 jours"), "{}", stderr(&expiring));
    assert!(!output.exists());
}
