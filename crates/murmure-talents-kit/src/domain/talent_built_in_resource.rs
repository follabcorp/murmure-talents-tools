//! The manifests « Par Murmure », shipped with the Relay (docs/murmure/talents.md, « Talents "Par Murmure" »).

use serde_json::Value;
use strict_json::parse_strict_json;

use super::talent_manifest_rule::is_valid_identifier;
use super::{TalentManifest, TalentManifestError, TalentManifestRule, TalentSource};

/// The manifests shipped with the Relay, one file each in `resources/built_in/`.
pub const BUILT_IN_MANIFESTS: [&str; 35] = [
    include_str!("../../resources/built_in/mes-mails.json"),
    include_str!("../../resources/built_in/mes-notes.json"),
    include_str!("../../resources/built_in/mes-rappels.json"),
    include_str!("../../resources/built_in/mon-calendrier.json"),
    include_str!("../../resources/built_in/naviguer-sur-le-web.json"),
    include_str!("../../resources/built_in/chercher-sur-le-web.json"),
    include_str!("../../resources/built_in/lire-le-web.json"),
    include_str!("../../resources/built_in/ma-boite-mail.json"),
    include_str!("../../resources/built_in/mes-contacts.json"),
    include_str!("../../resources/built_in/mes-contacts-carddav.json"),
    include_str!("../../resources/built_in/mes-messages.json"),
    include_str!("../../resources/built_in/mon-agenda-en-ligne.json"),
    include_str!("../../resources/built_in/obsidian.json"),
    include_str!("../../resources/built_in/trello.json"),
    include_str!("../../resources/built_in/asana.json"),
    include_str!("../../resources/built_in/clickup.json"),
    include_str!("../../resources/built_in/todoist.json"),
    include_str!("../../resources/built_in/mes-documents.json"),
    include_str!("../../resources/built_in/ma-maison.json"),
    include_str!("../../resources/built_in/notion.json"),
    include_str!("../../resources/built_in/google-drive.json"),
    include_str!("../../resources/built_in/microsoft-365.json"),
    include_str!("../../resources/built_in/things.json"),
    include_str!("../../resources/built_in/telegram.json"),
    include_str!("../../resources/built_in/slack.json"),
    include_str!("../../resources/built_in/discord.json"),
    include_str!("../../resources/built_in/bear.json"),
    include_str!("../../resources/built_in/lumieres-hue.json"),
    include_str!("../../resources/built_in/lumieres-govee.json"),
    include_str!("../../resources/built_in/spotify.json"),
    include_str!("../../resources/built_in/musique.json"),
    include_str!("../../resources/built_in/mes-trajets.json"),
    include_str!("../../resources/built_in/mes-photos.json"),
    include_str!("../../resources/built_in/ma-sante.json"),
    include_str!("../../resources/built_in/budget.json"),
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

    /// The Connector identifiers of the embedded resources, sorted and distinct: what an Agent Talent may request.
    pub fn embedded_connectors() -> Vec<String> {
        let mut connectors: Vec<String> = BUILT_IN_MANIFESTS
            .iter()
            .filter_map(|data| Self::parse(data.as_bytes()).ok())
            .map(|resource| resource.connector)
            .collect();
        connectors.sort();
        connectors.dedup();
        connectors
    }

    /// A built-in Talent is available when the definition of its Connector exists in the registry, even disabled.
    pub fn is_available(connector: &str, definitions: &[String]) -> bool {
        definitions.iter().any(|definition| definition == connector)
    }
}
