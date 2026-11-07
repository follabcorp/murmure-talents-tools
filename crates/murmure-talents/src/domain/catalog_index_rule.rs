use std::collections::BTreeMap;

use serde_json::{json, Value};
use strict_json::parse_strict_json;
use murmure_talents_kit::{TalentIndex, TalentIndexRevocation, TalentManifest, TalentManifestRule, TalentSource};

use super::{CatalogError, CatalogSource, CatalogValidationRule};

const VALIDITY_SECONDS: i64 = 14 * 86_400;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogIndexArtifact {
    /// `index.json`: sorted keys, compact, slashes unescaped, readable by `TalentIndex::parse`.
    pub index_data: Vec<u8>,
    /// `<id>/<version>/<path>` → bytes.
    pub files: BTreeMap<String, Vec<u8>>,
}

pub struct CatalogIndexRule;

impl CatalogIndexRule {
    /// Refuses an invalid catalog, an unsafe base URL or a negative-looking date; otherwise the index (format 1,
    /// `generatedAt` = `now`, `expiresAt` = `now` + 14 days, Talents sorted by folder) and the versioned files.
    pub fn build(
        talents: &[CatalogSource],
        featured: &[String],
        revoked: &[TalentIndexRevocation],
        base_url: &str,
        sequence: u64,
        now: i64,
        built_in: &[TalentManifest],
    ) -> Result<CatalogIndexArtifact, CatalogError> {
        let report = CatalogValidationRule::validate(talents, built_in);
        if !report.is_valid() {
            return Err(CatalogError::new(report.text()));
        }
        if !TalentIndex::is_safe_base_url(base_url) || now < 0 {
            return Err(CatalogError::new("URL de base, séquence ou date invalide."));
        }
        let mut sorted: Vec<&CatalogSource> = talents.iter().collect();
        sorted.sort_by(|left, right| left.directory_id.cmp(&right.directory_id));
        let mut files = BTreeMap::new();
        let mut entries = Vec::new();
        for source in sorted {
            let manifest = TalentManifestRule::parse(&source.manifest_json, TalentSource::Catalog)
                .map_err(|_| CatalogError::new("Manifeste invalide."))?;
            let version = format!("{}.{}.{}", manifest.version.major, manifest.version.minor, manifest.version.patch);
            let mut sorted_files = source.files.clone();
            sorted_files.sort_by(|left, right| left.path.cmp(&right.path));
            for file in &source.files {
                files.insert(format!("{}/{version}/{}", manifest.id, file.path), file.data.clone());
            }
            let listed: Vec<Value> = CatalogValidationRule::file_descriptors(&sorted_files)
                .into_iter()
                .map(|file| json!({"path": file.path, "size": file.size, "sha256": file.sha256}))
                .collect();
            let manifest_value = parse_strict_json(&source.manifest_json)
                .map_err(|_| CatalogError::new("Manifeste invalide."))?;
            entries.push(json!({"manifest": manifest_value, "files": listed}));
        }
        let revocations: Vec<Value> =
            revoked.iter().map(|revocation| json!({"id": revocation.id, "reason": revocation.reason.wire()})).collect();
        let value = json!({
            "format": 1, "sequence": sequence, "generatedAt": iso_date(now), "expiresAt": iso_date(now + VALIDITY_SECONDS),
            "baseURL": base_url, "featured": featured, "talents": entries, "revoked": revocations
        });
        let index_data = canonical(&value).into_bytes();
        TalentIndex::parse(&index_data).map_err(|_| CatalogError::new("Index invalide."))?;
        Ok(CatalogIndexArtifact { index_data, files })
    }
}

/// Compact JSON with the keys of every object sorted, whatever the order the map keeps.
fn canonical(value: &Value) -> String {
    match value {
        Value::Array(items) => format!("[{}]", items.iter().map(canonical).collect::<Vec<_>>().join(",")),
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let members: Vec<String> =
                keys.into_iter().map(|key| format!("{}:{}", Value::String(key.clone()), canonical(&map[key]))).collect();
            format!("{{{}}}", members.join(","))
        }
        other => other.to_string(),
    }
}

/// `2026-11-02T00:00:00Z` from Unix seconds (proleptic Gregorian calendar).
pub(super) fn iso_date(seconds: i64) -> String {
    let (days, rest) = (seconds.div_euclid(86_400), seconds.rem_euclid(86_400));
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_part + 2) / 5 + 1;
    let month = if month_part < 10 { month_part + 3 } else { month_part - 9 };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z", rest / 3600, rest % 3600 / 60, rest % 60)
}
