//! Why a Talent is revoked (docs/murmure/talents.md, « Catalogue »).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TalentRevocationReason {
    Malicious,
    Broken,
    AuthorRequest,
    Policy,
}

impl TalentRevocationReason {
    pub fn wire(self) -> &'static str {
        match self {
            Self::Malicious => "malicious",
            Self::Broken => "broken",
            Self::AuthorRequest => "authorRequest",
            Self::Policy => "policy",
        }
    }

    pub fn from_wire(value: &str) -> Option<Self> {
        [
            Self::Malicious,
            Self::Broken,
            Self::AuthorRequest,
            Self::Policy,
        ]
        .into_iter()
        .find(|reason| reason.wire() == value)
    }
}
