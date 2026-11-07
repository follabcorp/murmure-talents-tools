//! The conversion of a source snapshot into Catalogue proposals to review (ticket talents/12). Pure: the files are
//! given in memory, paths relative to the source folder; no link is followed, nothing is executed.

use std::collections::BTreeSet;

use serde_json::{json, Map, Value};

use super::catalog_validation_rule::{LICENSES, TRANSLATION_PLACEHOLDER};
use super::{CatalogFile, CatalogSource, InstructionLanguageRule, SourceTriageRule};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionStatus {
    Accepted,
    Refused,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversionItem {
    pub source_path: String,
    pub status: ConversionStatus,
    pub reason: String,
    pub talent_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversionReport {
    /// The proposals, each a `CatalogSource` ready to be written under `talents/<directory_id>/`.
    pub talents: Vec<CatalogSource>,
    /// Sorted by `source_path`.
    pub items: Vec<ConversionItem>,
}

impl ConversionReport {
    /// First line `Conversion — propositions à relire (français, besoins, capacités et droits).`, then one line per
    /// item `ACCEPTÉ [<path>] <reason>` / `REFUSÉ [<path>] <reason>`, then
    /// `Le texte source est copié sans exécution. TODO-fr bloque validate et build-index.`
    pub fn text(&self) -> String {
        let mut lines = vec!["Conversion — propositions à relire (français, besoins, capacités et droits).".to_owned()];
        for item in &self.items {
            let label = match item.status {
                ConversionStatus::Accepted => "ACCEPTÉ",
                ConversionStatus::Refused => "REFUSÉ",
            };
            lines.push(format!("{label} [{}] {}", item.source_path, item.reason));
        }
        lines.push(format!("Le texte source est copié sans exécution. {TRANSLATION_PLACEHOLDER} bloque validate et build-index."));
        lines.join("\n") + "\n"
    }

    /// Pretty JSON with sorted keys: `{"items":[{"reason","sourcePath","status":"accepted"|"refused","talentId"}],
    /// "review":"…","talentIds":["…"]}`.
    pub fn json(&self) -> String {
        let items: Vec<Value> = self
            .items
            .iter()
            .map(|item| {
                let status = match item.status {
                    ConversionStatus::Accepted => "accepted",
                    ConversionStatus::Refused => "refused",
                };
                json!({"sourcePath": item.source_path, "status": status, "reason": item.reason, "talentId": item.talent_id})
            })
            .collect();
        let ids: Vec<&str> = self.talents.iter().map(|talent| talent.directory_id.as_str()).collect();
        let report = json!({
            "items": items,
            "review": format!("Propositions à relire : français, besoins, capacités et droits. {TRANSLATION_PLACEHOLDER} bloque validate."),
            "talentIds": ids,
        });
        serde_json::to_string_pretty(&report).unwrap_or_default() + "\n"
    }
}

pub struct SkillConversionRule;

/// The bounds of `validate`, applied to each skill: a skill over them is refused, the others are converted.
const SKILL_FILES_MAX: usize = 32;
const SKILL_BYTES_MAX: usize = 256 * 1024;

impl SkillConversionRule {
    /// Triage (`SourceTriageRule`), then one proposal per retained skill. `oversized` lists source paths too large to
    /// have been read: the skill owning one is refused. A skill over 32 files or 256 Kio (its own files and the license
    /// it inherits) is refused too. `id` forces the identifier and needs a single
    /// skill. Refusals (reason fragments, French): `Licence refusée : <licence>` (outside the C9 list),
    /// `Binaire requis : <noms>`, `Variable d’environnement requise : <noms>`. An accepted skill whose
    /// instructions are not English says `consignes à traduire en anglais` in its reason.
    ///
    /// Proposal: `SKILL.md` and attachments copied byte for byte (relative to the skill folder), the nearest `LICENSE`
    /// copied; manifest `textOnly` version `1.0.0`, `localized.en` = name and description of the front matter,
    /// `localized.fr` = `TODO-fr` (name and summary), author = the original author (no GitHub account), license,
    /// `origin` = `{kind, author, url: homepage}`: kind `hermes` for `skills/<category>/<skill>/SKILL.md`, `openclaw`
    /// when the metadata holds an `openclaw` key (`os` → platforms, `requires` → refusals), otherwise `public`.
    pub fn convert(files: &[CatalogFile], oversized: &[String], id: Option<&str>) -> ConversionReport {
        let triage = SourceTriageRule::triage(files);
        let mut items: Vec<ConversionItem> = triage
            .refusals
            .iter()
            .map(|refusal| ConversionItem {
                source_path: refusal.path.clone(),
                status: ConversionStatus::Refused,
                reason: refusal.reason.clone(),
                talent_id: None,
            })
            .collect();
        let refused_paths: BTreeSet<&str> = triage.refusals.iter().map(|refusal| refusal.path.as_str()).collect();
        let skill_folders: Vec<&str> = triage.skill_paths.iter().map(|path| folder_of(path)).collect();
        let mut talents: Vec<CatalogSource> = Vec::new();
        for path in &triage.skill_paths {
            let folder = folder_of(path);
            let refuse = |reason: String| ConversionItem {
                source_path: path.clone(),
                status: ConversionStatus::Refused,
                reason,
                talent_id: None,
            };
            if id.is_some() && triage.skill_paths.len() > 1 {
                items.push(refuse("--id exige une seule skill dans la source.".to_owned()));
                continue;
            }
            let Some(skill) = files.iter().find(|file| &file.path == path) else { continue };
            let text = String::from_utf8_lossy(&skill.data);
            let header = FrontMatter::parse(&text);
            let owned: Vec<&CatalogFile> = files
                .iter()
                .filter(|file| inside(&file.path, folder) && !refused_paths.contains(file.path.as_str()))
                .filter(|file| !in_nested_skill(&file.path, folder, &skill_folders))
                .collect();
            let license_file = nearest_license(files, folder);
            if let Some(path) = oversized.iter().find(|path| inside(path, folder) && !in_nested_skill(path, folder, &skill_folders)) {
                items.push(refuse(format!("Fichier de plus de {} Kio : {path}.", SKILL_BYTES_MAX / 1024)));
                continue;
            }
            let files_count = owned.len() + usize::from(license_file.is_some_and(|license| !inside(&license.path, folder)));
            if files_count > SKILL_FILES_MAX {
                items.push(refuse(format!("Skill de plus de {SKILL_FILES_MAX} fichiers.")));
                continue;
            }
            let bytes: usize = owned.iter().map(|file| file.data.len()).sum::<usize>() + license_file.map_or(0, |license| license.data.len());
            if bytes > SKILL_BYTES_MAX {
                items.push(refuse(format!("Skill de plus de {} Kio.", SKILL_BYTES_MAX / 1024)));
                continue;
            }
            if let Some(reason) = refusal_reason(&header, license_file) {
                items.push(refuse(reason));
                continue;
            }
            let talent_id = match id.map(str::to_owned).or_else(|| derive_id(&header, folder)) {
                Some(talent_id) if murmure_talents_kit::is_valid_identifier(&talent_id) => talent_id,
                _ => {
                    items.push(refuse("Identifiant impossible à déduire : utiliser --id.".to_owned()));
                    continue;
                }
            };
            if talents.iter().any(|talent| talent.directory_id == talent_id) {
                items.push(refuse(format!("Identifiant déjà pris par une autre skill : {talent_id}.")));
                continue;
            }
            let mut copied: Vec<CatalogFile> = owned
                .iter()
                .map(|file| CatalogFile { path: relative(&file.path, folder).to_owned(), ..(*file).clone() })
                .collect();
            if let Some(license) = license_file.filter(|license| !inside(&license.path, folder)) {
                copied.push(CatalogFile { path: "LICENSE".to_owned(), ..license.clone() });
            }
            copied.sort_by(|left, right| left.path.cmp(&right.path));
            let mut reason = "proposition à relire (français, besoins, capacités)".to_owned();
            if InstructionLanguageRule::seems_not_english(&text) {
                reason.push_str(" ; consignes à traduire en anglais");
            }
            items.push(ConversionItem {
                source_path: path.clone(),
                status: ConversionStatus::Accepted,
                reason,
                talent_id: Some(talent_id.clone()),
            });
            let manifest_json = manifest(&talent_id, &header, folder).to_string().into_bytes();
            talents.push(CatalogSource { directory_id: talent_id, manifest_json, files: copied });
        }
        items.sort_by(|left, right| left.source_path.cmp(&right.source_path));
        ConversionReport { talents, items }
    }
}

/// Whether `path` belongs to a skill folder nested under `folder` (and not to `folder` itself).
fn in_nested_skill(path: &str, folder: &str, skill_folders: &[&str]) -> bool {
    skill_folders.iter().any(|other| !other.is_empty() && other.len() > folder.len() && inside(path, other))
}

fn folder_of(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(folder, _)| folder)
}

fn inside(path: &str, folder: &str) -> bool {
    folder.is_empty() || path.starts_with(&format!("{folder}/"))
}

fn relative<'a>(path: &'a str, folder: &str) -> &'a str {
    if folder.is_empty() { path } else { &path[folder.len() + 1..] }
}

