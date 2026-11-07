//! The manifests « Par Murmure », shipped with the Relay (docs/murmure/talents.md, « Talents "Par Murmure" »).

use serde_json::Value;
use strict_json::parse_strict_json;

use super::talent_manifest_rule::is_valid_identifier;
use super::{TalentManifest, TalentManifestError, TalentManifestRule, TalentSource};

/// The manifests shipped with the Relay, one file each in `resources/built_in/`.
pub const BUILT_IN_MANIFESTS: [&str; 5] = [
    include_str!("../../resources/built_in/mes-mails.json"),
    include_str!("../../resources/built_in/mes-notes.json"),
    include_str!("../../resources/built_in/mes-rappels.json"),
    include_str!("../../resources/built_in/mon-calendrier.json"),
    include_str!("../../resources/built_in/naviguer-sur-le-web.json"),
];

/// `{"connector":"<id>","manifest":{…}}`: the manifest with code of a Talent and the Connector it drives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TalentBuiltInResource {
    pub connector: String,
    pub manifest: TalentManifest,
    /// The manifest with sorted keys, compact, slashes unescaped.
    pub manifest_json: Vec<u8>,
}

impl TalentBuiltInResource {
    pub const MAXIMUM_BYTES: usize = 20_480;

    /// 20 KiB at most, strict JSON, exactly the keys `connector` (a valid name) and `manifest`, the manifest read as
    /// an embedded one (`withCode` allowed).
    pub fn parse(data: &[u8]) -> Result<Self, TalentManifestError> {
        if data.len() > Self::MAXIMUM_BYTES {
            return Err(TalentManifestError::TooLarge);
        }
        let envelope_error = || TalentManifestError::InvalidField("envelope".to_owned());
        let value = parse_strict_json(data).map_err(|_| TalentManifestError::MalformedJson)?;
        let object = value
            .as_object()
            .filter(|object| object.len() == 2)
            .ok_or_else(envelope_error)?;
        let connector = object
            .get("connector")
            .and_then(Value::as_str)
            .filter(|name| is_valid_identifier(name))
            .ok_or_else(envelope_error)?;
        let manifest_value = object
            .get("manifest")
            .filter(|manifest| manifest.is_object())
            .ok_or_else(envelope_error)?;
        let manifest_json = serde_json::to_vec(manifest_value).map_err(|_| envelope_error())?;
        let manifest = TalentManifestRule::parse(&manifest_json, TalentSource::Embedded)?;
        Ok(Self {
            connector: connector.to_owned(),
            manifest,
            manifest_json,
        })
    }

    /// A built-in Talent is available when the definition of its Connector exists in the registry, even disabled.
    pub fn is_available(connector: &str, definitions: &[String]) -> bool {
        definitions.iter().any(|definition| definition == connector)
    }
}
