use serde_json::{json, Value};

use super::fixtures::manifest;
use super::{TalentCapability, TalentExportError, TalentExportProjection};

fn skill() -> Vec<(String, Vec<u8>)> {
    vec![
        (
            "SKILL.md".to_owned(),
            b"---\nname: ma-competence\ndescription: Planifie un menu.\n---\nCorps.\n".to_vec(),
        ),
        ("exemples/menu.md".to_owned(), b"Lundi : soupe.\n".to_vec()),
    ]
}

fn paths(files: &[super::TalentExportFile]) -> Vec<&str> {
    files.iter().map(|file| file.path.as_str()).collect()
}

fn content<'a>(files: &'a [super::TalentExportFile], path: &str) -> &'a str {
    &files
        .iter()
        .find(|file| file.path == path)
        .expect("a file")
        .content
}

#[test]
fn a_talent_with_a_manifest_keeps_its_facts_and_gets_an_author_an_empty_github_and_an_mit_license()
{
    let source = manifest(
        "trier-mes-mails",
        "1.2.0",
        &[TalentCapability::ReadMail],
        &["mes-mails"],
    );
    let files = TalentExportProjection::folder(
        "trier-mes-mails",
        Some(&source),
        &skill(),
        "Camille Dupont",
        2026,
    )
    .expect("an exported folder");
    assert_eq!(
        paths(&files),
        ["LICENSE", "SKILL.md", "exemples/menu.md", "talent.json"]
    );
    let exported: Value = serde_json::from_str(content(&files, "talent.json")).expect("JSON");
    assert_eq!(exported["id"], "trier-mes-mails");
    assert_eq!(exported["version"], "1.2.0");
    assert_eq!(exported["kind"], "textOnly");
    assert_eq!(
        exported["localized"],
        json!({
            "fr": {"name": "Talent trier-mes-mails", "summary": "Résumé de Talent trier-mes-mails."},
            "en": {"name": "Talent trier-mes-mails (en)", "summary": "Résumé de Talent trier-mes-mails (en)."}
        })
    );
    assert_eq!(exported["needs"], json!(["organizing"]));
    assert_eq!(exported["capabilities"], json!(["read-mail"]));
    assert_eq!(exported["requires"], json!(["mes-mails"]));
    assert_eq!(exported["minimumMurmureVersion"], "1.0.0");
    assert_eq!(
        exported["author"],
        json!({"name": "Camille Dupont", "github": []})
    );
    assert_eq!(exported["license"], "MIT");
    let license = content(&files, "LICENSE");
    assert!(license.starts_with("MIT License"), "{license}");
    assert!(
        license.contains("Copyright (c) 2026 Camille Dupont"),
        "{license}"
    );
    assert_eq!(content(&files, "exemples/menu.md"), "Lundi : soupe.\n");
}

#[test]
fn a_lone_skill_becomes_a_talent_named_after_its_identifier_described_by_its_skill() {
    let files = TalentExportProjection::folder("ma-competence", None, &skill(), "Murmure", 2026)
        .expect("a folder");
    let exported: Value = serde_json::from_str(content(&files, "talent.json")).expect("JSON");
    assert_eq!(exported["id"], "ma-competence");
    assert_eq!(exported["version"], "1.0.0");
    assert_eq!(exported["kind"], "textOnly");
    assert_eq!(
        exported["localized"]["fr"],
        json!({"name": "ma-competence", "summary": "Planifie un menu."})
    );
    assert_eq!(exported["needs"].as_array().map(Vec::len), Some(1));
    assert_eq!(exported["capabilities"], json!([]));
    assert_eq!(exported["requires"], json!([]));
    assert_eq!(exported["minimumMurmureVersion"], "1.0.0");
}

#[test]
fn the_name_of_the_author_is_trimmed_and_bounded_to_64_characters_without_control_characters() {
    assert_eq!(
        TalentExportProjection::author_name("  Camille Dupont \n"),
        Ok("Camille Dupont".to_owned())
    );
    assert_eq!(
        TalentExportProjection::author_name(&"é".repeat(64)),
        Ok("é".repeat(64))
    );
    for refused in [
        "",
        "   ",
        "Cam\u{7}ille",
        "Cam\nille",
        "Cam\u{2028}ille",
        "Cam\u{2029}ille",
        "Cam\u{202E}ille",
        "Cam\u{2066}ille",
        "Cam\u{2069}ille",
    ] {
        assert_eq!(
            TalentExportProjection::author_name(refused),
            Err(TalentExportError::InvalidAuthorName),
            "{refused:?}"
        );
    }
    assert_eq!(
        TalentExportProjection::author_name(&"é".repeat(65)),
        Err(TalentExportError::InvalidAuthorName)
    );
    let source = manifest("ma-competence", "1.0.0", &[], &[]);
    assert_eq!(
        TalentExportProjection::folder("ma-competence", Some(&source), &skill(), "", 2026),
        Err(TalentExportError::InvalidAuthorName)
    );
}

#[test]
fn a_file_that_is_not_text_is_refused() {
    let mut files = skill();
    files.push(("binaire.bin".to_owned(), vec![0xff, 0xfe, 0x00]));
    assert_eq!(
        TalentExportProjection::folder("ma-competence", None, &files, "Murmure", 2026),
        Err(TalentExportError::NotText)
    );
}

#[test]
fn a_missing_english_text_is_a_copy_of_the_french_one() {
    let mut source = manifest("trier-mes-mails", "1.0.0", &[], &[]);
    source.localized.remove("en");
    let files =
        TalentExportProjection::folder("trier-mes-mails", Some(&source), &skill(), "Murmure", 2026)
            .expect("a folder");
    let exported: Value = serde_json::from_str(content(&files, "talent.json")).expect("JSON");
    assert_eq!(exported["localized"]["en"], exported["localized"]["fr"]);
}

#[test]
fn a_file_of_the_skill_that_meets_a_generated_file_whatever_its_case_is_refused() {
    for homonym in ["talent.json", "LICENSE", "license", "Talent.JSON"] {
        let mut files = skill();
        files.push((homonym.to_owned(), b"x".to_vec()));
        assert_eq!(
            TalentExportProjection::folder("ma-competence", None, &files, "Murmure", 2026),
            Err(TalentExportError::PathCollision),
            "{homonym}"
        );
    }
    let mut twins = skill();
    twins.push(("Exemples/Menu.md".to_owned(), b"x".to_vec()));
    assert_eq!(
        TalentExportProjection::folder("ma-competence", None, &twins, "Murmure", 2026),
        Err(TalentExportError::PathCollision)
    );
}
