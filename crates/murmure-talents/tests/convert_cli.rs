//! `convert` end to end on a tiny source, then `validate` on the converted folder. The Catalogue itself is validated by
//! its own CI (`talents-catalogue.yml`): this crate is also published alone, without it.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn tool(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_murmure-talents")).args(arguments).output().expect("the tool runs")
}

fn path(path: &Path) -> &str {
    path.to_str().expect("a UTF-8 path")
}

#[test]
fn converts_a_source_to_a_folder_that_validate_refuses_until_the_french_is_filled() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/convert-source");
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("converted");
    let convert = tool(&["convert", path(&source), "--id", "weekend-planner", "--output", path(&output), "--json"]);
    assert_eq!(convert.status.code(), Some(0), "{}", String::from_utf8_lossy(&convert.stderr));
    let report: serde_json::Value = serde_json::from_slice(&convert.stdout).expect("JSON on stdout");
    assert_eq!(report["talentIds"], serde_json::json!(["weekend-planner"]));
    let talent = output.join("talents/weekend-planner");
    assert_eq!(fs::read(talent.join("SKILL.md")).unwrap(), fs::read(source.join("SKILL.md")).unwrap());
    assert_eq!(fs::read(talent.join("LICENSE")).unwrap(), fs::read(source.join("LICENSE")).unwrap());

    assert_ne!(tool(&["validate", path(&output)]).status.code(), Some(0), "TODO-fr blocks validate");
    let manifest = fs::read_to_string(talent.join("talent.json")).unwrap();
    fs::write(
        talent.join("talent.json"),
        manifest.replace("TODO-fr", "Préparer un week-end"),
    )
    .unwrap();
    assert_eq!(tool(&["validate", path(&output)]).status.code(), Some(0));
}

fn skill_text(name: &str) -> String {
    format!(
        "---\nname: {name}\ndescription: Prepare an itinerary.\nauthor: Synthetic Author\nlicense: MIT\n---\n\
         Help plan a trip with the places the person gives you.\nReply in the language the person uses.\n"
    )
}

#[test]
fn applies_the_bounds_to_each_skill_of_a_large_source_and_ignores_hidden_folders() {
    let source = tempfile::tempdir().unwrap();
    let write = |relative: &str, data: &[u8]| {
        let target = source.path().join(relative);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, data).unwrap();
    };
    for number in 1..=20 {
        write(&format!("skills/cat/s{number}/SKILL.md"), skill_text(&format!("Coach {number}")).as_bytes());
        write(&format!("skills/cat/s{number}/LICENSE"), b"MIT License\n\nPermission is hereby granted, free of charge.\n");
    }
    for number in 0..33 {
        write(&format!("skills/cat/heavy/refs/r{number}.md"), b"x");
    }
    write("skills/cat/heavy/SKILL.md", skill_text("Heavy").as_bytes());
    write(".git/objects/ab/cd", b"not read");
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("converted");
    let convert = tool(&["convert", path(source.path()), "--output", path(&output), "--json"]);
    assert_eq!(convert.status.code(), Some(0), "{}", String::from_utf8_lossy(&convert.stderr));
    let report: serde_json::Value = serde_json::from_slice(&convert.stdout).expect("JSON on stdout");
    assert_eq!(report["talentIds"].as_array().unwrap().len(), 20);
    let heavy = report["items"].as_array().unwrap().iter().find(|item| item["sourcePath"] == "skills/cat/heavy/SKILL.md").unwrap();
    assert_eq!(heavy["status"], "refused");
    assert!(heavy["reason"].as_str().unwrap().contains("32 fichiers"));
}
