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
    ReadContacts,
    EditContacts,
    ReadMessages,
    SendMessages,
    ReadTasks,
    EditTasks,
    ReadFiles,
    EditFiles,
    ControlHome,
    ControlMedia,
    ReadPhotos,
    ReadHealth,
    ReadFinances,
}

impl TalentCapability {
    /// In the order of display.
    pub const ALL: [TalentCapability; 24] = [
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
        Self::ReadContacts,
        Self::EditContacts,
        Self::ReadMessages,
        Self::SendMessages,
        Self::ReadTasks,
        Self::EditTasks,
        Self::ReadFiles,
        Self::EditFiles,
        Self::ControlHome,
        Self::ControlMedia,
        Self::ReadPhotos,
        Self::ReadHealth,
        Self::ReadFinances,
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
            Self::ReadContacts => "read-contacts",
            Self::EditContacts => "edit-contacts",
            Self::ReadMessages => "read-messages",
            Self::SendMessages => "send-messages",
            Self::ReadTasks => "read-tasks",
            Self::EditTasks => "edit-tasks",
            Self::ReadFiles => "read-files",
            Self::EditFiles => "edit-files",
            Self::ControlHome => "control-home",
            Self::ControlMedia => "control-media",
            Self::ReadPhotos => "read-photos",
            Self::ReadHealth => "read-health",
            Self::ReadFinances => "read-finances",
        }
    }

    pub fn from_wire(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|capability| capability.wire() == value)
    }

    /// `read-*` and `web-search` read; `send-mail`, `organize-mail`, `edit-*` (contacts included) and `browse-web` act.
    pub fn level(self) -> CapabilityLevel {
        match self {
            Self::ReadMail
            | Self::ReadCalendar
            | Self::ReadReminders
            | Self::ReadNotes
            | Self::WebSearch
            | Self::ReadContacts
            | Self::ReadMessages
            | Self::ReadTasks
            | Self::ReadFiles
            | Self::ReadPhotos
            | Self::ReadHealth
            | Self::ReadFinances => CapabilityLevel::Read,
            Self::SendMail
            | Self::OrganizeMail
            | Self::EditCalendar
            | Self::EditReminders
            | Self::EditNotes
            | Self::BrowseWeb
            | Self::EditContacts
            | Self::SendMessages
            | Self::EditTasks
            | Self::EditFiles
            | Self::ControlHome
            | Self::ControlMedia => CapabilityLevel::Act,
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
