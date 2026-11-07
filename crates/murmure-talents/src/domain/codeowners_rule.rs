//! The `CODEOWNERS` of the Catalogue repository, derived from the Authors of the manifests (docs/murmure/catalogue-talents.md).

use std::collections::BTreeSet;

/// The Maintainers' team, owner of everything.
pub const TEAM: &str = "@follabcorp/murmure";

/// One Talent of the Catalogue: its folder name and the GitHub accounts of its Authors, without `@`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeownersEntry {
    pub talent_id: String,
    pub github: Vec<String>,
}

pub struct CodeownersRule;

impl CodeownersRule {
    /// The expected file: `* <team>`, then one `/talents/<id>/ <team> @handle…` line per Talent, then `/.github/`,
    /// `/featured.json` and `/revoked.json` to the team alone; one `\n` ends each line. Talents sorted by id and
    /// handles sorted without duplicates, whatever the input order.
    pub fn expected(entries: &[CodeownersEntry]) -> String {
        let mut talents: Vec<&CodeownersEntry> = entries.iter().collect();
        talents.sort_by(|left, right| left.talent_id.cmp(&right.talent_id));
        let mut lines = vec![format!("* {TEAM}")];
        for entry in talents {
            let handles: BTreeSet<&str> = entry.github.iter().map(String::as_str).collect();
            let owners: String = handles.into_iter().map(|handle| format!(" @{handle}")).collect();
            lines.push(format!("/talents/{}/ {TEAM}{owners}", entry.talent_id));
        }
        lines.extend(["/.github/", "/featured.json", "/revoked.json"].map(|path| format!("{path} {TEAM}")));
        lines.join("\n") + "\n"
    }

    /// True when the present file is exactly the expected one.
    pub fn is_current(expected: &str, present: &str) -> bool {
        expected == present
    }
}
