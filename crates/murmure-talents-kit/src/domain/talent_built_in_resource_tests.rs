//! Contract: « Talents "Par Murmure" ».

use serde_json::json;

use super::fixtures::manifest_json;
use super::{TalentBuiltInResource, TalentKind, TalentManifestError, BUILT_IN_MANIFESTS};

fn envelope(connector: &str) -> serde_json::Value {
    let mut manifest = manifest_json();
    manifest["kind"] = json!("withCode");
    manifest["requires"] = json!([]);
    json!({"connector": connector, "manifest": manifest})
}

#[test]
fn an_envelope_holds_a_connector_and_a_manifest_with_code_read_as_embedded() {
    let resource = TalentBuiltInResource::parse(envelope("chrome").to_string().as_bytes())
        .expect("a built-in Talent");
    assert_eq!(
        (resource.connector.as_str(), resource.manifest.kind),
        ("chrome", TalentKind::WithCode)
    );
    let text = String::from_utf8(resource.manifest_json).expect("UTF-8");
    assert!(
        text.starts_with(r#"{"author":"#)
            && text.contains("https://example.org")
            && !text.contains(": "),
        "sorted, compact, slashes unescaped: {text}"
    );
}

#[test]
fn an_envelope_with_another_set_of_keys_or_a_bad_connector_or_too_big_is_refused() {
    let mut extra = envelope("chrome");
    extra["extra"] = json!(1);
    let mut missing = envelope("chrome");
    missing
        .as_object_mut()
        .expect("an object")
        .remove("connector");
    for refused in [extra, missing, envelope("Chrome_Mail")] {
        assert_eq!(
            TalentBuiltInResource::parse(refused.to_string().as_bytes()),
            Err(TalentManifestError::InvalidField("envelope".into())),
            "{refused}"
        );
    }
    assert!(
        TalentBuiltInResource::parse(br#"{"connector":"a","connector":"a"}"#).is_err(),
        "duplicate key"
    );
    assert_eq!(
        TalentBuiltInResource::parse(&vec![b' '; TalentBuiltInResource::MAXIMUM_BYTES + 1]),
        Err(TalentManifestError::TooLarge)
    );
}

#[test]
fn a_built_in_talent_is_available_when_its_connector_is_defined() {
    let definitions = vec!["chrome".to_owned()];
    assert!(TalentBuiltInResource::is_available("chrome", &definitions));
    assert!(!TalentBuiltInResource::is_available("mail", &definitions));
}

fn my_mail() -> TalentBuiltInResource {
    let text = BUILT_IN_MANIFESTS
        .iter()
        .find(|text| text.contains(r#""id": "mes-mails""#))
        .expect("the built-in Talent mes-mails");
    TalentBuiltInResource::parse(text.as_bytes()).expect("a built-in Talent")
}

#[test]
fn my_mail_is_at_1_1_0_and_organizes_mail_after_reading_and_sending_it() {
    let resource = my_mail();
    assert_eq!(resource.connector, "mail");
    assert_eq!(resource.manifest.version.to_string(), "1.1.0");
    let wires: Vec<&str> = resource.manifest.capabilities.iter().map(|capability| capability.wire()).collect();
    assert_eq!(wires, ["read-mail", "send-mail", "organize-mail"]);
}

fn built_in(id: &str) -> TalentBuiltInResource {
    let text = BUILT_IN_MANIFESTS
        .iter()
        .find(|text| text.contains(&format!(r#""id": "{id}""#)))
        .unwrap_or_else(|| panic!("the built-in Talent {id}"));
    TalentBuiltInResource::parse(text.as_bytes()).expect("a built-in Talent")
}

#[test]
fn search_the_web_drives_the_web_search_connector_on_every_platform() {
    use super::{TalentCapability, TalentNeed};
    let resource = built_in("chercher-sur-le-web");
    assert_eq!(resource.connector, "web-search");
    let manifest = resource.manifest;
    assert_eq!(manifest.kind, TalentKind::WithCode);
    assert_eq!(manifest.capabilities, [TalentCapability::WebSearch]);
    assert_eq!(manifest.needs, [TalentNeed::Learning]);
    assert!(manifest.requires.is_empty());
    assert_eq!(manifest.platforms, None, "every platform");
    assert_eq!(manifest.localized["fr"].name, "Chercher sur le web");
    assert_eq!(manifest.localized["en"].name, "Search the web");
}

/// « Lire le web »: the Connector `web-reader`, every platform, same kind and need as « Chercher sur le web ».
#[test]
fn read_the_web_drives_the_web_reader_connector_on_every_platform() {
    use super::{TalentCapability, TalentNeed};
    let resource = built_in("lire-le-web");
    assert_eq!(resource.connector, "web-reader");
    let manifest = resource.manifest;
    assert_eq!(manifest.kind, TalentKind::WithCode);
    assert_eq!(manifest.capabilities, [TalentCapability::WebSearch]);
    assert_eq!(manifest.needs, [TalentNeed::Learning]);
    assert!(manifest.requires.is_empty());
    assert_eq!(manifest.platforms, None, "every platform");
    assert_eq!(manifest.localized["fr"].name, "Lire le web");
    assert_eq!(manifest.localized["en"].name, "Read the web");
}

#[test]
fn web_search_has_three_suppliers_among_the_built_in_talents() {
    use super::{TalentCapability, TalentDependencyRule};
    let manifests: Vec<_> = BUILT_IN_MANIFESTS
        .iter()
        .map(|text| TalentBuiltInResource::parse(text.as_bytes()).expect("built-in").manifest)
        .collect();
    let suppliers = TalentDependencyRule::suppliers(&manifests);
    assert_eq!(
        suppliers[&TalentCapability::WebSearch],
        ["chercher-sur-le-web", "lire-le-web", "naviguer-sur-le-web"]
    );
    assert_eq!(suppliers[&TalentCapability::BrowseWeb], ["naviguer-sur-le-web"]);
}

#[test]
fn my_mailbox_drives_the_imap_connector_on_every_platform() {
    use super::{TalentCapability, TalentNeed};
    let resource = built_in("ma-boite-mail");
    assert_eq!(resource.connector, "imap-1");
    let manifest = resource.manifest;
    assert_eq!(manifest.kind, TalentKind::WithCode);
    assert_eq!(
        manifest.capabilities,
        [TalentCapability::ReadMail, TalentCapability::SendMail, TalentCapability::OrganizeMail]
    );
    assert_eq!(manifest.needs, [TalentNeed::Mail]);
    assert!(manifest.requires.is_empty());
    assert_eq!(manifest.platforms, None, "every platform, unlike My email (macOS)");
    assert_eq!(manifest.localized["fr"].name, "Ma boîte mail");
    assert_eq!(manifest.localized["en"].name, "My mailbox");
    assert_eq!((manifest.author.name.as_str(), manifest.license.as_str()), ("Murmure", "MIT"));
}

#[test]
fn the_mail_capabilities_have_three_suppliers_among_the_built_in_talents_and_all_stay() {
    use super::{TalentCapability, TalentDependencyRule};
    let manifests: Vec<_> = BUILT_IN_MANIFESTS
        .iter()
        .map(|text| TalentBuiltInResource::parse(text.as_bytes()).expect("built-in").manifest)
        .collect();
    let suppliers = TalentDependencyRule::suppliers(&manifests);
    for capability in [TalentCapability::ReadMail, TalentCapability::SendMail, TalentCapability::OrganizeMail] {
        assert_eq!(suppliers[&capability], ["ma-boite-mail", "mes-mails", "microsoft-365"], "{capability:?}");
    }
}

#[test]
fn my_contacts_drives_the_contacts_connector_on_macos_and_reads_then_edits_contacts() {
    use super::{TalentCapability, TalentNeed};
    let resource = built_in("mes-contacts");
    assert_eq!(resource.connector, "contacts");
    let manifest = resource.manifest;
    assert_eq!(manifest.kind, TalentKind::WithCode);
    assert_eq!(manifest.capabilities, [TalentCapability::ReadContacts, TalentCapability::EditContacts]);
    assert_eq!(manifest.needs, [TalentNeed::Organizing]);
    assert!(manifest.requires.is_empty());
    assert_eq!(manifest.platforms, Some(vec![super::TalentPlatform::Macos]));
    assert_eq!(manifest.localized["fr"].name, "Mes contacts");
    assert_eq!(manifest.localized["en"].name, "My contacts");
    assert_eq!((manifest.author.name.as_str(), manifest.license.as_str()), ("Murmure", "MIT"));
}

#[test]
fn online_contacts_drive_the_carddav_connector_on_every_platform_with_the_same_capabilities() {
    use super::{TalentCapability, TalentNeed};
    let resource = built_in("mes-contacts-carddav");
    assert_eq!(resource.connector, "carddav");
    let manifest = resource.manifest;
    assert_eq!(manifest.kind, TalentKind::WithCode);
    assert_eq!(manifest.capabilities, [TalentCapability::ReadContacts, TalentCapability::EditContacts]);
    assert_eq!(manifest.needs, [TalentNeed::Organizing]);
    assert_eq!(manifest.platforms, None, "every platform, unlike My contacts (macOS)");
    assert_eq!(manifest.localized["fr"].name, "Mes contacts en ligne");
    assert_eq!(manifest.localized["en"].name, "My online contacts");
}

#[test]
fn the_contacts_capabilities_have_two_suppliers_among_the_built_in_talents_and_both_stay() {
    use super::{TalentCapability, TalentDependencyRule};
    let manifests: Vec<_> = BUILT_IN_MANIFESTS
        .iter()
        .map(|text| TalentBuiltInResource::parse(text.as_bytes()).expect("built-in").manifest)
        .collect();
    let suppliers = TalentDependencyRule::suppliers(&manifests);
    for capability in [TalentCapability::ReadContacts, TalentCapability::EditContacts] {
        assert_eq!(suppliers[&capability], ["mes-contacts", "mes-contacts-carddav"], "{capability:?}");
    }
}

#[test]
fn my_online_calendar_drives_the_caldav_connector_on_every_platform_with_the_calendar_capabilities() {
    use super::{TalentCapability, TalentNeed};
    let resource = built_in("mon-agenda-en-ligne");
    assert_eq!(resource.connector, "caldav");
    let manifest = resource.manifest;
    assert_eq!(manifest.kind, TalentKind::WithCode);
    assert_eq!(manifest.capabilities, [TalentCapability::ReadCalendar, TalentCapability::EditCalendar]);
    assert_eq!(manifest.needs, [TalentNeed::Calendar]);
    assert!(manifest.requires.is_empty());
    assert_eq!(manifest.platforms, None, "every platform (like the online contacts), unlike My calendar (macOS)");
    assert_eq!(manifest.localized["fr"].name, "Mon agenda en ligne");
    assert_eq!(manifest.localized["en"].name, "My online calendar");
    assert_eq!((manifest.author.name.as_str(), manifest.license.as_str()), ("Murmure", "MIT"));
}

#[test]
fn the_calendar_capabilities_have_three_suppliers_among_the_built_in_talents_and_all_stay() {
    use super::{TalentCapability, TalentDependencyRule};
    let manifests: Vec<_> = BUILT_IN_MANIFESTS
        .iter()
        .map(|text| TalentBuiltInResource::parse(text.as_bytes()).expect("built-in").manifest)
        .collect();
    let suppliers = TalentDependencyRule::suppliers(&manifests);
    for capability in [TalentCapability::ReadCalendar, TalentCapability::EditCalendar] {
        assert_eq!(suppliers[&capability], ["microsoft-365", "mon-agenda-en-ligne", "mon-calendrier"], "{capability:?}");
    }
}

#[test]
fn my_obsidian_vault_drives_the_obsidian_connector_on_macos_and_linux_with_the_notes_capabilities() {
    use super::{TalentCapability, TalentNeed, TalentPlatform};
    let resource = built_in("obsidian");
    assert_eq!(resource.connector, "obsidian");
    let manifest = resource.manifest;
    assert_eq!(manifest.kind, TalentKind::WithCode);
    assert_eq!(manifest.capabilities, [TalentCapability::ReadNotes, TalentCapability::EditNotes]);
    assert_eq!(manifest.needs, [TalentNeed::Organizing]);
    assert!(manifest.requires.is_empty());
    assert_eq!(manifest.platforms, Some(vec![TalentPlatform::Macos, TalentPlatform::Linux]));
    assert_eq!(manifest.localized["fr"].name, "Mon coffre Obsidian");
    assert_eq!(manifest.localized["en"].name, "My Obsidian vault");
    assert_eq!(manifest.minimum_murmure_version, built_in("mon-agenda-en-ligne").manifest.minimum_murmure_version);
    assert_eq!((manifest.author.name.as_str(), manifest.license.as_str()), ("Murmure", "MIT"));
}

#[test]
fn the_notes_capabilities_have_four_suppliers_among_the_built_in_talents_and_they_all_stay() {
    use super::{TalentCapability, TalentDependencyRule};
    let manifests: Vec<_> = BUILT_IN_MANIFESTS
        .iter()
        .map(|text| TalentBuiltInResource::parse(text.as_bytes()).expect("built-in").manifest)
        .collect();
    let suppliers = TalentDependencyRule::suppliers(&manifests);
    for capability in [TalentCapability::ReadNotes, TalentCapability::EditNotes] {
        assert_eq!(suppliers[&capability], ["bear", "mes-notes", "notion", "obsidian"], "{capability:?}");
    }
}

#[test]
fn my_messages_drives_the_messages_connector_on_macos_reads_conversations_and_sends_messages() {
    use super::{TalentCapability, TalentNeed};
    let resource = built_in("mes-messages");
    assert_eq!(resource.connector, "messages");
    let manifest = resource.manifest;
    assert_eq!(manifest.kind, TalentKind::WithCode);
    assert_eq!(manifest.capabilities, [TalentCapability::ReadMessages, TalentCapability::SendMessages]);
    assert_eq!(manifest.needs, [TalentNeed::Organizing]);
    assert!(manifest.requires.is_empty());
    assert_eq!(manifest.platforms, Some(vec![super::TalentPlatform::Macos]));
    assert_eq!(manifest.version.to_string(), "1.1.0", "like « Mes mails » when it learned to send");
    assert_eq!(manifest.minimum_murmure_version.to_string(), "1.0.0");
    assert_eq!(manifest.localized["fr"].name, "Mes messages");
    assert_eq!(manifest.localized["en"].name, "My messages");
    assert_eq!((manifest.author.name.as_str(), manifest.license.as_str()), ("Murmure", "MIT"));
}

/// Only « Mes messages », « Telegram », « Slack », « Discord » and « Microsoft Teams » send messages.
#[test]
fn only_the_five_message_talents_declare_the_send_messages_capability() {
    use super::TalentCapability;
    let senders: Vec<String> = BUILT_IN_MANIFESTS
        .iter()
        .map(|text| TalentBuiltInResource::parse(text.as_bytes()).expect("built-in").manifest)
        .filter(|manifest| manifest.capabilities.contains(&TalentCapability::SendMessages))
        .map(|manifest| manifest.id)
        .collect();
    let mut senders = senders;
    senders.sort();
    assert_eq!(senders, ["discord", "mes-messages", "slack", "teams", "telegram"]);
}

/// « Telegram »: the Connector `telegram`, macOS and Linux, need `organizing`, the two messages capabilities (no new one).
#[test]
fn telegram_drives_the_telegram_connector_on_macos_and_linux_with_the_messages_capabilities() {
    use super::{TalentCapability, TalentDependencyRule, TalentNeed, TalentPlatform};
    let resource = built_in("telegram");
    assert_eq!(resource.connector, "telegram");
    let manifest = resource.manifest;
    assert_eq!(manifest.kind, built_in("ma-maison").manifest.kind);
    assert_eq!(manifest.kind, TalentKind::WithCode);
    assert_eq!(manifest.capabilities, [TalentCapability::ReadMessages, TalentCapability::SendMessages]);
    assert_eq!(manifest.needs, [TalentNeed::Organizing]);
    assert!(manifest.requires.is_empty());
    assert_eq!(manifest.platforms, Some(vec![TalentPlatform::Macos, TalentPlatform::Linux]));
    assert_eq!((manifest.localized["fr"].name.as_str(), manifest.localized["en"].name.as_str()), ("Telegram", "Telegram"));
    assert!(!manifest.localized["fr"].summary.is_empty() && !manifest.localized["en"].summary.is_empty());
    assert_eq!(manifest.minimum_murmure_version, built_in("things").manifest.minimum_murmure_version);
    assert_eq!((manifest.author.name.as_str(), manifest.license.as_str()), ("Murmure", "MIT"));
    let manifests: Vec<_> = BUILT_IN_MANIFESTS
        .iter()
        .map(|text| TalentBuiltInResource::parse(text.as_bytes()).expect("built-in").manifest)
        .collect();
    let suppliers = TalentDependencyRule::suppliers(&manifests);
    assert_eq!(suppliers[&TalentCapability::ReadMessages], ["discord", "mes-messages", "slack", "teams", "telegram"]);
    assert_eq!(suppliers[&TalentCapability::SendMessages], ["discord", "mes-messages", "slack", "teams", "telegram"]);
}

/// The three task Talents: one Connector each, the same two capabilities, the same need, macOS and Linux.
#[test]
fn the_task_talents_drive_their_connector_with_the_task_capabilities_on_macos_and_linux() {
    use super::{TalentCapability, TalentNeed, TalentPlatform};
    for (id, fr, en) in [
        ("trello", "Trello", "Trello"),
        ("asana", "Asana", "Asana"),
        ("clickup", "ClickUp", "ClickUp"),
        ("todoist", "Todoist", "Todoist"),
    ] {
        let resource = built_in(id);
        assert_eq!(resource.connector, id);
        let manifest = resource.manifest;
        assert_eq!(manifest.kind, TalentKind::WithCode, "{id}");
        assert_eq!(manifest.capabilities, [TalentCapability::ReadTasks, TalentCapability::EditTasks], "{id}");
        assert_eq!(manifest.needs, [TalentNeed::Organizing], "{id}");
        assert!(manifest.requires.is_empty(), "{id}");
        assert_eq!(manifest.platforms, Some(vec![TalentPlatform::Macos, TalentPlatform::Linux]), "{id}");
        assert_eq!((manifest.localized["fr"].name.as_str(), manifest.localized["en"].name.as_str()), (fr, en), "{id}");
        assert_eq!(manifest.minimum_murmure_version, built_in("obsidian").manifest.minimum_murmure_version, "{id}");
        assert_eq!((manifest.author.name.as_str(), manifest.license.as_str()), ("Murmure", "MIT"), "{id}");
    }
}

/// Three suppliers of the same two capabilities; none of them replaces a reminders Talent.
#[test]
fn the_task_capabilities_have_three_suppliers_and_are_not_the_reminders_ones() {
    use super::{TalentCapability, TalentDependencyRule};
    let manifests: Vec<_> = BUILT_IN_MANIFESTS
        .iter()
        .map(|text| TalentBuiltInResource::parse(text.as_bytes()).expect("built-in").manifest)
        .collect();
    let suppliers = TalentDependencyRule::suppliers(&manifests);
    for capability in [TalentCapability::ReadTasks, TalentCapability::EditTasks] {
        assert_eq!(suppliers[&capability], ["asana", "clickup", "things", "todoist", "trello"], "{capability:?}");
    }
    assert_eq!(suppliers[&TalentCapability::EditReminders], ["mes-rappels"]);
}

/// « Mes documents »: the Connector `files`, macOS and Linux, and a supplier of the two files capabilities (with « Google Drive »).
#[test]
fn my_documents_drive_the_files_connector_on_macos_and_linux_with_the_files_capabilities() {
    use super::{TalentCapability, TalentDependencyRule, TalentNeed, TalentPlatform};
    let resource = built_in("mes-documents");
    assert_eq!(resource.connector, "files");
    let manifest = resource.manifest;
    assert_eq!(manifest.kind, TalentKind::WithCode);
    assert_eq!(manifest.capabilities, [TalentCapability::ReadFiles, TalentCapability::EditFiles]);
    assert_eq!(manifest.needs, [TalentNeed::Organizing]);
    assert!(manifest.requires.is_empty());
    assert_eq!(manifest.platforms, Some(vec![TalentPlatform::Macos, TalentPlatform::Linux]));
    assert_eq!((manifest.localized["fr"].name.as_str(), manifest.localized["en"].name.as_str()), ("Mes documents", "My documents"));
    assert_eq!(manifest.minimum_murmure_version, built_in("obsidian").manifest.minimum_murmure_version);
    assert_eq!((manifest.author.name.as_str(), manifest.license.as_str()), ("Murmure", "MIT"));
    let manifests: Vec<_> = BUILT_IN_MANIFESTS
        .iter()
        .map(|text| TalentBuiltInResource::parse(text.as_bytes()).expect("built-in").manifest)
        .collect();
    let suppliers = TalentDependencyRule::suppliers(&manifests);
    for capability in [TalentCapability::ReadFiles, TalentCapability::EditFiles] {
        assert_eq!(suppliers[&capability], ["google-drive", "mes-documents"], "{capability:?}");
    }
}

/// « Ma maison »: the Connector `home-assistant`, macOS and Linux, need `home`, and the only supplier of `control-home`.
#[test]
fn my_home_drives_the_home_assistant_connector_on_macos_and_linux_with_the_home_capability() {
    use super::{TalentCapability, TalentDependencyRule, TalentNeed, TalentPlatform};
    let resource = built_in("ma-maison");
    assert_eq!(resource.connector, "home-assistant");
    let manifest = resource.manifest;
    assert_eq!(manifest.kind, TalentKind::WithCode);
    assert_eq!(manifest.capabilities, [TalentCapability::ControlHome]);
    assert_eq!(manifest.needs, [TalentNeed::Home]);
    assert!(manifest.requires.is_empty());
    assert_eq!(manifest.platforms, Some(vec![TalentPlatform::Macos, TalentPlatform::Linux]));
    assert_eq!((manifest.localized["fr"].name.as_str(), manifest.localized["en"].name.as_str()), ("Ma maison", "My home"));
    assert_eq!(manifest.minimum_murmure_version, built_in("obsidian").manifest.minimum_murmure_version);
    assert_eq!((manifest.author.name.as_str(), manifest.license.as_str()), ("Murmure", "MIT"));
    let manifests: Vec<_> = BUILT_IN_MANIFESTS
        .iter()
        .map(|text| TalentBuiltInResource::parse(text.as_bytes()).expect("built-in").manifest)
        .collect();
    let suppliers = TalentDependencyRule::suppliers(&manifests);
    assert!(suppliers[&TalentCapability::ControlHome].contains(&"ma-maison".to_owned()));
}

/// « Notion »: the Connector `notion`, macOS and Linux, the two notes capabilities (no new capability) and the organizing need.
#[test]
fn notion_drives_the_notion_connector_on_macos_and_linux_with_the_notes_capabilities() {
    use super::{TalentCapability, TalentNeed, TalentPlatform};
    let resource = built_in("notion");
    assert_eq!(resource.connector, "notion");
    let manifest = resource.manifest;
    assert_eq!(manifest.kind, TalentKind::WithCode);
    assert_eq!(manifest.capabilities, [TalentCapability::ReadNotes, TalentCapability::EditNotes]);
    assert_eq!(manifest.needs, [TalentNeed::Organizing]);
    assert!(manifest.requires.is_empty());
    assert_eq!(manifest.platforms, Some(vec![TalentPlatform::Macos, TalentPlatform::Linux]));
    assert_eq!((manifest.localized["fr"].name.as_str(), manifest.localized["en"].name.as_str()), ("Notion", "Notion"));
    assert_eq!(manifest.localized["fr"].summary, "Cherche, lis et complète tes pages et bases Notion.");
    assert_eq!(manifest.localized["en"].summary, "Search, read and update your Notion pages and databases.");
    assert_eq!(manifest.minimum_murmure_version, built_in("obsidian").manifest.minimum_murmure_version);
    assert_eq!((manifest.author.name.as_str(), manifest.license.as_str()), ("Murmure", "MIT"));
}

/// « Google Drive »: the Connector `google-drive`, macOS and Linux, the two files capabilities (no new capability), the organizing need.
#[test]
fn google_drive_drives_the_google_drive_connector_on_macos_and_linux_with_the_files_capabilities() {
    use super::{TalentCapability, TalentNeed, TalentPlatform};
    let resource = built_in("google-drive");
    assert_eq!(resource.connector, "google-drive");
    let manifest = resource.manifest;
    assert_eq!(manifest.kind, TalentKind::WithCode);
    assert_eq!(manifest.capabilities, [TalentCapability::ReadFiles, TalentCapability::EditFiles]);
    assert_eq!(manifest.needs, [TalentNeed::Organizing]);
    assert!(manifest.requires.is_empty());
    assert_eq!(manifest.platforms, Some(vec![TalentPlatform::Macos, TalentPlatform::Linux]));
    assert_eq!((manifest.localized["fr"].name.as_str(), manifest.localized["en"].name.as_str()), ("Google Drive", "Google Drive"));
    assert_eq!(manifest.localized["fr"].summary, "Lis et complète tes documents et feuilles Google Drive.");
    assert_eq!(manifest.localized["en"].summary, "Read and update your Google Drive documents and sheets.");
    assert_eq!(manifest.minimum_murmure_version, built_in("obsidian").manifest.minimum_murmure_version);
    assert_eq!((manifest.author.name.as_str(), manifest.license.as_str()), ("Murmure", "MIT"));
}

/// « Microsoft 365 »: the Connector `microsoft-365`, macOS and Linux, the mail and calendar capabilities of the two other
/// Talents (no new capability), the needs `mail` and `calendar`.
#[test]
fn microsoft_365_drives_its_connector_on_macos_and_linux_with_the_mail_and_calendar_capabilities() {
    use super::{TalentCapability, TalentNeed, TalentPlatform};
    let resource = built_in("microsoft-365");
    assert_eq!(resource.connector, "microsoft-365");
    let manifest = resource.manifest;
    assert_eq!(manifest.kind, TalentKind::WithCode);
    assert_eq!(
        manifest.capabilities,
        [TalentCapability::ReadMail, TalentCapability::SendMail, TalentCapability::OrganizeMail, TalentCapability::ReadCalendar, TalentCapability::EditCalendar]
    );
    assert_eq!(manifest.needs, [TalentNeed::Mail, TalentNeed::Calendar]);
    assert!(manifest.requires.is_empty());
    assert_eq!(manifest.platforms, Some(vec![TalentPlatform::Macos, TalentPlatform::Linux]));
    assert_eq!((manifest.localized["fr"].name.as_str(), manifest.localized["en"].name.as_str()), ("Microsoft 365", "Microsoft 365"));
    assert_eq!(manifest.localized["fr"].summary, "Lis, cherche et range ton courrier Outlook et gère ton agenda Microsoft 365.");
    assert_eq!(manifest.localized["en"].summary, "Read, search and sort your Outlook mail and manage your Microsoft 365 calendar.");
    assert_eq!(manifest.minimum_murmure_version, built_in("obsidian").manifest.minimum_murmure_version);
    assert_eq!((manifest.author.name.as_str(), manifest.license.as_str()), ("Murmure", "MIT"));
}

#[test]
fn things_drives_the_things_connector_on_macos_only_with_the_task_capabilities() {
    use super::{TalentCapability, TalentNeed, TalentPlatform};
    let resource = built_in("things");
    assert_eq!(resource.connector, "things");
    let manifest = resource.manifest;
    assert_eq!(manifest.kind, TalentKind::WithCode);
    assert_eq!(manifest.capabilities, [TalentCapability::ReadTasks, TalentCapability::EditTasks]);
    assert_eq!(manifest.needs, [TalentNeed::Organizing]);
    assert!(manifest.requires.is_empty());
    assert_eq!(manifest.platforms, Some(vec![TalentPlatform::Macos]));
    assert_eq!((manifest.localized["fr"].name.as_str(), manifest.localized["en"].name.as_str()), ("Things", "Things"));
    assert_eq!(manifest.localized["fr"].summary, "Vois et crée tes tâches dans Things 3 sur ton Mac.");
    assert_eq!(manifest.localized["en"].summary, "See and create your tasks in Things 3 on your Mac.");
    assert_eq!(manifest.minimum_murmure_version, built_in("trello").manifest.minimum_murmure_version);
    assert_eq!((manifest.author.name.as_str(), manifest.license.as_str()), ("Murmure", "MIT"));
}

/// « Slack », « Discord » and « Microsoft Teams »: one Connector each, macOS and Linux, need `organizing`, the two messages capabilities (no new one).
#[test]
fn slack_discord_and_teams_drive_their_connectors_on_macos_and_linux_with_the_messages_capabilities() {
    use super::{TalentCapability, TalentNeed, TalentPlatform};
    for (id, name) in [("slack", "Slack"), ("discord", "Discord"), ("teams", "Microsoft Teams")] {
        let resource = built_in(id);
        assert_eq!(resource.connector, id);
        let manifest = resource.manifest;
        assert_eq!(manifest.id, id);
        assert_eq!(manifest.kind, TalentKind::WithCode, "{id}");
        assert_eq!(manifest.kind, built_in("ma-maison").manifest.kind);
        assert_eq!(manifest.capabilities, [TalentCapability::ReadMessages, TalentCapability::SendMessages], "{id}");
        assert_eq!(manifest.needs, [TalentNeed::Organizing], "{id}");
        assert!(manifest.requires.is_empty(), "{id}");
        assert_eq!(manifest.platforms, Some(vec![TalentPlatform::Macos, TalentPlatform::Linux]), "{id}");
        assert_eq!((manifest.localized["fr"].name.as_str(), manifest.localized["en"].name.as_str()), (name, name));
        assert!(!manifest.localized["fr"].summary.is_empty() && !manifest.localized["en"].summary.is_empty(), "{id}");
        assert_eq!(manifest.minimum_murmure_version, built_in("things").manifest.minimum_murmure_version, "{id}");
        assert_eq!((manifest.author.name.as_str(), manifest.license.as_str()), ("Murmure", "MIT"), "{id}");
    }
}

#[test]
fn bear_drives_the_bear_connector_on_macos_only_with_the_notes_capabilities() {
    use super::{TalentCapability, TalentNeed, TalentPlatform};
    let resource = built_in("bear");
    assert_eq!(resource.connector, "bear");
    let manifest = resource.manifest;
    assert_eq!(manifest.kind, TalentKind::WithCode);
    assert_eq!(manifest.capabilities, [TalentCapability::ReadNotes, TalentCapability::EditNotes]);
    assert_eq!(manifest.needs, [TalentNeed::Organizing]);
    assert!(manifest.requires.is_empty());
    assert_eq!(manifest.platforms, Some(vec![TalentPlatform::Macos]));
    assert_eq!((manifest.localized["fr"].name.as_str(), manifest.localized["en"].name.as_str()), ("Bear", "Bear"));
    assert_eq!(manifest.localized["fr"].summary, "Cherche, lis et écris tes notes Bear sur ton Mac.");
    assert_eq!(manifest.localized["en"].summary, "Search, read and write your Bear notes on your Mac.");
    assert_eq!(manifest.minimum_murmure_version, built_in("trello").manifest.minimum_murmure_version);
    assert_eq!((manifest.author.name.as_str(), manifest.license.as_str()), ("Murmure", "MIT"));
}

/// « Lumières Hue » and « Lumières Govee »: one Connector each, macOS and Linux, need `home`, capability `control-home`.
#[test]
fn the_hue_and_govee_lights_drive_their_connector_on_macos_and_linux_with_the_home_capability() {
    use super::{TalentCapability, TalentDependencyRule, TalentNeed, TalentPlatform};
    for (id, connector, fr, en, summary_fr, summary_en) in [
        (
            "lumieres-hue",
            "hue",
            "Lumières Hue",
            "Hue lights",
            "Allume, éteins ou règle les lumières Philips Hue de ta maison, sans Home Assistant.",
            "Turn on, turn off or adjust your Philips Hue lights, without Home Assistant.",
        ),
        (
            "lumieres-govee",
            "govee",
            "Lumières Govee",
            "Govee lights",
            "Allume, éteins ou règle tes lumières Govee, sans Home Assistant.",
            "Turn on, turn off or adjust your Govee lights, without Home Assistant.",
        ),
    ] {
        let resource = built_in(id);
        assert_eq!(resource.connector, connector, "{id}");
        let manifest = resource.manifest;
        assert_eq!(manifest.kind, TalentKind::WithCode, "{id}");
        assert_eq!(manifest.capabilities, [TalentCapability::ControlHome], "{id}");
        assert_eq!(manifest.needs, [TalentNeed::Home], "{id}");
        assert!(manifest.requires.is_empty(), "{id}");
        assert_eq!(manifest.platforms, Some(vec![TalentPlatform::Macos, TalentPlatform::Linux]), "{id}");
        assert_eq!((manifest.localized["fr"].name.as_str(), manifest.localized["en"].name.as_str()), (fr, en), "{id}");
        assert_eq!((manifest.localized["fr"].summary.as_str(), manifest.localized["en"].summary.as_str()), (summary_fr, summary_en), "{id}");
        assert_eq!(manifest.minimum_murmure_version, built_in("ma-maison").manifest.minimum_murmure_version, "{id}");
        assert_eq!((manifest.author.name.as_str(), manifest.license.as_str()), ("Murmure", "MIT"), "{id}");
    }
    let manifests: Vec<_> = BUILT_IN_MANIFESTS
        .iter()
        .map(|text| TalentBuiltInResource::parse(text.as_bytes()).expect("built-in").manifest)
        .collect();
    let suppliers = TalentDependencyRule::suppliers(&manifests);
    for id in ["ma-maison", "lumieres-hue", "lumieres-govee"] {
        assert!(suppliers[&TalentCapability::ControlHome].contains(&id.to_owned()), "{id}");
    }
}

/// « Spotify » and « Musique »: the Connectors `spotify` and `music`, the need `home`, the one capability `control-media`; Spotify on macOS and
/// Linux, Musique on macOS only.
#[test]
fn spotify_and_musique_drive_their_connector_with_the_media_capability_each_on_its_platforms() {
    use super::{TalentCapability, TalentDependencyRule, TalentNeed, TalentPlatform};
    for (id, connector, fr, en, summary_fr, summary_en, platforms) in [
        (
            "spotify",
            "spotify",
            "Spotify",
            "Spotify",
            "Lance, mets en pause ou choisis ta musique sur Spotify, et gère tes playlists.",
            "Play, pause or pick your music on Spotify, and manage your playlists.",
            vec![TalentPlatform::Macos, TalentPlatform::Linux],
        ),
        (
            "musique",
            "music",
            "Musique",
            "Music",
            "Lance ta musique et gère tes playlists dans l'app Musique de ton Mac.",
            "Play your music and manage your playlists in the Music app on your Mac.",
            vec![TalentPlatform::Macos],
        ),
    ] {
        let resource = built_in(id);
        assert_eq!(resource.connector, connector, "{id}");
        let manifest = resource.manifest;
        assert_eq!(manifest.kind, TalentKind::WithCode, "{id}");
        assert_eq!(manifest.capabilities, [TalentCapability::ControlMedia], "{id}");
        assert_eq!(manifest.needs, [TalentNeed::Home], "{id}");
        assert!(manifest.requires.is_empty(), "{id}");
        assert_eq!(manifest.platforms, Some(platforms), "{id}");
        assert_eq!((manifest.localized["fr"].name.as_str(), manifest.localized["en"].name.as_str()), (fr, en), "{id}");
        assert_eq!((manifest.localized["fr"].summary.as_str(), manifest.localized["en"].summary.as_str()), (summary_fr, summary_en), "{id}");
        assert_eq!(manifest.minimum_murmure_version, built_in("ma-maison").manifest.minimum_murmure_version, "{id}");
        assert_eq!((manifest.author.name.as_str(), manifest.license.as_str()), ("Murmure", "MIT"), "{id}");
    }
    let manifests: Vec<_> = BUILT_IN_MANIFESTS
        .iter()
        .map(|text| TalentBuiltInResource::parse(text.as_bytes()).expect("built-in").manifest)
        .collect();
    let suppliers = TalentDependencyRule::suppliers(&manifests);
    for id in ["spotify", "musique"] {
        assert!(suppliers[&TalentCapability::ControlMedia].contains(&id.to_owned()), "{id}");
    }
}

/// « Mes trajets »: the Connector `transit`, the need `travel`, no capability (« N'accède à aucune de tes données »: no personal data
/// is read, and no capability is added to the vocabulary), every platform.
#[test]
fn my_journeys_drive_the_transit_connector_with_no_capability_on_every_platform() {
    use super::{TalentDependencyRule, TalentNeed};
    let resource = built_in("mes-trajets");
    assert_eq!(resource.connector, "transit");
    let manifest = resource.manifest;
    assert_eq!(manifest.kind, TalentKind::WithCode);
    assert!(manifest.capabilities.is_empty(), "no personal data is read");
    assert_eq!(manifest.needs, [TalentNeed::Travel]);
    assert!(manifest.requires.is_empty());
    assert_eq!(manifest.platforms, None, "every platform");
    assert_eq!((manifest.localized["fr"].name.as_str(), manifest.localized["en"].name.as_str()), ("Mes trajets", "My journeys"));
    assert!(!manifest.localized["fr"].summary.is_empty() && !manifest.localized["en"].summary.is_empty());
    assert_eq!(manifest.minimum_murmure_version, built_in("ma-maison").manifest.minimum_murmure_version, "no new capability, no raised minimum version");
    assert_eq!((manifest.author.name.as_str(), manifest.license.as_str()), ("Murmure", "MIT"));
    let manifests: Vec<_> = BUILT_IN_MANIFESTS
        .iter()
        .map(|text| TalentBuiltInResource::parse(text.as_bytes()).expect("built-in").manifest)
        .collect();
    let suppliers = TalentDependencyRule::suppliers(&manifests);
    assert!(suppliers.values().all(|ids| !ids.contains(&"mes-trajets".to_owned())), "it supplies no capability: {suppliers:?}");
}

/// « Mes photos »: the Connector `photos`, the need `home`, the one capability `read-photos`, macOS only.
#[test]
fn mes_photos_drives_the_photos_connector_with_the_read_photos_capability_on_macos_only() {
    use super::{TalentCapability, TalentDependencyRule, TalentNeed, TalentPlatform};
    let resource = built_in("mes-photos");
    assert_eq!(resource.connector, "photos");
    let manifest = resource.manifest;
    assert_eq!(manifest.kind, TalentKind::WithCode);
    assert_eq!(manifest.capabilities, [TalentCapability::ReadPhotos]);
    assert_eq!(manifest.needs, [TalentNeed::Home]);
    assert!(manifest.requires.is_empty());
    assert_eq!(manifest.platforms, Some(vec![TalentPlatform::Macos]));
    assert_eq!((manifest.localized["fr"].name.as_str(), manifest.localized["en"].name.as_str()), ("Mes photos", "My photos"));
    assert_eq!(manifest.minimum_murmure_version, built_in("ma-maison").manifest.minimum_murmure_version);
    assert_eq!((manifest.author.name.as_str(), manifest.license.as_str()), ("Murmure", "MIT"));
    let manifests: Vec<_> = BUILT_IN_MANIFESTS
        .iter()
        .map(|text| TalentBuiltInResource::parse(text.as_bytes()).expect("built-in").manifest)
        .collect();
    let suppliers = TalentDependencyRule::suppliers(&manifests);
    assert!(suppliers[&TalentCapability::ReadPhotos].contains(&"mes-photos".to_owned()));
}

/// « Mon budget »: the Connector `budget`, macOS and Linux, the one capability `read-finances`, the need `paperwork`.
#[test]
fn my_budget_drives_the_budget_connector_with_the_read_finances_capability_on_macos_and_linux() {
    use super::{TalentCapability, TalentDependencyRule, TalentNeed, TalentPlatform};
    let resource = built_in("budget");
    assert_eq!(resource.connector, "budget");
    let manifest = resource.manifest;
    let finances = TalentCapability::from_wire("read-finances").expect("read-finances is in the closed vocabulary");
    assert_eq!(manifest.kind, TalentKind::WithCode);
    assert_eq!(manifest.capabilities, [finances]);
    assert_eq!(manifest.needs, [TalentNeed::Paperwork]);
    assert!(manifest.requires.is_empty());
    assert_eq!(manifest.platforms, Some(vec![TalentPlatform::Macos, TalentPlatform::Linux]));
    assert_eq!((manifest.localized["fr"].name.as_str(), manifest.localized["en"].name.as_str()), ("Mon budget", "My budget"));
    assert!(!manifest.localized["fr"].summary.is_empty() && !manifest.localized["en"].summary.is_empty());
    assert_eq!(manifest.minimum_murmure_version, built_in("ma-sante").manifest.minimum_murmure_version);
    assert_eq!((manifest.author.name.as_str(), manifest.license.as_str()), ("Murmure", "MIT"));
    let manifests: Vec<_> = BUILT_IN_MANIFESTS
        .iter()
        .map(|text| TalentBuiltInResource::parse(text.as_bytes()).expect("built-in").manifest)
        .collect();
    let suppliers = TalentDependencyRule::suppliers(&manifests);
    assert!(suppliers[&finances].contains(&"budget".to_owned()));
}