/// The `LICENSE` file at the skill folder or the nearest folder above it.
fn nearest_license<'a>(files: &'a [CatalogFile], folder: &str) -> Option<&'a CatalogFile> {
    let mut current = folder;
    loop {
        let found = files.iter().find(|file| {
            folder_of(&file.path) == current && {
                let name = file.path.rsplit('/').next().unwrap_or("").to_ascii_lowercase();
                name.starts_with("license") || name.starts_with("licence")
            }
        });
        if found.is_some() || current.is_empty() {
            return found;
        }
        current = folder_of(current);
    }
}

fn refusal_reason(header: &FrontMatter, license_file: Option<&CatalogFile>) -> Option<String> {
    let mut missing = Vec::new();
    if !header.binaries.is_empty() {
        missing.push(format!("Binaire requis : {}", header.binaries.join(", ")));
    }
    if !header.environment.is_empty() {
        missing.push(format!("Variable d’environnement requise : {}", header.environment.join(", ")));
    }
    if !missing.is_empty() {
        return Some(missing.join(" ; ") + ".");
    }
    let license = header.license.clone().or_else(|| license_file.and_then(|file| detect_license(&file.data)));
    match license {
        Some(license) if LICENSES.contains(&license.as_str()) => None,
        Some(license) => Some(format!("Licence refusée : {license}.")),
        None => Some("Licence refusée : non déclarée.".to_owned()),
    }
}

