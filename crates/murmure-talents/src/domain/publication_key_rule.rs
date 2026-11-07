use base64::{engine::general_purpose::STANDARD, Engine};
use ed25519_dalek::SigningKey;

/// `publication-key --public`: what is printed about a private key.
pub struct PublicationKeyRule;

impl PublicationKeyRule {
    /// The Base64 of the Ed25519 public key of the 32 private bytes, without a newline; never the private key.
    pub fn public_key(private_key: &[u8; 32]) -> String {
        STANDARD.encode(Self::public_bytes(private_key))
    }

    /// The 32 bytes of the public key, to compare with the key a delegation names.
    pub fn public_bytes(private_key: &[u8; 32]) -> [u8; 32] {
        SigningKey::from_bytes(private_key).verifying_key().to_bytes()
    }
}
