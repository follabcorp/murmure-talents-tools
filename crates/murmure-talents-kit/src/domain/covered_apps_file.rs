//! The list of the covered apps as the Relay hands it to `murmure-mcp talent-guard`: a JSON file (no secret), written
//! by the Relay before the launch of the session and read by the hook. One codec, so both sides agree.

use super::CoveredApp;
use serde_json::{json, Value};

pub struct CoveredAppsFile;

impl CoveredAppsFile {
    pub fn encode(apps: &[CoveredApp]) -> String {
        Value::Array(
            apps.iter()
                .map(|app| {
                    json!({
                        "connectorId": app.connector_id,
                        "talentId": app.talent_id,
                        "talentName": app.talent_name,
                        "displayName": app.display_name,
                        "names": app.names,
                        "bundle": app.bundle,
                        "databasePaths": app.database_paths,
                    })
                })
                .collect(),
        )
        .to_string()
    }

    /// `None` when the text is not such a list: the hook then decides nothing.
    pub fn decode(text: &str) -> Option<Vec<CoveredApp>> {
        let list: Value = serde_json::from_str(text).ok()?;
        list.as_array()?
            .iter()
            .map(|entry| {
                let text = |key: &str| entry[key].as_str().map(str::to_owned);
                let texts = |key: &str| {
                    entry[key].as_array().map(|list| {
                        list.iter()
                            .filter_map(|item| item.as_str().map(str::to_owned))
                            .collect()
                    })
                };
                Some(CoveredApp {
                    connector_id: text("connectorId")?,
                    talent_id: text("talentId")?,
                    talent_name: text("talentName")?,
                    display_name: text("displayName")?,
                    names: texts("names")?,
                    bundle: text("bundle")?,
                    database_paths: texts("databasePaths")?,
                })
            })
            .collect()
    }
}
