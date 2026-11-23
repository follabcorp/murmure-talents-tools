//! The files of a Talent of the Catalogue (docs/murmure/talents.md, « Catalogue », « Pour apprendre… »).

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

use super::talent_manifest_rule::is_valid_identifier;
use super::{TalentIndexError, TalentIndexFile, TalentKind, TalentManifest, TalentOperationError};

const FILES_MAX: usize = 32;
const BYTES_MAX: usize = 256 * 1024;
const SEGMENTS_MAX: usize = 8;
const PATH_BYTES_MAX: usize = 1024;

pub struct TalentFileVerification;

impl TalentFileVerification {
    /// 1 to 32 distinct files, 256 KiB in all, 8 segments per path at most; no absolute, empty, `.`, `..`,
    /// backslash, `%`, `?`, `#`, `:` or control character in a path; `SKILL.md` or `skills/<name>/SKILL.md`
    /// present; the size is not negative and the SHA-256 is 64 lowercase hexadecimal characters.
    pub fn validate(files: &[TalentIndexFile]) -> Result<(), TalentIndexError> {
        let distinct = files.iter().enumerate().all(|(position, file)| {
            files[..position]
                .iter()
                .all(|other| other.path != file.path)
        });
        let has_skill = files.iter().any(|file| {
            file.path == "SKILL.md"
                || (file.path.starts_with("skills/") && file.path.ends_with("/SKILL.md"))
        });
        if !(1..=FILES_MAX).contains(&files.len()) || !distinct || !has_skill {
            return Err(TalentIndexError::InvalidFiles);
        }
        let mut total = 0usize;
        for file in files {
            if !is_valid_path(&file.path)
                || file.size > BYTES_MAX - total
                || !is_sha256(&file.sha256)
            {
                return Err(TalentIndexError::InvalidFiles);
            }
            total += file.size;
        }
        Ok(())
    }

    /// What was downloaded is exactly what the index lists: same paths, same sizes, same SHA-256. One error refuses
    /// the whole batch.
    pub fn verify(
        files: &[TalentIndexFile],
        downloaded: &BTreeMap<String, Vec<u8>>,
    ) -> Result<(), TalentIndexError> {
        Self::validate(files)?;
        if downloaded.len() != files.len() {
            return Err(TalentIndexError::InvalidFiles);
        }
        for file in files {
            let matches = downloaded.get(&file.path).is_some_and(|data| {
                data.len() == file.size && hex(&Sha256::digest(data)) == file.sha256
            });
            if !matches {
                return Err(TalentIndexError::InvalidFiles);
            }
        }
        Ok(())
    }
}

fn is_valid_path(path: &str) -> bool {
    let segments: Vec<&str> = path.split('/').collect();
    path.len() <= PATH_BYTES_MAX
        && segments.len() <= SEGMENTS_MAX
        && segments
            .iter()
            .all(|segment| !segment.is_empty() && *segment != "." && *segment != "..")
        && !path.chars().any(|character| {
            character.is_control() || matches!(character, '\\' | '%' | '?' | '#' | ':')
        })
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The files of one Skill: relative path and bytes.
pub type SkillFiles = Vec<(String, Vec<u8>)>;

pub struct TalentBundleRule;

impl TalentBundleRule {
    /// Groups the verified files by Skill: a `SKILL.md` at the root is the single Skill that has the name of the
    /// Talent; otherwise each `skills/<name>/…` is one Skill (the name is a valid name). Every file is strict UTF-8,
    /// annexes included, and the Talent is textual: anything else is `InvalidResources`.
    pub fn group_skills(
        manifest: &TalentManifest,
        downloaded: &BTreeMap<String, Vec<u8>>,
    ) -> Result<BTreeMap<String, SkillFiles>, TalentOperationError> {
        let invalid = TalentOperationError::InvalidResources;
        if manifest.kind == TalentKind::WithCode
            || downloaded
                .values()
                .any(|data| std::str::from_utf8(data).is_err())
        {
            return Err(invalid);
        }
        let single = downloaded.contains_key("SKILL.md");
        let mut groups: BTreeMap<String, SkillFiles> = BTreeMap::new();
        for (path, data) in downloaded {
            let (name, relative) = if single {
                (manifest.id.clone(), path.clone())
            } else {
                let parts: Vec<&str> = path.split('/').collect();
                if parts.len() < 3 || parts[0] != "skills" || !is_valid_identifier(parts[1]) {
                    return Err(invalid);
                }
                (parts[1].to_owned(), parts[2..].join("/"))
            };
            groups
                .entry(name)
                .or_default()
                .push((relative, data.clone()));
        }
        Ok(groups)
    }
}
