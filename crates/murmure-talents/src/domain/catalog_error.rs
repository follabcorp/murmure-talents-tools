/// A refusal of the tool; the message is French, shown after `ERREUR : `. It never carries key material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogError(pub String);

impl CatalogError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}
