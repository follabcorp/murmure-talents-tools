//! The closed vocabularies of a manifest (docs/murmure/talents.md, « Vocabulaire fermé des capacités » and « Liste
//! fermée des besoins »). The order of the lists is the order of display.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CapabilityLevel {
    Read,
    Act,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TalentCapability {
    ReadMail,
    SendMail,
    OrganizeMail,
    ReadCalendar,
    EditCalendar,
    ReadReminders,
    EditReminders,
    ReadNotes,
    EditNotes,
    WebSearch,
    BrowseWeb,
}

impl TalentCapability {
    /// In the order of display.
    pub const ALL: [TalentCapability; 11] = [
        Self::ReadMail,
        Self::SendMail,
        Self::OrganizeMail,
        Self::ReadCalendar,
        Self::EditCalendar,
        Self::ReadReminders,
        Self::EditReminders,
        Self::ReadNotes,
        Self::EditNotes,
        Self::WebSearch,
        Self::BrowseWeb,
    ];

    /// The name in `talent.json` (`read-mail`).
    pub fn wire(self) -> &'static str {
        match self {
            Self::ReadMail => "read-mail",
            Self::SendMail => "send-mail",
            Self::OrganizeMail => "organize-mail",
            Self::ReadCalendar => "read-calendar",
            Self::EditCalendar => "edit-calendar",
            Self::ReadReminders => "read-reminders",
            Self::EditReminders => "edit-reminders",
            Self::ReadNotes => "read-notes",
            Self::EditNotes => "edit-notes",
            Self::WebSearch => "web-search",
            Self::BrowseWeb => "browse-web",
        }
    }

    pub fn from_wire(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|capability| capability.wire() == value)
    }

    /// `read-*` and `web-search` read; `send-mail`, `organize-mail`, `edit-*` and `browse-web` act.
    pub fn level(self) -> CapabilityLevel {
        match self {
            Self::ReadMail
            | Self::ReadCalendar
            | Self::ReadReminders
            | Self::ReadNotes
            | Self::WebSearch => CapabilityLevel::Read,
            Self::SendMail
            | Self::OrganizeMail
            | Self::EditCalendar
            | Self::EditReminders
            | Self::EditNotes
            | Self::BrowseWeb => CapabilityLevel::Act,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TalentNeed {
    Mail,
    Calendar,
    Organizing,
    Writing,
    Paperwork,
    Cooking,
    Travel,
    Home,
    Health,
    Learning,
}

impl TalentNeed {
    /// In the order of the headings of the Catalogue.
    pub const ALL: [TalentNeed; 10] = [
        Self::Mail,
        Self::Calendar,
        Self::Organizing,
        Self::Writing,
        Self::Paperwork,
        Self::Cooking,
        Self::Travel,
        Self::Home,
        Self::Health,
        Self::Learning,
    ];

    pub fn wire(self) -> &'static str {
        match self {
            Self::Mail => "mail",
            Self::Calendar => "calendar",
            Self::Organizing => "organizing",
            Self::Writing => "writing",
            Self::Paperwork => "paperwork",
            Self::Cooking => "cooking",
            Self::Travel => "travel",
            Self::Home => "home",
            Self::Health => "health",
            Self::Learning => "learning",
        }
    }

    pub fn from_wire(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|need| need.wire() == value)
    }
}
