//! Vault - Secure storage with encryption
//!
//! Handles:
//! - Key derivation (Argon2id)
//! - Encryption at rest (AES-256-GCM)
//! - Session management
//! - Rate limiting
//!
//! Note: Scroll sealing (seal_scroll, unseal_scroll) has moved to megab.
//! This module provides only the crypto primitives.
//!
//! ## Feature Flags
//!
//! - `crypto`: Enables crypto primitives (seal, unseal, derive_key, etc.)
//! - `wallet`: Enables VaultStore (uses SecureSeed for zeroizing seed)

use zeroize::{Zeroize, ZeroizeOnDrop};

pub mod crypto;
pub mod session;

// VaultStore requires wallet feature
#[cfg(feature = "wallet")]
pub mod store;

pub use crypto::{
    CryptoError, SealedValue,
    derive_key, derive_app_key, generate_salt, seal, unseal,
    hash_passphrase, verify_passphrase, zeroize_key,
};
pub use session::{RateLimiter, SessionManager};

#[cfg(feature = "wallet")]
pub use store::VaultStore;

/// Zeroizing wrapper for seed phrase string
///
/// Automatically zeroizes the string content when dropped.
/// Use this when you need a seed phrase that cleans up after itself.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct SecureSeed(String);

impl SecureSeed {
    /// Create a new secure seed from a string
    pub fn new(seed: String) -> Self {
        Self(seed)
    }

    /// Get a reference to the seed phrase
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for SecureSeed {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
