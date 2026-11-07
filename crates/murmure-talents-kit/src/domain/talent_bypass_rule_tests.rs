//! Contract: « Garde-fou : pas de pilotage direct d'une app couverte par un Talent ».

use super::{CoveredApp, TalentBypassRule};

fn app(
    connector: &str,
    talent: &str,
    talent_name: &str,
    shown: &str,
    names: &[&str],
    bundle: &str,
    paths: &[&str],
) -> CoveredApp {
    CoveredApp {
        connector_id: connector.to_owned(),
        talent_id: talent.to_owned(),
        talent_name: talent_name.to_owned(),
        display_name: shown.to_owned(),
        names: names.iter().map(|name| (*name).to_owned()).collect(),
        bundle: bundle.to_owned(),
        database_paths: paths.iter().map(|path| (*path).to_owned()).collect(),
    }
}

fn apps() -> Vec<CoveredApp> {
    vec![
        app(
            "mail",
            "mes-mails",
            "Mes mails",
            "Mail",
            &["Mail"],
            "com.apple.mail",
            &["~/Library/Mail"],
        ),
        app(
            "calendrier",
            "mon-calendrier",
            "Mon calendrier",
            "Calendrier",
            &["Calendrier", "Calendar"],
            "com.apple.iCal",
            &["~/Library/Calendars"],
        ),
        app(
            "rappels",
            "mes-rappels",
            "Mes rappels",
            "Rappels",
            &["Rappels", "Reminders"],
            "com.apple.reminders",
            &["group.com.apple.reminders"],
        ),
    ]
}

fn refused(command: &str) -> Option<String> {
    TalentBypassRule::check(command, &apps()).map(|refusal| refusal.connector_id)
}

#[test]
fn osascript_that_tells_a_covered_app_is_refused_by_name_or_by_bundle_in_either_language() {
    assert_eq!(
        refused(r#"osascript -e 'tell application "Mail" to get unread count of inbox'"#)
            .as_deref(),
        Some("mail")
    );
    assert_eq!(
        refused(r#"osascript -e 'tell application "mail" to activate'"#).as_deref(),
        Some("mail"),
        "case ignored"
    );
    assert_eq!(
        refused(r#"osascript -e 'tell application id "com.apple.reminders" to get name of lists'"#)
            .as_deref(),
        Some("rappels")
    );
    assert_eq!(
        refused(r#"osascript -e 'tell application "Calendrier" to get events'"#).as_deref(),
        Some("calendrier")
    );
    assert_eq!(
        refused(r#"osascript -e 'tell application "Calendar" to get events'"#).as_deref(),
        Some("calendrier")
    );
    assert_eq!(
        refused(r#"osascript -e 'set x to 1' -e 'tell application "Mail"' -e 'end tell'"#)
            .as_deref(),
        Some("mail"),
        "one -e among several"
    );
}

#[test]
fn a_javascript_for_automation_script_that_opens_a_covered_app_is_refused() {
    assert_eq!(
        refused(r#"osascript -l JavaScript -e "Application('Mail').inbox.messages.length""#)
            .as_deref(),
        Some("mail")
    );
}

#[test]
fn a_direct_access_to_the_databases_of_a_covered_app_is_refused() {
    assert_eq!(
        refused(r#"sqlite3 ~/Library/Mail/V10/MailData/"Envelope Index" 'select count(*) from messages'"#).as_deref(),
        Some("mail")
    );
    assert_eq!(
        refused("ls ~/Library/Group\\ Containers/group.com.apple.reminders/Container_v1")
            .as_deref(),
        Some("rappels")
    );
}

#[test]
fn the_refusal_has_a_fixed_message_and_a_journal_line_without_the_command() {
    let refusal =
        TalentBypassRule::check(r#"osascript -e 'tell application "Mail" to quit'"#, &apps())
            .expect("a refusal");
    assert_eq!(
        refusal.message,
        "Murmure passe par le Talent « Mes mails » pour Mail. Utilise ses outils s'il est appris, sinon propose-le avec murmure_proposer_talent."
    );
    let line = refusal.journal_line();
    assert_eq!(line, "Contournement de Talent refusé : mail");
    assert!(!line.contains("osascript"));
}

#[test]
fn other_commands_and_other_apps_are_left_to_the_usual_decision() {
    for command in [
        r#"osascript -e 'display notification "ok" with title "Murmure"'"#,
        "git status",
        r#"osascript -e 'tell application "Finder" to get name of home'"#,
        "echo Mail && git log --grep Mail",
    ] {
        assert_eq!(refused(command), None, "{command}");
    }
}

#[test]
fn a_command_over_16_kib_is_not_decided_by_the_rule() {
    let long = format!(
        "osascript -e 'tell application \"Mail\" to quit' # {}",
        "x".repeat(TalentBypassRule::MAX_COMMAND_BYTES)
    );
    assert!(long.len() > TalentBypassRule::MAX_COMMAND_BYTES);
    assert_eq!(refused(&long), None);
}
