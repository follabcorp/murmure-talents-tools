//! Contract: what the Relay writes for the hook is what the hook reads back, and nothing else is read.

use super::{CoveredApp, CoveredAppsFile};

#[test]
fn a_list_is_read_back_as_it_was_written_and_anything_else_is_not_a_list() {
    let apps = vec![CoveredApp {
        connector_id: "mail".to_owned(),
        talent_id: "mes-mails".to_owned(),
        talent_name: "Mes mails".to_owned(),
        display_name: "Mail".to_owned(),
        names: vec!["Mail".to_owned()],
        bundle: "com.apple.mail".to_owned(),
        database_paths: vec!["~/Library/Mail".to_owned()],
    }];
    assert_eq!(
        CoveredAppsFile::decode(&CoveredAppsFile::encode(&apps)),
        Some(apps)
    );
    assert_eq!(CoveredAppsFile::decode("{}"), None);
    assert_eq!(
        CoveredAppsFile::decode(r#"[{"connectorId": "mail"}]"#),
        None
    );
    assert_eq!(CoveredAppsFile::decode("not json"), None);
}
