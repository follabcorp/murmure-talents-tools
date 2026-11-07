//! The folder a Talent « Créé par toi » leaves in for the Catalogue (docs/murmure/talents.md, « Exporter pour le
//! Catalogue »): `talent.json` in the form of the Catalogue, `SKILL.md` and its attached files at the root, and the
//! MIT `LICENSE`. In the Kit because the Relay builds it and the Catalogue tool checks it (ADR-0024).

use std::collections::BTreeMap;

use serde_json::{json, Value};
use unicode_segmentation::UnicodeSegmentation;

use super::{LocalizedText, SemanticVersion, SkillFiles, TalentManifest, TalentNeed};

const AUTHOR_CHARACTERS_MAX: usize = 64;
const NAME_GRAPHEMES_MAX: usize = 40;
const SUMMARY_GRAPHEMES_MAX: usize = 140;

/// One file of the exported folder: a path relative to `<id>/`, UTF-8 text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TalentExportFile {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TalentExportError {
    /// The name of the Author is not 1 to 64 characters after trimming, or carries a control character.
    InvalidAuthorName,
    /// A file of the Skill is not UTF-8 text.
    NotText,
    /// Two files of the folder share a path once the case is folded (a Skill with `license` next to `LICENSE`): a
    /// case-insensitive disk would keep one of them.
    PathCollision,
}

pub struct TalentExportProjection;

impl TalentExportProjection {
    /// The name of the Author when the app gives none.
    pub const DEFAULT_AUTHOR: &'static str = "Murmure";

    /// The trimmed name: 1 to 64 characters, no control character, line or paragraph separator, nor bidirectional
    /// control (which would reorder the copyright line).
    pub fn author_name(raw: &str) -> Result<String, TalentExportError> {
        let name = raw.trim();
        let count = name.chars().count();
        if (1..=AUTHOR_CHARACTERS_MAX).contains(&count) && !name.chars().any(is_refused_in_name) {
            Ok(name.to_owned())
        } else {
            Err(TalentExportError::InvalidAuthorName)
        }
    }

    /// The files, sorted by path: `LICENSE`, `SKILL.md`, the attached files, `talent.json`. `manifest` is the one the
    /// Talent has, if any (version, names, needs, capabilities and requirements are kept; a Talent made of a lone
    /// Skill gets version `1.0.0`, its identifier as name and the description of its `SKILL.md` as summary).
    /// The Catalogue wants French and English: a missing language is a copy of the other, for the Author to translate.
    /// `author.github` is empty, to be completed; the license is MIT, with `year` and the Author in `LICENSE`.
    pub fn folder(
        id: &str,
        manifest: Option<&TalentManifest>,
        skill: &SkillFiles,
        author: &str,
        year: i32,
    ) -> Result<Vec<TalentExportFile>, TalentExportError> {
        let author = Self::author_name(author)?;
        let mut files = Vec::with_capacity(skill.len() + 2);
        for (path, data) in skill {
            let content =
                String::from_utf8(data.clone()).map_err(|_| TalentExportError::NotText)?;
            files.push(TalentExportFile {
                path: path.clone(),
                content,
            });
        }
        files.push(TalentExportFile {
            path: "LICENSE".to_owned(),
            content: mit_license(year, &author),
        });
        files.push(TalentExportFile {
            path: "talent.json".to_owned(),
            content: manifest_text(id, manifest, skill, &author),
        });
        let mut folded = std::collections::BTreeSet::new();
        if !files
            .iter()
            .all(|file| folded.insert(file.path.to_lowercase()))
        {
            return Err(TalentExportError::PathCollision);
        }
        files.sort_by(|left, right| left.path.cmp(&right.path));
        Ok(files)
    }
}

fn is_refused_in_name(character: char) -> bool {
    character.is_control()
        || matches!(character, '\u{2028}' | '\u{2029}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
}

fn manifest_text(
    id: &str,
    manifest: Option<&TalentManifest>,
    skill: &SkillFiles,
    author: &str,
) -> String {
    let localized = localized_texts(id, manifest, skill);
    let wire_text = |text: &LocalizedText| json!({"name": text.name, "summary": text.summary});
    let value = json!({
        "id": id,
        "version": manifest.map_or_else(|| SemanticVersion::new(1, 0, 0), |m| m.version).to_string(),
        "kind": "textOnly",
        "localized": {"fr": wire_text(&localized.0), "en": wire_text(&localized.1)},
        "needs": manifest.map_or_else(|| vec![TalentNeed::Organizing.wire()], |m| m.needs.iter().map(|need| need.wire()).collect()),
        "capabilities": manifest.map_or_else(Vec::new, |m| m.capabilities.iter().map(|capability| capability.wire()).collect()),
        "requires": manifest.map_or_else(Vec::new, |m| m.requires.clone()),
        "minimumMurmureVersion": manifest.map_or_else(|| SemanticVersion::new(1, 0, 0), |m| m.minimum_murmure_version).to_string(),
        "author": {"name": author, "github": Vec::<String>::new()},
        "license": "MIT",
    });
    serde_json::to_string_pretty(&value)
        .map_or_else(|_| Value::Null.to_string(), |text| text + "\n")
}

/// French then English: each taken from the manifest, else copied from the other, else made of the Skill.
fn localized_texts(
    id: &str,
    manifest: Option<&TalentManifest>,
    skill: &SkillFiles,
) -> (LocalizedText, LocalizedText) {
    let empty = BTreeMap::new();
    let known = manifest.map_or(&empty, |m| &m.localized);
    let made = || LocalizedText {
        name: truncated(id, NAME_GRAPHEMES_MAX),
        summary: truncated(
            &skill_description(skill).unwrap_or_else(|| id.to_owned()),
            SUMMARY_GRAPHEMES_MAX,
        ),
    };
    match (known.get("fr"), known.get("en")) {
        (Some(french), Some(english)) => (french.clone(), english.clone()),
        (Some(only), None) | (None, Some(only)) => (only.clone(), only.clone()),
        (None, None) => (made(), made()),
    }
}

fn truncated(text: &str, limit: usize) -> String {
    text.trim().graphemes(true).take(limit).collect()
}

/// The `description` of the front matter of `SKILL.md`, on one line, without quotes around it.
fn skill_description(skill: &SkillFiles) -> Option<String> {
    let (_, data) = skill.iter().find(|(path, _)| path == "SKILL.md")?;
    let text = std::str::from_utf8(data).ok()?;
    let mut lines = text.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }
    lines
        .take_while(|line| line.trim() != "---")
        .find_map(|line| line.strip_prefix("description:"))
        .map(|value| {
            value
                .trim()
                .trim_matches(|c| c == '"' || c == '\'')
                .trim()
                .to_owned()
        })
        .filter(|value| !value.is_empty())
}

fn mit_license(year: i32, author: &str) -> String {
    format!(
        "MIT License

Copyright (c) {year} {author}

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the \"Software\"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
"
    )
}
