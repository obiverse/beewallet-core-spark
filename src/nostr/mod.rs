//! Nostr Protocol Primitives
//!
//! Provides Nostr signing and encryption capabilities derived from MasterKey.
//! This module exposes the bare minimum for higher layers to build upon.
//!
//! ## Capabilities
//!
//! - **Signing**: Sign events with NIP-06 derived keys
//! - **Encryption**: NIP-44 encrypted payloads (modern, audited)
//!
//! ## Architecture
//!
//! ```text
//! MasterKey
//!     │
//!     └─► NostrSigner (factory)
//!             │
//!             ├─► sign_event()      - Sign unsigned events
//!             ├─► encrypt()         - NIP-44 encryption
//!             ├─► decrypt()         - NIP-44 decryption
//!             └─► public_key()      - Get npub/hex
//! ```
//!
//! Higher layers (megab) can use NostrSigner to build relay clients.

use crate::keys::{KeyError, MasterKey};
use nostr::nips::nip44;
use nostr::{Event, EventBuilder, FromBech32, Keys, Kind, PublicKey, Tag, ToBech32, UnsignedEvent};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NostrError {
    #[error("Key derivation error: {0}")]
    KeyError(#[from] KeyError),
    #[error("Signing error: {0}")]
    SigningError(String),
    #[error("Encryption error: {0}")]
    EncryptionError(String),
    #[error("Decryption error: {0}")]
    DecryptionError(String),
    #[error("Invalid public key: {0}")]
    InvalidPublicKey(String),
}

/// Nostr signer - factory for signing and encryption operations
///
/// Created from a MasterKey, provides all Nostr cryptographic operations.
/// The underlying keys are zeroized when dropped.
pub struct NostrSigner {
    keys: Keys,
}

impl NostrSigner {
    /// Create a NostrSigner from a MasterKey
    ///
    /// Uses NIP-06 derivation path: m/44'/1237'/0'/0/0
    pub fn from_master_key(master: &MasterKey) -> Result<Self, NostrError> {
        let keys = master.nostr_keys(None)?;
        Ok(Self { keys })
    }

    /// Create a NostrSigner from a MasterKey with a specific account
    ///
    /// Uses NIP-06 derivation path: m/44'/1237'/account'/0/0
    pub fn from_master_key_with_account(master: &MasterKey, account: u32) -> Result<Self, NostrError> {
        let keys = master.nostr_keys(Some(account))?;
        Ok(Self { keys })
    }

    /// Get the public key in hex format
    pub fn public_key_hex(&self) -> String {
        self.keys.public_key().to_hex()
    }

    /// Get the public key in bech32 (npub) format
    pub fn npub(&self) -> Result<String, NostrError> {
        self.keys
            .public_key()
            .to_bech32()
            .map_err(|e| NostrError::SigningError(e.to_string()))
    }

    /// Get the raw public key
    pub fn public_key(&self) -> PublicKey {
        self.keys.public_key()
    }

    /// Sign an unsigned event
    ///
    /// Returns a fully signed Event ready for publishing to relays.
    pub fn sign_event(&self, unsigned: UnsignedEvent) -> Result<Event, NostrError> {
        unsigned
            .sign_with_keys(&self.keys)
            .map_err(|e| NostrError::SigningError(e.to_string()))
    }

    /// Create and sign a text note (kind 1)
    pub fn sign_text_note(&self, content: &str, tags: Vec<Tag>) -> Result<Event, NostrError> {
        let unsigned = EventBuilder::new(Kind::TextNote, content)
            .tags(tags)
            .build(self.keys.public_key());
        self.sign_event(unsigned)
    }

    /// Create and sign a custom event
    pub fn sign_custom_event(
        &self,
        kind: Kind,
        content: &str,
        tags: Vec<Tag>,
    ) -> Result<Event, NostrError> {
        let unsigned = EventBuilder::new(kind, content)
            .tags(tags)
            .build(self.keys.public_key());
        self.sign_event(unsigned)
    }

    /// Encrypt a message using NIP-44 (modern, audited encryption)
    ///
    /// # Arguments
    /// * `recipient` - The recipient's public key (hex or npub)
    /// * `plaintext` - The message to encrypt
    ///
    /// # Returns
    /// Base64-encoded ciphertext suitable for NIP-44 events
    pub fn encrypt(&self, recipient: &PublicKey, plaintext: &str) -> Result<String, NostrError> {
        nip44::encrypt(
            self.keys.secret_key(),
            recipient,
            plaintext,
            nip44::Version::V2,
        )
        .map_err(|e| NostrError::EncryptionError(e.to_string()))
    }

    /// Decrypt a message using NIP-44
    ///
    /// # Arguments
    /// * `sender` - The sender's public key
    /// * `ciphertext` - Base64-encoded ciphertext from NIP-44 event
    pub fn decrypt(&self, sender: &PublicKey, ciphertext: &str) -> Result<String, NostrError> {
        nip44::decrypt(self.keys.secret_key(), sender, ciphertext)
            .map_err(|e| NostrError::DecryptionError(e.to_string()))
    }

    /// Get access to the underlying nostr Keys (for advanced usage)
    ///
    /// Use with caution - prefer the higher-level methods.
    pub fn keys(&self) -> &Keys {
        &self.keys
    }

    /// Sign an arbitrary message with Schnorr signature (BIP-340)
    ///
    /// This is used for authentication protocols like BeeBase Lightning Address
    /// registration. The message is hashed with SHA-256 before signing.
    ///
    /// # Returns
    /// A 128-character hex string representing the 64-byte Schnorr signature.
    ///
    /// # Example
    /// ```rust,ignore
    /// let message = format!("register:{}:{}:{}", mobinumber, webhook_url, timestamp);
    /// let signature = signer.sign_message_schnorr(&message)?;
    /// ```
    pub fn sign_message_schnorr(&self, message: &str) -> Result<String, NostrError> {
        use nostr::secp256k1::{Message, Secp256k1};
        use sha2::{Digest, Sha256};

        // Hash the message with SHA-256 (same as BeeBase expects)
        let mut hasher = Sha256::new();
        hasher.update(message.as_bytes());
        let hash = hasher.finalize();

        // Create secp256k1 message from hash
        let msg = Message::from_digest_slice(&hash)
            .map_err(|e| NostrError::SigningError(format!("Invalid message hash: {}", e)))?;

        // Sign with Schnorr (BIP-340)
        let secp = Secp256k1::new();
        let keypair = self.keys.secret_key().keypair(&secp);
        let signature = secp.sign_schnorr(&msg, &keypair);

        // Return as hex (64 bytes = 128 hex chars)
        // Use nostr's hex utility
        let bytes = signature.serialize();
        Ok(bytes.iter().map(|b| format!("{:02x}", b)).collect())
    }
}

/// Parse a public key from hex or bech32 (npub) format
pub fn parse_public_key(input: &str) -> Result<PublicKey, NostrError> {
    // Try npub first
    if input.starts_with("npub") {
        return PublicKey::from_bech32(input)
            .map_err(|e| NostrError::InvalidPublicKey(e.to_string()));
    }

    // Try hex
    PublicKey::from_hex(input).map_err(|e| NostrError::InvalidPublicKey(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_MNEMONIC: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    #[test]
    fn test_signer_from_master_key() {
        let master = MasterKey::from_mnemonic(TEST_MNEMONIC).unwrap();
        let signer = NostrSigner::from_master_key(&master).unwrap();

        let npub = signer.npub().unwrap();
        assert!(npub.starts_with("npub1"));

        let hex = signer.public_key_hex();
        assert_eq!(hex.len(), 64);
    }

    #[test]
    fn test_sign_text_note() {
        let master = MasterKey::from_mnemonic(TEST_MNEMONIC).unwrap();
        let signer = NostrSigner::from_master_key(&master).unwrap();

        let event = signer.sign_text_note("Hello, Nostr!", vec![]).unwrap();
        assert_eq!(event.kind, Kind::TextNote);
        assert_eq!(event.content, "Hello, Nostr!");
        assert!(event.verify().is_ok());
    }

    #[test]
    fn test_encrypt_decrypt() {
        let master = MasterKey::from_mnemonic(TEST_MNEMONIC).unwrap();
        let signer = NostrSigner::from_master_key(&master).unwrap();

        // Self-encrypt (for testing)
        let plaintext = "Secret message";
        let ciphertext = signer.encrypt(&signer.public_key(), plaintext).unwrap();
        let decrypted = signer.decrypt(&signer.public_key(), &ciphertext).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_different_accounts() {
        let master = MasterKey::from_mnemonic(TEST_MNEMONIC).unwrap();

        let signer0 = NostrSigner::from_master_key_with_account(&master, 0).unwrap();
        let signer1 = NostrSigner::from_master_key_with_account(&master, 1).unwrap();

        // Different accounts should have different keys
        assert_ne!(signer0.public_key_hex(), signer1.public_key_hex());
    }

    #[test]
    fn test_parse_public_key() {
        let master = MasterKey::from_mnemonic(TEST_MNEMONIC).unwrap();
        let signer = NostrSigner::from_master_key(&master).unwrap();

        let hex = signer.public_key_hex();
        let npub = signer.npub().unwrap();

        // Both should parse to the same key
        let from_hex = parse_public_key(&hex).unwrap();
        let from_npub = parse_public_key(&npub).unwrap();

        assert_eq!(from_hex, from_npub);
        assert_eq!(from_hex, signer.public_key());
    }

    #[test]
    fn test_cross_party_encryption() {
        let master1 = MasterKey::from_mnemonic(TEST_MNEMONIC).unwrap();
        let master2 = MasterKey::from_mnemonic(
            "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong",
        )
        .unwrap();

        let signer1 = NostrSigner::from_master_key(&master1).unwrap();
        let signer2 = NostrSigner::from_master_key(&master2).unwrap();

        // Signer1 encrypts a message for signer2
        let plaintext = "Hello from signer1!";
        let ciphertext = signer1.encrypt(&signer2.public_key(), plaintext).unwrap();

        // Signer2 decrypts it
        let decrypted = signer2.decrypt(&signer1.public_key(), &ciphertext).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_sign_message_schnorr() {
        let master = MasterKey::from_mnemonic(TEST_MNEMONIC).unwrap();
        let signer = NostrSigner::from_master_key(&master).unwrap();

        // Test message format used for BeeBase registration
        let message = "register:650073047435:https://example.com/webhook:1703000000";
        let signature = signer.sign_message_schnorr(message).unwrap();

        // Signature should be 128 hex chars (64 bytes)
        assert_eq!(signature.len(), 128);
        // Should be valid hex
        assert!(signature.chars().all(|c| c.is_ascii_hexdigit()));

        // Note: Schnorr signatures are NOT deterministic (random nonce)
        // Each call produces a different valid signature

        // Different message should produce different signature
        let different = signer.sign_message_schnorr("different message").unwrap();
        assert_ne!(signature, different);
    }
}