fn detect_license(data: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(data);
    let head: String = text.chars().take(400).collect();
    if head.trim_start().starts_with("MIT License") || head.contains("Permission is hereby granted, free of charge") {
        Some("MIT".to_owned())
    } else if head.contains("Apache License") && text.contains("Version 2.0") {
        Some("Apache-2.0".to_owned())
    } else {
        None
    }
}

fn derive_id(header: &FrontMatter, folder: &str) -> Option<String> {
    let source = header.name.clone().filter(|name| !name.is_empty()).or_else(|| {
        let last = folder.rsplit('/').next().unwrap_or("");
        (!last.is_empty()).then(|| last.to_owned())
    })?;
    let mut slug = String::new();
    for character in source.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            slug.push(character);
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug: String = slug.trim_matches('-').chars().take(64).collect();
    let slug = slug.trim_end_matches('-').to_owned();
    (!slug.is_empty()).then_some(slug)
}

/// `skills/<category>/<skill>`, possibly below the folder of a clone.
fn is_hermes_layout(folder: &str) -> bool {
    let segments: Vec<&str> = folder.split('/').collect();
    segments.len() >= 3 && segments[segments.len() - 3] == "skills"
}

fn truncated(text: &str, limit: usize) -> String {
    text.chars().take(limit).collect::<String>().trim().to_owned()
}

fn manifest(id: &str, header: &FrontMatter, folder: &str) -> Value {
    let name = header.name.clone().filter(|name| !name.is_empty()).unwrap_or_else(|| id.to_owned());
    let summary = header.description.clone().filter(|text| !text.is_empty()).unwrap_or_else(|| name.clone());
    let author = header.author.clone().filter(|author| !author.is_empty()).unwrap_or_else(|| "À renseigner".to_owned());
    let kind = if header.openclaw {
        "openclaw"
    } else if is_hermes_layout(folder) {
        "hermes"
    } else {
        "public"
    };
    let mut origin = Map::new();
    origin.insert("kind".to_owned(), json!(kind));
    origin.insert("author".to_owned(), json!(truncated(&author, 64)));
    if let Some(url) = &header.homepage {
        origin.insert("url".to_owned(), json!(url));
    }
    let mut value = json!({
        "id": id,
        "version": "1.0.0",
        "kind": "textOnly",
        "localized": {
            "fr": {"name": TRANSLATION_PLACEHOLDER, "summary": TRANSLATION_PLACEHOLDER},
            "en": {"name": truncated(&name, 40), "summary": truncated(&summary, 140)},
        },
        "needs": ["organizing"],
        "capabilities": [],
        "requires": [],
        "minimumMurmureVersion": "1.0.0",
        "author": {"name": truncated(&author, 64)},
        "license": header.license.clone().unwrap_or_else(|| "MIT".to_owned()),
        "origin": origin,
    });
    if !header.platforms.is_empty() {
        value["platforms"] = json!(header.platforms);
    }
    value
}

