//! `MAJOR.MINOR.PATCH`, strict (docs/murmure/talents.md, « Manifeste »).

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SemanticVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl SemanticVersion {
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Three numbers of 0 to 999 999, without leading zero, without pre-release or build suffix.
    pub fn parse(value: &str) -> Option<Self> {
        let mut parts = value.split('.');
        let major = Self::number(parts.next()?)?;
        let minor = Self::number(parts.next()?)?;
        let patch = Self::number(parts.next()?)?;
        if parts.next().is_some() {
            return None;
        }
        Some(Self {
            major,
            minor,
            patch,
        })
    }

    fn number(part: &str) -> Option<u32> {
        let valid = !part.is_empty()
            && part.len() <= 6
            && part.bytes().all(|byte| byte.is_ascii_digit())
            && (part.len() == 1 || !part.starts_with('0'));
        if valid {
            part.parse().ok()
        } else {
            None
        }
    }
}

impl fmt::Display for SemanticVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}
