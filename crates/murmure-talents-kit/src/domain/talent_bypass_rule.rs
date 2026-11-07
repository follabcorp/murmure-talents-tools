//! The guard against driving an app of the Mac that a Talent serves (docs/murmure/adaptateur-acp.md, « Garde-fou des
//! Talents »): pure and synchronous, on a bounded string. It lives in the contract crate so the Relay (permissions) and
//! the `PreToolUse` hook (`murmure-mcp talent-guard`) apply the same rule.

/// An app of the Mac a built-in Connector serves, as the rule needs it. The list is produced by `relay-connectors` and
/// given to the rule: the rule holds none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoveredApp {
    /// `mail`: the identifier of the Connector, the only thing the journal says.
    pub connector_id: String,
    /// `mes-mails`.
    pub talent_id: String,
    /// `Mes mails`: the name of the Talent as the message gives it.
    pub talent_name: String,
    /// `Mail`: the app as the message names it.
    pub display_name: String,
    /// The names of the app, French and English, compared without regard to case (`Calendrier`, `Calendar`).
    pub names: Vec<String>,
    /// `com.apple.mail`.
    pub bundle: String,
    /// What a direct access to the databases of the app looks like (`~/Library/Mail`, `group.com.apple.notes`).
    pub database_paths: Vec<String>,
}

/// The refusal: the fixed message for the Agent and the line of the journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TalentBypassRefusal {
    pub connector_id: String,
    pub message: String,
}

impl TalentBypassRefusal {
    /// The refusal for a command that drives `app`: the fixed message and the identifier for the journal.
    pub fn for_app(app: &CoveredApp) -> Self {
        Self {
            connector_id: app.connector_id.clone(),
            message: format!(
                "Murmure passe par le Talent « {} » pour {}. Utilise ses outils s'il est appris, sinon propose-le avec murmure_proposer_talent.",
                app.talent_name, app.display_name
            ),
        }
    }

    /// « Contournement de Talent refusé : <id du Connecteur> », never the command.
    pub fn journal_line(&self) -> String {
        format!("Contournement de Talent refusé : {}", self.connector_id)
    }
}

pub struct TalentBypassRule;

impl TalentBypassRule {
    /// The longest command the rule reads, as the Validations do; a longer one is not decided here.
    pub const MAX_COMMAND_BYTES: usize = 16 * 1024;

    /// `Some` when the command of an `execute` permission drives a covered app: `osascript` (AppleScript or JXA) that
    /// names the app or its bundle, or a direct access to its databases.
    pub fn check(command: &str, apps: &[CoveredApp]) -> Option<TalentBypassRefusal> {
        if command.len() > Self::MAX_COMMAND_BYTES {
            return None;
        }
        let lower = command.to_lowercase();
        let scripted = lower.contains("osascript");
        let targets = if scripted {
            script_targets(&lower)
        } else {
            Vec::new()
        };
        apps.iter()
            .find(|app| {
                targets.iter().any(|target| names_app(app, target))
                    || app
                        .database_paths
                        .iter()
                        .any(|path| touches_path(&lower, path))
            })
            .map(TalentBypassRefusal::for_app)
    }
}

fn names_app(app: &CoveredApp, target: &str) -> bool {
    std::iter::once(&app.bundle)
        .chain(&app.names)
        .any(|name| !name.is_empty() && name.to_lowercase() == target)
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// The names that follow `application`, `app` or `application id` (AppleScript `tell application "X"`, JXA
/// `Application('X')`) in a lowercased command, whatever the quoting (`'`, `"`, `\"`).
fn script_targets(lower: &str) -> Vec<String> {
    let mut targets = Vec::new();
    for keyword in ["application", "app"] {
        for (at, _) in lower.match_indices(keyword) {
            let before = lower[..at].chars().next_back();
            let rest = &lower[at + keyword.len()..];
            if before.is_some_and(is_word) || rest.chars().next().is_some_and(is_word) {
                continue;
            }
            let mut rest = rest.trim_start_matches(|c: char| c.is_whitespace() || c == '(');
            if let Some(after_id) = rest.strip_prefix("id") {
                if after_id
                    .starts_with(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == '\\')
                {
                    rest = after_id;
                }
            }
            let rest = rest.trim_start_matches(|c: char| {
                c.is_whitespace() || matches!(c, '"' | '\'' | '\\' | '(')
            });
            let name: String = rest
                .chars()
                .take_while(|c| !matches!(c, '"' | '\'' | '\\' | ')' | '\n'))
                .collect();
            if !name.is_empty() {
                targets.push(name.trim().to_owned());
            }
        }
    }
    targets
}

/// A path of the databases of an app: `~/Library/Mail` also matches `$HOME/Library/Mail` and `/Users/x/Library/Mail`;
/// a path without `~/` (`group.com.apple.notes`) matches as it is. The next character must not continue a name.
fn touches_path(lower: &str, path: &str) -> bool {
    let needle = path.trim_start_matches('~').to_lowercase();
    if needle.is_empty() {
        return false;
    }
    lower.match_indices(&needle).any(|(at, _)| {
        let end = at + needle.len();
        let after_ok = !lower[end..].chars().next().is_some_and(is_word);
        let before_ok = !path.starts_with('~') || at > 0;
        after_ok && before_ok
    })
}
