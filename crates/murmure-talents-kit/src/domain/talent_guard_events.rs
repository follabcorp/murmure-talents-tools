//! The events file of the guard of the Talents: the hook (`murmure-mcp talent-guard --events`) appends the identifier
//! of the Connector it refused, the Relay drains it at the end of the Run and writes the journal line. One codec, so
//! both sides agree. Pure: nothing here reads or writes a file.

use super::{CoveredApp, TalentBypassRefusal};

pub struct TalentGuardEvents;

impl TalentGuardEvents {
    /// The most of lines a drain keeps: the Agent can write in the file, so what it can falsify is bounded.
    pub const MAX_LINES: usize = 32;
    /// The file is no longer appended to once it holds this much.
    pub const MAX_FILE_BYTES: u64 = 4 * 1024;

    /// `<connector_id>\n`: never the command.
    pub fn encode(connector_id: &str) -> String {
        format!("{connector_id}\n")
    }

    /// The refusals the text records: only the lines equal to the identifier of a known covered app, at most
    /// `MAX_LINES`; anything else (empty, too long, NUL, CRLF, unknown) is ignored.
    pub fn decode(text: &str, apps: &[CoveredApp]) -> Vec<TalentBypassRefusal> {
        text.split('\n')
            .filter_map(|line| apps.iter().find(|app| app.connector_id == line))
            .take(Self::MAX_LINES)
            .map(TalentBypassRefusal::for_app)
            .collect()
    }
}
