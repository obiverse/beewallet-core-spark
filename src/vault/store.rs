//! Persistent vault storage using 9S FileNamespace
//!
//! # Security
//! - Seeds are returned as SecureSeed which zeroizes on drop
//! - Keys are zeroized after cryptographic operations
//! - Passphrase changes zeroize intermediate seed material
//!
//! # Storage Layout (9S paths)
//! - /passphrase-hash  -> PHC string for verification
//! - /salt             -> Base64 salt for key derivation
//! - /seed             -> SealedValue JSON (encrypted seed)

use super::crypto::{self, CryptoError, SealedValue};
use super::SecureSeed;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use crate::nine_s::{FileNamespace, Namespace};
use std::path::Path;
use thiserror::Error;
use zeroize::Zeroize;

#[derive(Error, Debug)]
pub enum StoreError {
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Crypto error: {0}")]
    Crypto(#[from] CryptoError),
    #[error("Vault not initialized")]
    VaultNotInitialized,
    #[error("Invalid passphrase")]
    InvalidPassphrase,
    #[error("Vault already initialized - reset required before re-initialization")]
    AlreadyInitialized,
    #[error("Rate limited: {0}")]
    RateLimited(String),
}

impl From<crate::nine_s::Error> for StoreError {
    fn from(e: crate::nine_s::Error) -> Self {
        StoreError::Storage(e.to_string())
    }
}

/// Persistent vault store using 9S FileNamespace
pub struct VaultStore {
    ns: FileNamespace,
    rate_limiter: std::sync::Mutex<super::session::RateLimiter>,
}

impl VaultStore {
    /// Open or create a vault store at the given path
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, StoreError> {
        let ns = FileNamespace::new(path)?;
        Ok(Self {
            ns,
            rate_limiter: std::sync::Mutex::new(super::session::RateLimiter::new()),
        })
    }

    /// Check if the vault has been initialized
    ///
    /// Returns true if the vault contains a valid passphrase hash.
    /// A vault that has been reset (containing null) returns false.
    pub fn is_initialized(&self) -> Result<bool, StoreError> {
        match self.ns.read("/passphrase-hash")? {
            Some(scroll) => {
                // Check that the value is a non-null string (valid PHC hash)
                // A null value indicates the vault was reset
                Ok(scroll.data.is_string())
            }
            None => Ok(false),
        }
    }

    /// Initialize the vault with a passphrase and seed
    /// Returns the vault key for immediate use
    ///
    /// # Security
    /// This method will fail if the vault is already initialized.
    /// Use `reset()` first if you need to re-initialize (requires user confirmation).
    pub fn initialize(
        &self,
        passphrase: &str,
        seed_phrase: &str,
    ) -> Result<[u8; 32], StoreError> {
        // SECURITY: Block re-initialization to prevent seed replacement attacks
        if self.is_initialized()? {
            return Err(StoreError::AlreadyInitialized);
        }

        self.initialize_internal(passphrase, seed_phrase)
    }

    /// Unlock the vault with a passphrase, returning the vault key
    ///
    /// # Rate Limiting
    /// This method is protected by rate limiting. After 3 failed attempts,
    /// there is an exponential backoff starting at 60 seconds.
    pub fn unlock(&self, passphrase: &str) -> Result<[u8; 32], StoreError> {
        // Check rate limiter BEFORE attempting unlock
        {
            let mut limiter = self.rate_limiter.lock().unwrap();
            if let Err(msg) = limiter.check_locked() {
                return Err(StoreError::RateLimited(msg));
            }
        }

        // Get stored passphrase hash
        let hash_scroll = self
            .ns
            .read("/passphrase-hash")?
            .ok_or(StoreError::VaultNotInitialized)?;

        let hash = hash_scroll.data.as_str()
            .ok_or_else(|| StoreError::Storage("Invalid passphrase hash format".into()))?;

        // Verify passphrase
        if !crypto::verify_passphrase(passphrase, hash)? {
            // Record failed attempt
            let mut limiter = self.rate_limiter.lock().unwrap();
            limiter.record_failure();
            return Err(StoreError::InvalidPassphrase);
        }

        // Success - reset rate limiter
        {
            let mut limiter = self.rate_limiter.lock().unwrap();
            limiter.record_success();
        }

        // Get salt and derive key
        let salt_scroll = self
            .ns
            .read("/salt")?
            .ok_or(StoreError::VaultNotInitialized)?;

        let salt_b64 = salt_scroll.data.as_str()
            .ok_or_else(|| StoreError::Storage("Invalid salt format".into()))?;

        let salt = BASE64
            .decode(salt_b64)
            .map_err(|e| CryptoError::InvalidData(e.to_string()))?;

        crypto::derive_key(passphrase, &salt).map_err(StoreError::Crypto)
    }

