//! `index.json` of the Catalogue, once read (docs/murmure/talents.md, « Catalogue »). Times are Unix seconds: the
//! domain never reads a clock.

use serde_json::{Map, Value};
use strict_json::parse_strict_json;

use super::iso_date::parse_iso;
use super::talent_manifest_rule::is_valid_identifier;
use super::{
    TalentFileVerification, TalentManifest, TalentManifestRule, TalentRevocationReason,
    TalentSource,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TalentIndexError {
    TooLarge,
    MalformedJson,
    UnknownKey,
    InvalidFormat,
    InvalidSignature,
    Expired,
    SequenceRollback,
    ReplacementKey,
    InvalidFiles,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TalentIndexFile {
    pub path: String,
    pub size: usize,
    /// 64 lowercase hexadecimal characters.
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TalentIndexRevocation {
    pub id: String,
    pub reason: TalentRevocationReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TalentIndexEntry {
    pub manifest: TalentManifest,
    /// The bytes whose SHA-256 the Relay keeps: the manifest with sorted keys, compact.
    pub manifest_json: Vec<u8>,
    pub files: Vec<TalentIndexFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TalentIndex {
    pub format: u32,
    pub sequence: u64,
    pub generated_at: i64,
    pub expires_at: i64,
    pub base_url: String,
    pub featured: Vec<String>,
    /// Only the entries this version understands: a manifest with a newer vocabulary hides itself, not the index.
    pub talents: Vec<TalentIndexEntry>,
    pub revoked: Vec<TalentIndexRevocation>,
}

type Failure = TalentIndexError;

const ENTRIES_MAX: usize = 2000;
const VALIDITY_SECONDS_MAX: i64 = 30 * 86_400;

impl TalentIndex {
    pub const MAXIMUM_BYTES: usize = 4 * 1024 * 1024;

    /// 4 MiB at most before any analysis, strict JSON, no unknown or missing key, `format` 1, a `sequence` that is an
    /// integer from 0, a validity that is positive and at most 30 days, at most 2 000 entries, `featured` of 0 to 3
    /// distinct identifiers present in the raw entries, unique identifiers, the reasons of revocation.
    pub fn parse(data: &[u8]) -> Result<Self, TalentIndexError> {
        if data.len() > Self::MAXIMUM_BYTES {
            return Err(Failure::TooLarge);
        }
        let value = parse_strict_json(data).map_err(|_| Failure::MalformedJson)?;
        let keys = [
            "format",
            "sequence",
            "generatedAt",
            "expiresAt",
            "baseURL",
            "featured",
            "talents",
            "revoked",
        ];
        let root = object(Some(&value), &keys)?;
        if natural(root.get("format"))? != 1 {
            return Err(Failure::InvalidFormat);
        }
        let sequence = natural(root.get("sequence"))?;
        let generated_at = date(root.get("generatedAt"))?;
        let expires_at = date(root.get("expiresAt"))?;
        let base_url = root
            .get("baseURL")
            .and_then(Value::as_str)
            .filter(|url| Self::is_safe_base_url(url))
            .ok_or(Failure::InvalidFormat)?;
        let featured = strings(root.get("featured"), 3)?;
        let entries = array(root.get("talents"), ENTRIES_MAX)?;
        let revocations = array(root.get("revoked"), ENTRIES_MAX)?;
        if expires_at <= generated_at
            || expires_at - generated_at > VALIDITY_SECONDS_MAX
            || distinct(&featured).is_none()
        {
            return Err(Failure::InvalidFormat);
        }

        let mut identifiers: Vec<String> = Vec::with_capacity(entries.len());
        let mut talents = Vec::new();
        for entry in entries {
            let entry = object(Some(entry), &["manifest", "files"])?;
            let manifest = entry
                .get("manifest")
                .and_then(Value::as_object)
                .ok_or(Failure::InvalidFormat)?;
            let id = manifest
                .get("id")
                .and_then(Value::as_str)
                .filter(|id| is_valid_identifier(id))
                .ok_or(Failure::InvalidFormat)?;
            if identifiers.iter().any(|known| known == id) {
                return Err(Failure::InvalidFormat);
            }
            identifiers.push(id.to_owned());
            let files = files(entry.get("files"))?;
            TalentFileVerification::validate(&files)?;
            let manifest_json = serde_json::to_vec(manifest).map_err(|_| Failure::InvalidFormat)?;
            // A newer manifest vocabulary only hides this entry; the signed envelope stays useful.
            if let Ok(manifest) = TalentManifestRule::parse(&manifest_json, TalentSource::Catalog) {
                talents.push(TalentIndexEntry {
                    manifest,
                    manifest_json,
                    files,
                });
            }
        }
        if !featured.iter().all(|id| identifiers.contains(id)) {
            return Err(Failure::InvalidFormat);
        }

        let mut revoked: Vec<TalentIndexRevocation> = Vec::with_capacity(revocations.len());
        for revocation in revocations {
            let revocation = object(Some(revocation), &["id", "reason"])?;
            let id = revocation
                .get("id")
                .and_then(Value::as_str)
                .filter(|id| is_valid_identifier(id))
                .ok_or(Failure::InvalidFormat)?;
            let reason = revocation
                .get("reason")
                .and_then(Value::as_str)
                .and_then(TalentRevocationReason::from_wire)
                .ok_or(Failure::InvalidFormat)?;
            if revoked.iter().any(|known| known.id == id) {
                return Err(Failure::InvalidFormat);
            }
            revoked.push(TalentIndexRevocation {
                id: id.to_owned(),
                reason,
            });
        }
        Ok(Self {
            format: 1,
            sequence,
            generated_at,
            expires_at,
            base_url: base_url.to_owned(),
            featured,
            talents,
            revoked,
        })
    }

    /// HTTPS, or HTTP on a loopback host; no credentials, query or fragment.
    pub fn is_safe_base_url(url: &str) -> bool {
        let Ok(parsed) = url::Url::parse(url) else {
            return false;
        };
        if !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
        {
            return false;
        }
        let Some(host) = parsed.host_str().filter(|host| !host.is_empty()) else {
            return false;
        };
        match parsed.scheme() {
            "https" => true,
            "http" => matches!(host, "127.0.0.1" | "localhost" | "[::1]"),
            _ => false,
        }
    }
}

fn object<'a>(value: Option<&'a Value>, keys: &[&str]) -> Result<&'a Map<String, Value>, Failure> {
    let object = value
        .and_then(Value::as_object)
        .ok_or(Failure::InvalidFormat)?;
    if !object.keys().all(|key| keys.contains(&key.as_str())) {
        return Err(Failure::UnknownKey);
    }
    if object.len() == keys.len() {
        Ok(object)
    } else {
        Err(Failure::InvalidFormat)
    }
}

fn natural(value: Option<&Value>) -> Result<u64, Failure> {
    value.and_then(Value::as_u64).ok_or(Failure::InvalidFormat)
}

fn date(value: Option<&Value>) -> Result<i64, Failure> {
    value
        .and_then(Value::as_str)
        .and_then(parse_iso)
        .ok_or(Failure::InvalidFormat)
}

fn array(value: Option<&Value>, maximum: usize) -> Result<&Vec<Value>, Failure> {
    value
        .and_then(Value::as_array)
        .filter(|items| items.len() <= maximum)
        .ok_or(Failure::InvalidFormat)
}

fn strings(value: Option<&Value>, maximum: usize) -> Result<Vec<String>, Failure> {
    array(value, maximum)?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .ok_or(Failure::InvalidFormat)
        })
        .collect()
}

fn distinct(values: &[String]) -> Option<()> {
    values
        .iter()
        .enumerate()
        .all(|(position, value)| !values[..position].contains(value))
        .then_some(())
}

fn files(value: Option<&Value>) -> Result<Vec<TalentIndexFile>, Failure> {
    let items = value
        .and_then(Value::as_array)
        .ok_or(Failure::InvalidFormat)?;
    items
        .iter()
        .map(|item| {
            let file = object(Some(item), &["path", "size", "sha256"])?;
            let path = file
                .get("path")
                .and_then(Value::as_str)
                .ok_or(Failure::InvalidFiles)?;
            let sha256 = file
                .get("sha256")
                .and_then(Value::as_str)
                .ok_or(Failure::InvalidFiles)?;
            let size =
                usize::try_from(natural(file.get("size"))?).map_err(|_| Failure::InvalidFormat)?;
            Ok(TalentIndexFile {
                path: path.to_owned(),
                size,
                sha256: sha256.to_owned(),
            })
        })
        .collect()
}
