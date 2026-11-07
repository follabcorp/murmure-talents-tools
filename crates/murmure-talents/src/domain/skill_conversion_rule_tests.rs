use murmure_talents_kit::{TalentKind, TalentManifest, TalentManifestRule, TalentOriginKind, TalentPlatform, TalentSource};

use super::test_fixtures::{file, validate};
use super::{CatalogFile, ConversionReport, ConversionStatus, SkillConversionRule};

const LICENSE: &str = "MIT License\n\nCopyright (c) 2026 Synthetic Author\n\nPermission is hereby granted, free of charge.\n";

fn convert_in_memory(files: &[CatalogFile], id: Option<&str>) -> ConversionReport {
    SkillConversionRule::convert(files, &[], id)
}

fn skill_text(license: &str, metadata: &str, body: &str) -> String {
    format!(
        "---\nname: Travel Coach\ndescription: Prepare an itinerary.\nauthor: Synthetic Author\n\
         homepage: https://example.invalid/synthetic-travel-coach\nlicense: {license}\nmetadata: {metadata}\n---\n{body}\n"
    )
}

fn skill_at(directory: &str, license: &str, metadata: &str, body: &str) -> Vec<CatalogFile> {
    let prefix = if directory.is_empty() { String::new() } else { format!("{directory}/") };
    vec![
        file(&format!("{prefix}SKILL.md"), skill_text(license, metadata, body).as_bytes()),
        file(&format!("{prefix}LICENSE"), LICENSE.as_bytes()),
    ]
}

fn simple(metadata: &str) -> Vec<CatalogFile> {
    skill_at("", "MIT", metadata, "Help plan a trip with the places the person gives you.\nReply in the language the person uses.")
}

fn manifest_of(report: &ConversionReport) -> TalentManifest {
    let talent = report.talents.first().expect("one proposal");
    TalentManifestRule::parse(&talent.manifest_json, TalentSource::LocalFolder).expect("a readable manifest")
}

fn reason<'a>(report: &'a ConversionReport, path: &str) -> (&'a ConversionStatus, &'a str) {
    let item = report.items.iter().find(|item| item.source_path == path).unwrap_or_else(|| panic!("no item for {path}"));
    (&item.status, &item.reason)
}

#[test]
fn converts_a_simple_skill_without_inventing_translations_or_rights() {
    let files = simple("{}");
    let report = convert_in_memory(&files, None);
    assert_eq!(report.talents.iter().map(|talent| talent.directory_id.as_str()).collect::<Vec<_>>(), ["travel-coach"]);
    let manifest = manifest_of(&report);
    assert_eq!(manifest.kind, TalentKind::TextOnly);
    assert_eq!((manifest.localized["fr"].name.as_str(), manifest.localized["fr"].summary.as_str()), ("TODO-fr", "TODO-fr"));
    assert_eq!(
        (manifest.localized["en"].name.as_str(), manifest.localized["en"].summary.as_str()),
        ("Travel Coach", "Prepare an itinerary.")
    );
    assert_eq!((manifest.author.name.as_str(), manifest.license.as_str()), ("Synthetic Author", "MIT"));
    let origin = manifest.origin.expect("an origin");
    assert_eq!(origin.kind, TalentOriginKind::Public);
    assert_eq!(origin.author, "Synthetic Author");
    assert_eq!(origin.url.as_deref(), Some("https://example.invalid/synthetic-travel-coach"));
    let talent = &report.talents[0];
    let copied = |path: &str| talent.files.iter().find(|file| file.path == path).map(|file| file.data.clone());
    assert_eq!(copied("SKILL.md"), Some(files[0].data.clone()));
    assert_eq!(copied("LICENSE"), Some(LICENSE.as_bytes().to_vec()));
    assert_eq!(reason(&report, "SKILL.md").0, &ConversionStatus::Accepted);
    assert!(!validate(talent.clone(), vec![], &[]).is_valid(), "TODO-fr must block validation");
}

