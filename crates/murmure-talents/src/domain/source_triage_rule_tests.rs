use super::{SourceTriage, SourceTriageRule};
use super::test_fixtures::file;

fn triage(files: &[(&str, &str)]) -> SourceTriage {
    let files: Vec<_> = files.iter().map(|(path, text)| file(path, text.as_bytes())).collect();
    SourceTriageRule::triage(&files)
}

fn reason<'a>(triage: &'a SourceTriage, path: &str) -> &'a str {
    &triage.refusals.iter().find(|refusal| refusal.path == path).unwrap_or_else(|| panic!("no refusal for {path}")).reason
}

const SKILL: &str = "---\nname: Travel Coach\n---\nPlan a trip.\n";

#[test]
fn a_bundle_keeps_its_skills_and_sets_aside_mcp_servers_and_hooks() {
    let triage = triage(&[
        ("skills/travel/SKILL.md", SKILL),
        (".claude-plugin/plugin.json", r#"{"name":"bundle"}"#),
        (".mcp.json", r#"{"mcpServers":{"sample":{"command":"sample-mcp"}}}"#),
        ("hooks/hooks.json", r#"{"hooks":{}}"#),
    ]);
    assert_eq!(triage.skill_paths, ["skills/travel/SKILL.md"]);
    assert!(reason(&triage, ".mcp.json").contains("outils : V2"));
    assert!(reason(&triage, "hooks/hooks.json").contains("hook : non pris en charge"));
}

#[test]
fn components_declared_inline_in_the_bundle_manifest_are_reported_too() {
    let triage = triage(&[
        ("skills/travel/SKILL.md", SKILL),
        (".claude-plugin/plugin.json", r#"{"mcpServers":{"sample":{"command":"sample"}},"hooks":"automation/events.json"}"#),
        ("automation/events.json", "{}"),
    ]);
    assert!(reason(&triage, ".claude-plugin/plugin.json#mcpServers").contains("outils : V2"));
    assert!(reason(&triage, "automation/events.json").contains("hook : non pris en charge"));
}

#[test]
fn a_native_plugin_is_set_aside_with_its_reason_and_so_are_its_skills() {
    let tools = triage(&[("openclaw.plugin.json", r#"{"id":"sample","configSchema":{"type":"object"}}"#), ("SKILL.md", SKILL)]);
    assert!(tools.skill_paths.is_empty());
    let text = reason(&tools, "openclaw.plugin.json");
    assert!(text.contains("Plugin natif refusé") && text.contains("outils : V2"));
    let channel = triage(&[("openclaw.plugin.json", r#"{"id":"sample","channels":["chat"]}"#)]);
    assert!(reason(&channel, "openclaw.plugin.json").contains("canal"));
}
