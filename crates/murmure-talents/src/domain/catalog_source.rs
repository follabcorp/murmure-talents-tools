//! What the disk adapter hands to the rules, and what the rules report.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Regular,
    SymbolicLink,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogFile {
    pub path: String,
    pub data: Vec<u8>,
    pub kind: FileKind,
    pub permissions: u16,
}

/// What the disk holds of a conversion source: the files read, and the paths too large to be read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceSnapshot {
    pub files: Vec<CatalogFile>,
    pub oversized: Vec<String>,
}

/// One folder `talents/<directory_id>/`: its `talent.json` and its other files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogSource {
    pub directory_id: String,
    pub manifest_json: Vec<u8>,
    pub files: Vec<CatalogFile>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticKind {
    InvalidContent,
    SuspiciousInstructions,
    MailSendingWithoutCapability,
    /// A present `.github/CODEOWNERS` differs from the one the manifests give.
    CodeownersOutdated,
    /// The manifest lists no GitHub account yet (an exported Talent, to complete): a warning, never an error.
    MissingAuthorGithub,
    /// A `SKILL.md` seems written in a language other than English: a warning, never an error.
    NotEnglishInstructions,
    /// A `SKILL.md` does not tell the Agent to reply in the language the person uses: a warning.
    MissingReplyLanguageInstruction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub talent_id: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationReport {
    pub errors: Vec<Diagnostic>,
    pub warnings: Vec<Diagnostic>,
}

impl ValidationReport {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    /// « Pré-relecture du Catalogue », the verdict, `ERREUR [id] …` then `AVERTISSEMENT [id] …`, the closing line.
    pub fn text(&self) -> String {
        let verdict = if self.is_valid() { "Validation : conforme." } else { "Validation : refusée." };
        let mut lines = vec!["Pré-relecture du Catalogue".to_owned(), verdict.to_owned()];
        lines.extend(self.errors.iter().map(|error| format!("ERREUR [{}] {}", error.talent_id, error.message)));
        lines.extend(self.warnings.iter().map(|warning| format!("AVERTISSEMENT [{}] {}", warning.talent_id, warning.message)));
        lines.push("Recherche statique défensive uniquement ; relecture humaine obligatoire avant publication.".to_owned());
        lines.join("\n") + "\n"
    }
}