#[test]
fn the_maintainer_may_force_the_identifier_and_openclaw_metadata_sets_origin_and_platform() {
    let report = convert_in_memory(&simple(r#"{"openclaw":{"os":["darwin"]}}"#), Some("weekend-planner"));
    assert_eq!(report.talents[0].directory_id, "weekend-planner");
    let manifest = manifest_of(&report);
    assert_eq!(manifest.id, "weekend-planner");
    assert_eq!(manifest.platforms, Some(vec![TalentPlatform::Macos]));
    assert_eq!(manifest.origin.expect("an origin").kind, TalentOriginKind::Openclaw);
}

#[test]
fn converts_a_hermes_skill_that_sits_under_a_category() {
    let files = skill_at("skills/productivity/travel-coach", "MIT", "{}", "Plan trips.\nReply in the language the person uses.");
    let report = convert_in_memory(&files, None);
    assert_eq!(report.talents.len(), 1);
    assert_eq!(manifest_of(&report).origin.expect("an origin").kind, TalentOriginKind::Hermes);
    let paths: Vec<_> = report.talents[0].files.iter().map(|file| file.path.as_str()).collect();
    assert!(paths.contains(&"SKILL.md") && paths.contains(&"LICENSE"), "{paths:?}");
    assert_eq!(reason(&report, "skills/productivity/travel-coach/SKILL.md").0, &ConversionStatus::Accepted);
}

#[test]
fn sorts_a_bundle_into_a_proposal_and_explained_refusals() {
    let mut files = skill_at("skills/travel", "MIT", "{}", "Plan trips.\nReply in the language the person uses.");
    files.push(file(".mcp.json", br#"{"mcpServers":{"sample":{"command":"sample-mcp"}}}"#));
    files.push(file("hooks/hooks.json", br#"{"hooks":{}}"#));
    let report = convert_in_memory(&files, None);
    assert_eq!(report.talents.len(), 1);
    assert_eq!(reason(&report, "skills/travel/SKILL.md").0, &ConversionStatus::Accepted);
    let (status, mcp) = reason(&report, ".mcp.json");
    assert_eq!(status, &ConversionStatus::Refused);
    assert!(mcp.contains("outils : V2"));
    assert!(reason(&report, "hooks/hooks.json").1.contains("hook : non pris en charge"));
    assert!(report.talents[0].files.iter().all(|file| file.path != ".mcp.json" && !file.path.starts_with("hooks/")));
}

#[test]
fn sets_aside_a_native_plugin() {
    let files = vec![file("openclaw.plugin.json", br#"{"id":"sample","providers":["sample-model"]}"#)];
    let report = convert_in_memory(&files, None);
    assert!(report.talents.is_empty());
    let (status, text) = reason(&report, "openclaw.plugin.json");
    assert_eq!(status, &ConversionStatus::Refused);
    assert!(text.contains("fournisseur de modèles : déjà couvert"));
}

#[test]
fn refuses_a_license_outside_the_accepted_list() {
    let report = convert_in_memory(&skill_at("", "GPL-3.0-only", "{}", "Plan trips."), None);
    assert!(report.talents.is_empty());
    let (status, text) = reason(&report, "SKILL.md");
    assert_eq!(status, &ConversionStatus::Refused);
    assert!(text.contains("Licence refusée") && text.contains("GPL-3.0-only"), "{text}");
}

#[test]
fn refuses_a_skill_that_needs_a_binary_or_an_environment_variable() {
    for (requirement, value, word) in [("bins", "sample-tool", "Binaire"), ("env", "SAMPLE_API_KEY", "environnement")] {
        let metadata = format!(r#"{{"openclaw":{{"requires":{{"{requirement}":["{value}"]}}}}}}"#);
        let report = convert_in_memory(&skill_at("", "MIT", &metadata, "Plan trips."), None);
        assert!(report.talents.is_empty(), "{requirement}");
        let (status, text) = reason(&report, "SKILL.md");
        assert_eq!(status, &ConversionStatus::Refused);
        assert!(text.contains(value) && text.contains(word), "{text}");
    }
}

#[test]
fn flags_non_english_instructions_in_the_report_but_keeps_the_proposal() {
    let french = "Tu aides la personne à préparer son voyage. Propose un itinéraire avec les lieux que la personne a donnés, \
                  et pose une question quand il manque une information pour la suite.";
    let report = convert_in_memory(&skill_at("", "MIT", "{}", french), None);
    assert_eq!(report.talents.len(), 1);
    assert!(reason(&report, "SKILL.md").1.contains("consignes à traduire en anglais"));
    let english = convert_in_memory(&simple("{}"), None);
    assert!(!reason(&english, "SKILL.md").1.contains("traduire en anglais"));
}

#[test]
fn the_report_is_readable_as_text_and_as_json() {
    let mut files = simple("{}");
    files.push(file(".mcp.json", b"{}"));
    let report = convert_in_memory(&files, None);
    let text = report.text();
    assert!(text.contains("ACCEPTÉ [SKILL.md]") && text.contains("REFUSÉ [.mcp.json]") && text.contains("TODO-fr"), "{text}");
    let json: serde_json::Value = serde_json::from_str(&report.json()).expect("valid JSON");
    assert_eq!(json["talentIds"], serde_json::json!(["travel-coach"]));
    let item = json["items"].as_array().expect("items").iter().find(|item| item["sourcePath"] == ".mcp.json").expect("an item");
    assert_eq!(item["status"], "refused");
}

fn skills_under_category(count: usize, extra_files: usize) -> Vec<CatalogFile> {
    (1..=count)
        .flat_map(|number| {
            let text = skill_text("MIT", "{}", "Help plan a trip with the places the person gives you.\nReply in the language the person uses.")
                .replace("Travel Coach", &format!("Coach {number}"));
            let mut files = vec![file(&format!("skills/cat/s{number}/SKILL.md"), text.as_bytes())];
            files.extend((0..extra_files).map(|index| file(&format!("skills/cat/s{number}/notes/n{index}.md"), b"note")));
            files
        })
        .chain([file("skills/cat/LICENSE", LICENSE.as_bytes())])
        .collect()
}

#[test]
fn bounds_apply_to_each_skill_not_to_the_whole_source() {
    let files = skills_under_category(20, 2);
    assert!(files.len() > 32);
    let report = convert_in_memory(&files, None);
    assert_eq!(report.talents.len(), 20);
    assert!(report.talents.iter().all(|talent| talent.files.len() == 4), "SKILL.md, two notes, the inherited LICENSE");
}

#[test]
fn a_skill_over_the_bounds_is_refused_with_its_reason_and_the_others_are_converted() {
    let mut files = skills_under_category(2, 1);
    files.extend((0..32).map(|index| file(&format!("skills/cat/s2/more/m{index}.md"), b"x")));
    let heavy = file("skills/cat/s1/notes/big.md", &vec![b'a'; 200 * 1024]);
    files.push(heavy);
    files.push(file("skills/cat/s1/notes/big2.md", &vec![b'a'; 100 * 1024]));
    let report = convert_in_memory(&files, None);
    assert_eq!(report.talents.len(), 0);
    assert_eq!(reason(&report, "skills/cat/s1/SKILL.md").1, "Skill de plus de 256 Kio.");
    assert_eq!(reason(&report, "skills/cat/s2/SKILL.md").1, "Skill de plus de 32 fichiers.");
    let mut small = skills_under_category(3, 1);
    small.extend((0..32).map(|index| file(&format!("skills/cat/s3/more/m{index}.md"), b"x")));
    let report = convert_in_memory(&small, None);
    assert_eq!(report.talents.len(), 2);
    assert_eq!(reason(&report, "skills/cat/s3/SKILL.md").1, "Skill de plus de 32 fichiers.");
}

#[test]
fn a_skill_holding_a_file_too_large_to_be_read_is_refused() {
    let files = skills_under_category(2, 0);
    let oversized = ["skills/cat/s2/assets/photo.png".to_owned(), "README.md".to_owned()];
    let report = SkillConversionRule::convert(&files, &oversized, None);
    assert_eq!(report.talents.len(), 1);
    assert_eq!(reason(&report, "skills/cat/s2/SKILL.md").1, "Fichier de plus de 256 Kio : skills/cat/s2/assets/photo.png.");
}
