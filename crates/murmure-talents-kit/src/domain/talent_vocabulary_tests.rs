//! Contract: « Vocabulaire fermé des capacités » and « Liste fermée des besoins ».

use super::{CapabilityLevel, TalentCapability, TalentNeed};

#[test]
fn every_capability_has_its_wire_name_and_comes_back_from_it() {
    let names = [
        "read-mail",
        "send-mail",
        "organize-mail",
        "read-calendar",
        "edit-calendar",
        "read-reminders",
        "edit-reminders",
        "read-notes",
        "edit-notes",
        "web-search",
        "browse-web",
        "read-contacts",
        "edit-contacts",
        "read-messages",
        "send-messages",
        "read-tasks",
        "edit-tasks",
        "read-files",
        "edit-files",
        "control-home",
        "control-media",
        "read-photos",
        "read-health",
        "read-finances",
    ];
    assert_eq!(TalentCapability::ALL.len(), names.len(), "the finances capability closes the list of display");
    assert_eq!(TalentCapability::ALL.len(), 24);
    for (capability, name) in TalentCapability::ALL.iter().zip(names) {
        assert_eq!(capability.wire(), name);
        assert_eq!(TalentCapability::from_wire(name), Some(*capability));
    }
    for outside in ["read_mail", "READ-MAIL", "send-sms", ""] {
        assert_eq!(TalentCapability::from_wire(outside), None, "{outside:?}");
    }
}

#[test]
fn reading_capabilities_read_and_the_others_act() {
    use TalentCapability::*;
    for capability in [ReadMail, ReadCalendar, ReadReminders, ReadNotes, WebSearch, ReadContacts, ReadMessages, ReadTasks, ReadFiles, ReadPhotos] {
        assert_eq!(capability.level(), CapabilityLevel::Read, "{capability:?}");
    }
    for capability in [SendMail, OrganizeMail, EditCalendar, EditReminders, EditNotes, BrowseWeb, EditContacts, SendMessages, EditTasks, EditFiles, ControlHome, ControlMedia] {
        assert_eq!(capability.level(), CapabilityLevel::Act, "{capability:?}");
    }
}

#[test]
fn the_health_capability_reads() {
    let health = TalentCapability::from_wire("read-health").expect("read-health is in the closed vocabulary");
    assert_eq!(health.level(), CapabilityLevel::Read);
}

#[test]
fn every_need_has_its_wire_name_and_comes_back_from_it() {
    let names = [
        "mail",
        "calendar",
        "organizing",
        "writing",
        "paperwork",
        "cooking",
        "travel",
        "home",
        "health",
        "learning",
    ];
    for (need, name) in TalentNeed::ALL.iter().zip(names) {
        assert_eq!(need.wire(), name);
        assert_eq!(TalentNeed::from_wire(name), Some(*need));
    }
    assert_eq!(TalentNeed::from_wire("shopping"), None);
}

#[test]
fn the_finances_capability_reads() {
    let finances = TalentCapability::from_wire("read-finances").expect("read-finances is in the closed vocabulary");
    assert_eq!(finances.level(), CapabilityLevel::Read);
}