    /// Get remaining lockout time in seconds (0 if not locked)
    pub fn lockout_remaining(&self) -> u64 {
        let limiter = self.rate_limiter.lock().unwrap();
        limiter.lockout_remaining().unwrap_or(0)
    }

    /// Get the decrypted seed phrase
    ///
    /// # Security
    /// Returns a SecureSeed that automatically zeroizes when dropped.
    pub fn get_seed(&self, vault_key: &[u8; 32]) -> Result<SecureSeed, StoreError> {
        let seed_scroll = self
            .ns
            .read("/seed")?
            .ok_or(StoreError::VaultNotInitialized)?;

        let sealed: SealedValue = serde_json::from_value(seed_scroll.data)?;
        let mut plaintext = crypto::unseal(vault_key, &sealed)?;

        let seed_str = String::from_utf8(plaintext.clone())
            .map_err(|e| CryptoError::InvalidData(e.to_string()))?;

        // SECURITY: Zeroize the intermediate plaintext bytes
        plaintext.zeroize();

        Ok(SecureSeed::new(seed_str))
    }

    /// Change the passphrase (requires current passphrase)
    ///
    /// # Security
    /// - Current key is zeroized after use
    /// - Seed is automatically zeroized via SecureSeed when function returns
    pub fn change_passphrase(
        &self,
        current_passphrase: &str,
        new_passphrase: &str,
    ) -> Result<[u8; 32], StoreError> {
        // Unlock with current passphrase (proves ownership)
        let mut current_key = self.unlock(current_passphrase)?;

        // Get the seed (SecureSeed zeroizes on drop)
        let seed = self.get_seed(&current_key)?;

        // SECURITY: Zeroize the current key before re-initialization
        current_key.zeroize();

        // Re-initialize with new passphrase (authenticated, bypass init check)
        // seed is zeroized when dropped at end of function
        self.initialize_internal(new_passphrase, seed.as_str())
    }

    /// Internal initialization that bypasses the already-initialized check.
    /// Only called from authenticated contexts (change_passphrase).
    fn initialize_internal(
        &self,
        passphrase: &str,
        seed_phrase: &str,
    ) -> Result<[u8; 32], StoreError> {
        use serde_json::json;

        // Generate salt for key derivation
        let salt = crypto::generate_salt();

        // Derive encryption key from passphrase
        let vault_key = crypto::derive_key(passphrase, &salt)?;

        // Hash passphrase for verification
        let passphrase_hash = crypto::hash_passphrase(passphrase)?;

        // Encrypt seed with vault key
        let sealed_seed = crypto::seal(&vault_key, seed_phrase.as_bytes())?;

        // Store everything as scrolls
        self.ns.write("/passphrase-hash", json!(passphrase_hash))?;
        self.ns.write("/salt", json!(BASE64.encode(&salt)))?;
        self.ns.write("/seed", serde_json::to_value(&sealed_seed)?)?;

        Ok(vault_key)
    }

    /// Reset the vault (DANGER: destroys encrypted seed)
    ///
    /// Overwrites all vault data with null values and physically deletes the files.
    pub fn reset(&self) -> Result<(), StoreError> {
        use serde_json::json;

        // First overwrite with null values to corrupt any cached data
        let paths = self.ns.list("/")?;
        for path in paths {
            // Overwrite with null - corrupts the data
            let _ = self.ns.write(&path, json!(null));
        }

        // Now physically delete the files from disk
        self.ns.delete_all()?;

        Ok(())
    }

    /// Flush is a no-op for FileNamespace (writes are synchronous)
    pub fn flush(&self) -> Result<(), StoreError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    const TEST_SEED: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    #[test]
    fn vault_initialize_and_unlock() {
        let dir = tempdir().unwrap();
        let store = VaultStore::open(dir.path()).unwrap();

        assert!(!store.is_initialized().unwrap());

        let passphrase = "test-passphrase";
        let key1 = store.initialize(passphrase, TEST_SEED).unwrap();
        assert!(store.is_initialized().unwrap());

        let key2 = store.unlock(passphrase).unwrap();
        assert_eq!(key1, key2);

        let retrieved_seed = store.get_seed(&key1).unwrap();
        assert_eq!(retrieved_seed.as_str(), TEST_SEED);
    }

