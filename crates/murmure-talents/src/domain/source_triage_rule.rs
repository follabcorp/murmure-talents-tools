//! The triage of a source snapshot (a skill, a bundle, a `skills/` folder of a clone, a repository of public
//! skills): which `SKILL.md` are candidates, which components are set aside and why. Pure: paths and bytes only.

use std::collections::BTreeMap;

use serde_json::Value;

use super::CatalogFile;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriageRefusal {
    /// Path in the source; `<plugin manifest>#mcpServers` or `#hooks` when the component is declared inline.
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceTriage {
    /// Paths of the retained `SKILL.md`, sorted.
    pub skill_paths: Vec<String>,
    /// Sorted by path.
    pub refusals: Vec<TriageRefusal>,
}

const BUNDLE_MANIFESTS: [&str; 4] = [".claude-plugin/plugin.json", ".codex-plugin/plugin.json", ".cursor-plugin/plugin.json", "plugin.json"];
const TOOLS: &str = "outils : V2";
const HOOK: &str = "hook : non pris en charge";

pub struct SourceTriageRule;

impl SourceTriageRule {
    /// Reasons (exact fragments): MCP server → `outils : V2`; hook → `hook : non pris en charge`; native plugin
    /// (`openclaw.plugin.json`) → `Plugin natif refusé — ` then `canal`, `fournisseur de modèles : déjà couvert` or
    /// `outils : V2` (checked in that order of the descriptor: channels, providers, otherwise tools). A `SKILL.md`
    /// next to or under a native plugin is not retained. Bundle manifests: `.claude-plugin/plugin.json`,
    /// `.codex-plugin/plugin.json`, `.cursor-plugin/plugin.json`, `plugin.json`.
    pub fn triage(files: &[CatalogFile]) -> SourceTriage {
        let mut refusals: BTreeMap<String, String> = BTreeMap::new();
        let mut plugin_roots: Vec<String> = Vec::new();
        for file in files {
            let name = file.path.rsplit('/').next().unwrap_or("");
            let folder = parent(&file.path);
            let json = || serde_json::from_slice::<Value>(&file.data).ok();
            match name {
                "openclaw.plugin.json" => {
                    plugin_roots.push(folder.to_owned());
                    refusals.insert(file.path.clone(), format!("Plugin natif refusé — {}.", native_plugin_scope(json().as_ref())));
                }
                ".mcp.json" => {
                    refusals.insert(file.path.clone(), format!("Serveur MCP écarté ({TOOLS})."));
                }
                "hooks.json" => {
                    refusals.insert(file.path.clone(), format!("Hook écarté ({HOOK})."));
                }
                _ => {}
            }
            if BUNDLE_MANIFESTS.iter().any(|manifest| file.path == *manifest || file.path.ends_with(&format!("/{manifest}"))) {
                let root = [".claude-plugin", ".codex-plugin", ".cursor-plugin"]
                    .iter()
                    .find_map(|hidden| folder.strip_suffix(hidden))
                    .unwrap_or(folder)
                    .trim_end_matches('/');
                if let Some(manifest) = json() {
                    declared(&manifest, "mcpServers", &file.path, root, TOOLS, "Serveur MCP écarté", &mut refusals);
                    declared(&manifest, "hooks", &file.path, root, HOOK, "Hook écarté", &mut refusals);
                }
            }
        }
        let mut skill_paths: Vec<String> = files
            .iter()
            .filter(|file| file.path == "SKILL.md" || file.path.ends_with("/SKILL.md"))
            .filter(|file| !plugin_roots.iter().any(|root| root.is_empty() || file.path.starts_with(&format!("{root}/"))))
            .map(|file| file.path.clone())
            .collect();
        skill_paths.sort();
        SourceTriage {
            skill_paths,
            refusals: refusals.into_iter().map(|(path, reason)| TriageRefusal { path, reason }).collect(),
        }
    }
}

fn parent(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(folder, _)| folder)
}

fn native_plugin_scope(descriptor: Option<&Value>) -> &'static str {
    let has = |key: &str| descriptor.and_then(|value| value.get(key)).is_some_and(|value| value.as_array().map_or(!value.is_null(), |items| !items.is_empty()));
    if has("channels") {
        "canal"
    } else if has("providers") {
        "fournisseur de modèles : déjà couvert"
    } else {
        TOOLS
    }
}

/// A component declared in a bundle manifest: inline (an object: `<manifest>#<key>`) or by path (relative to the
/// bundle root).
fn declared(manifest: &Value, key: &str, manifest_path: &str, root: &str, scope: &str, label: &str, refusals: &mut BTreeMap<String, String>) {
    let reason = format!("{label} ({scope}).");
    match manifest.get(key) {
        Some(Value::String(path)) => {
            let relative = path.trim_start_matches("./");
            let full = if root.is_empty() { relative.to_owned() } else { format!("{root}/{relative}") };
            refusals.insert(full, reason);
        }
        Some(Value::Object(_)) | Some(Value::Array(_)) => {
            refusals.insert(format!("{manifest_path}#{key}"), reason);
        }
        _ => {}
    }
}