/// What the conversion reads of the front matter of a `SKILL.md`.
#[derive(Default)]
struct FrontMatter {
    name: Option<String>,
    description: Option<String>,
    author: Option<String>,
    homepage: Option<String>,
    license: Option<String>,
    openclaw: bool,
    platforms: Vec<&'static str>,
    binaries: Vec<String>,
    environment: Vec<String>,
}

impl FrontMatter {
    fn parse(text: &str) -> Self {
        let mut header = Self::default();
        let mut lines = text.lines();
        if lines.next().map(str::trim_end) != Some("---") {
            return header;
        }
        let block: Vec<&str> = lines.take_while(|line| line.trim_end() != "---").collect();
        let mut metadata = String::new();
        let mut in_metadata = false;
        for line in &block {
            if line.starts_with([' ', '\t']) {
                if in_metadata {
                    metadata.push_str(line);
                    metadata.push('\n');
                }
                continue;
            }
            in_metadata = false;
            let Some((key, value)) = line.split_once(':') else { continue };
            let value = value.trim().trim_matches(['"', '\'']).to_owned();
            match key.trim() {
                "name" => header.name = Some(value),
                "description" => header.description = Some(value),
                "author" => header.author = Some(value),
                "homepage" => header.homepage = Some(value),
                "license" => header.license = Some(value),
                "metadata" => {
                    in_metadata = true;
                    metadata.push_str(line.split_once(':').map_or("", |(_, rest)| rest));
                    metadata.push('\n');
                }
                _ => {}
            }
        }
        header.read_metadata(&metadata);
        header
    }

    fn read_metadata(&mut self, metadata: &str) {
        let openclaw = serde_json::from_str::<Value>(metadata.trim()).ok().and_then(|value| value.get("openclaw").cloned());
        let Some(openclaw) = openclaw else {
            self.read_yaml_metadata(metadata);
            return;
        };
        self.openclaw = true;
        let strings = |value: Option<&Value>| -> Vec<String> {
            value.and_then(Value::as_array).map(|items| items.iter().filter_map(|item| item.as_str().map(str::to_owned)).collect()).unwrap_or_default()
        };
        self.platforms = platforms(&strings(openclaw.get("os")));
        let requires = openclaw.get("requires");
        self.binaries = strings(requires.and_then(|requires| requires.get("bins")));
        self.binaries.extend(strings(requires.and_then(|requires| requires.get("anyBins"))));
        self.environment = strings(requires.and_then(|requires| requires.get("env")));
    }

    /// Block-style YAML: the `openclaw:` key, and the `os`, `bins`, `anyBins` and `env` lists under it.
    fn read_yaml_metadata(&mut self, metadata: &str) {
        let mut current: Option<&str> = None;
        let mut os = Vec::new();
        for line in metadata.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("openclaw:") {
                self.openclaw = true;
                continue;
            }
            if !self.openclaw {
                continue;
            }
            if let Some(item) = trimmed.strip_prefix("- ") {
                let item = item.trim().trim_matches(['"', '\'']).to_owned();
                match current {
                    Some("os") => os.push(item),
                    Some("bins" | "anyBins") => self.binaries.push(item),
                    Some("env") => self.environment.push(item),
                    _ => {}
                }
                continue;
            }
            let Some((key, rest)) = trimmed.split_once(':') else { continue };
            current = ["os", "bins", "anyBins", "env"].into_iter().find(|candidate| *candidate == key);
            let inline: Vec<String> = rest
                .trim()
                .trim_start_matches('[')
                .trim_end_matches(']')
                .split(',')
                .map(|item| item.trim().trim_matches(['"', '\'']).to_owned())
                .filter(|item| !item.is_empty())
                .collect();
            match current {
                Some("os") => os.extend(inline),
                Some("bins" | "anyBins") => self.binaries.extend(inline),
                Some("env") => self.environment.extend(inline),
                _ => {}
            }
        }
        self.platforms = platforms(&os);
    }
}

fn platforms(os: &[String]) -> Vec<&'static str> {
    let mut found = Vec::new();
    for name in os {
        let platform = match name.as_str() {
            "darwin" | "macos" => "macos",
            "linux" => "linux",
            _ => continue,
        };
        if !found.contains(&platform) {
            found.push(platform);
        }
    }
    found
}