    #[test]
    fn vault_wrong_passphrase_fails() {
        let dir = tempdir().unwrap();
        let store = VaultStore::open(dir.path()).unwrap();

        store.initialize("correct", TEST_SEED).unwrap();
        assert!(store.unlock("wrong").is_err());
    }

    #[test]
    fn vault_change_passphrase() {
        let dir = tempdir().unwrap();
        let store = VaultStore::open(dir.path()).unwrap();

        store.initialize("old-pass", TEST_SEED).unwrap();

        let new_key = store.change_passphrase("old-pass", "new-pass").unwrap();

        // Old passphrase should fail
        assert!(store.unlock("old-pass").is_err());

        // New passphrase should work
        let unlocked_key = store.unlock("new-pass").unwrap();
        assert_eq!(new_key, unlocked_key);

        // Seed should still be accessible
        let seed = store.get_seed(&new_key).unwrap();
        assert_eq!(seed.as_str(), TEST_SEED);
    }

    #[test]
    fn vault_reinitialization_blocked() {
        let dir = tempdir().unwrap();
        let store = VaultStore::open(dir.path()).unwrap();

        // First initialization succeeds
        store.initialize("passphrase", TEST_SEED).unwrap();

        // Second initialization should fail
        let result = store.initialize("new-passphrase", "different seed phrase here");
        assert!(matches!(result, Err(StoreError::AlreadyInitialized)));

        // Original seed should still be intact
        let key = store.unlock("passphrase").unwrap();
        let seed = store.get_seed(&key).unwrap();
        assert_eq!(seed.as_str(), TEST_SEED);
    }

    #[test]
    fn vault_persistence_across_instances() {
        let dir = tempdir().unwrap();

        // Initialize with first instance
        {
            let store = VaultStore::open(dir.path()).unwrap();
            store.initialize("passphrase", TEST_SEED).unwrap();
        }

        // Read with second instance
        {
            let store = VaultStore::open(dir.path()).unwrap();
            assert!(store.is_initialized().unwrap());

            let key = store.unlock("passphrase").unwrap();
            let seed = store.get_seed(&key).unwrap();
            assert_eq!(seed.as_str(), TEST_SEED);
        }
    }

    #[test]
    fn vault_reset_allows_reinit() {
        // Tests fix for issue #2: reset() should allow re-initialization
        let dir = tempdir().unwrap();
        let store = VaultStore::open(dir.path()).unwrap();

        // Initialize
        store.initialize("passphrase", TEST_SEED).unwrap();
        assert!(store.is_initialized().unwrap());

        // Reset
        store.reset().unwrap();

        // is_initialized() should return false after reset
        assert!(!store.is_initialized().unwrap(), "Vault should not be initialized after reset");

        // Re-initialization should succeed
        let result = store.initialize("new-passphrase", "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong");
        assert!(result.is_ok(), "Re-initialization should succeed after reset");

        // New credentials should work
        let key = store.unlock("new-passphrase").unwrap();
        let seed = store.get_seed(&key).unwrap();
        assert_eq!(seed.as_str(), "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong");
    }

    #[test]
    fn vault_rate_limiting_enforced() {
        // Tests fix for issue #4: rate limiting should be enforced
        let dir = tempdir().unwrap();
        let store = VaultStore::open(dir.path()).unwrap();

        store.initialize("correct", TEST_SEED).unwrap();

        // First 3 wrong attempts should fail with InvalidPassphrase
        for i in 0..3 {
            let result = store.unlock("wrong");
            assert!(
                matches!(result, Err(StoreError::InvalidPassphrase)),
                "Attempt {} should fail with InvalidPassphrase",
                i + 1
            );
        }

        // 4th attempt should be rate limited
        let result = store.unlock("wrong");
        assert!(
            matches!(result, Err(StoreError::RateLimited(_))),
            "Should be rate limited after 3 failures"
        );

        // Even correct password should be blocked during lockout
        let result = store.unlock("correct");
        assert!(
            matches!(result, Err(StoreError::RateLimited(_))),
            "Even correct password should be blocked during lockout"
        );

        // Verify lockout_remaining returns non-zero
        assert!(store.lockout_remaining() > 0, "Should have remaining lockout time");
    }
}
