//! Contract: what the hook appends to the events file is what the Relay reads back, and nothing else is read.

use super::{CoveredApp, TalentGuardEvents};

fn app(connector: &str) -> CoveredApp {
    CoveredApp {
        connector_id: connector.to_owned(),
        talent_id: format!("talent-{connector}"),
        talent_name: "Mes mails".to_owned(),
        display_name: "Mail".to_owned(),
        names: vec!["Mail".to_owned()],
        bundle: "com.apple.mail".to_owned(),
        database_paths: vec![],
    }
}

#[test]
fn a_line_is_the_connector_identifier_alone() {
    assert_eq!(TalentGuardEvents::encode("mail"), "mail\n");
}

#[test]
fn a_known_identifier_is_kept_and_an_unknown_one_is_dropped() {
    let refusals = TalentGuardEvents::decode("mail\nnotes\n", &[app("mail")]);
    assert_eq!(refusals.len(), 1);
    assert_eq!(
        refusals[0].journal_line(),
        "Contournement de Talent refusé : mail"
    );
}

#[test]
fn a_drain_is_capped_at_thirty_two_lines() {
    let text = TalentGuardEvents::encode("mail").repeat(40);
    assert_eq!(
        TalentGuardEvents::decode(&text, &[app("mail")]).len(),
        TalentGuardEvents::MAX_LINES
    );
    assert_eq!(TalentGuardEvents::MAX_LINES, 32);
}

#[test]
fn empty_long_nul_and_crlf_lines_are_ignored() {
    let long = "m".repeat(10_000);
    let text = format!("\n{long}\nma\0il\nmail\r\n\nmail\n");
    let refusals = TalentGuardEvents::decode(&text, &[app("mail")]);
    assert_eq!(refusals.len(), 1, "only the clean line counts");
}
