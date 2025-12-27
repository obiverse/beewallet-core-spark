//! Cryptographic utilities for vault encryption
//!
//! # Security Parameters
//! - Key derivation: Argon2id with hardened parameters (64 MiB memory, 3 iterations)
//! - Encryption: AES-256-GCM with random nonces
//! - All sensitive material should be zeroized after use
//!
//! # Known Limitations
//!
//! ## Nonce Collision Risk (Issue #6)
//! Uses 96-bit random nonces for AES-GCM. Birthday paradox gives ~2^48 encryptions
//! before 50% collision probability. For a wallet encrypting hundreds of times per day,
//! this is acceptable. AES-GCM-SIV would be nonce-misuse resistant but adds complexity.
//!
//! ## Session Key Not Bound to Hash (Issue #5)
//! If an attacker modifies `/passphrase-hash` on disk, unlock succeeds with a wrong
//! passphrase, but decryption will fail (key mismatch). This is a DoS vector, not
//! a confidentiality breach. Future: consider MAC over passphrase hash file.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Algorithm, Argon2, Params, Version,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use zeroize::Zeroize;

/// Argon2id hardened parameters per OWASP recommendations
/// Memory: 64 MiB (65536 KiB)
/// Iterations: 3
/// Parallelism: 4
/// Output: 32 bytes (256 bits)
const ARGON2_MEMORY_KIB: u32 = 65536; // 64 MiB
const ARGON2_ITERATIONS: u32 = 3;
const ARGON2_PARALLELISM: u32 = 4;
const ARGON2_OUTPUT_LEN: usize = 32;

/// Create a hardened Argon2id instance
fn create_argon2() -> Argon2<'static> {
    let params = Params::new(
        ARGON2_MEMORY_KIB,
        ARGON2_ITERATIONS,
        ARGON2_PARALLELISM,
        Some(ARGON2_OUTPUT_LEN),
    )
    .expect("Invalid Argon2 parameters");

    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
}

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),
    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),
    #[error("Key derivation failed: {0}")]
    KeyDerivationFailed(String),
    #[error("Invalid data: {0}")]
    InvalidData(String),
}

/// Sealed value with version for forward compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealedValue {
    pub version: u8,
    pub nonce: String,
    pub ciphertext: String,
}

const SEALED_VERSION: u8 = 1;

/// Derive a 32-byte encryption key from a passphrase using Argon2id
///
/// Uses hardened parameters: 64 MiB memory, 3 iterations, 4 parallelism.
/// This provides strong resistance against GPU/ASIC attacks.
pub fn derive_key(passphrase: &str, salt: &[u8]) -> Result<[u8; 32], CryptoError> {
    let argon2 = create_argon2();

    // Derive directly into a fixed-size buffer (no PHC string truncation)
    let mut key = [0u8; ARGON2_OUTPUT_LEN];
    argon2
        .hash_password_into(passphrase.as_bytes(), salt, &mut key)
        .map_err(|e| CryptoError::KeyDerivationFailed(e.to_string()))?;

    Ok(key)
}

/// Zeroize a key in place
pub fn zeroize_key(key: &mut [u8; 32]) {
    key.zeroize();
}

/// Hash a passphrase for storage (verification only)
///
/// Uses hardened Argon2id parameters. The output is a PHC string
/// that can be stored and later verified.
pub fn hash_passphrase(passphrase: &str) -> Result<String, CryptoError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = create_argon2();
    let hash = argon2
        .hash_password(passphrase.as_bytes(), &salt)
        .map_err(|e| CryptoError::KeyDerivationFailed(e.to_string()))?;
    Ok(hash.to_string())
}

/// Verify a passphrase against a stored hash
///
/// The hash contains its own parameters, so this works with both
/// old (default) and new (hardened) hashes.
pub fn verify_passphrase(passphrase: &str, hash: &str) -> Result<bool, CryptoError> {
    let parsed_hash =
        PasswordHash::new(hash).map_err(|e| CryptoError::InvalidData(e.to_string()))?;
    // Use create_argon2() but verification extracts params from stored hash
    Ok(create_argon2()
        .verify_password(passphrase.as_bytes(), &parsed_hash)
        .is_ok())
}

/// Encrypt data with a 32-byte key
pub fn seal(key: &[u8; 32], plaintext: &[u8]) -> Result<SealedValue, CryptoError> {
    let cipher =
        Aes256Gcm::new_from_slice(key).map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;

    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;

    Ok(SealedValue {
        version: SEALED_VERSION,
        nonce: BASE64.encode(nonce_bytes),
        ciphertext: BASE64.encode(ciphertext),
    })
}

/// Decrypt data with a 32-byte key
pub fn unseal(key: &[u8; 32], sealed: &SealedValue) -> Result<Vec<u8>, CryptoError> {
    if sealed.version != SEALED_VERSION {
        return Err(CryptoError::InvalidData(format!(
            "Unsupported sealed version: {}",
            sealed.version
        )));
    }

    let cipher =
        Aes256Gcm::new_from_slice(key).map_err(|e| CryptoError::DecryptionFailed(e.to_string()))?;

    let nonce_bytes = BASE64
        .decode(&sealed.nonce)
        .map_err(|e| CryptoError::InvalidData(e.to_string()))?;

    if nonce_bytes.len() != 12 {
        return Err(CryptoError::InvalidData(
            "Nonce must be 12 bytes".to_string(),
        ));
    }

    let ciphertext = BASE64
        .decode(&sealed.ciphertext)
        .map_err(|e| CryptoError::InvalidData(e.to_string()))?;

    let nonce = Nonce::from_slice(&nonce_bytes);

    cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| CryptoError::DecryptionFailed(e.to_string()))
}

/// Generate a random salt
pub fn generate_salt() -> [u8; 16] {
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    salt
}

/// Derive an app-specific key from a master key using HKDF-SHA256
///
/// This provides cryptographic isolation between apps:
/// - Each app_key produces a unique, deterministic derived key
/// - Apps cannot decrypt each other's data even with filesystem access
/// - Master key compromise affects all apps, but app key compromise is isolated
///
/// # Algorithm
/// Uses HKDF (RFC 5869) with SHA-256:
/// - IKM (Input Key Material): master_key
/// - Salt: "beewallet-9s-v1" (domain separation)
/// - Info: app_key bytes (context binding)
///
/// # Security
/// - Derived keys are cryptographically independent
/// - No way to reverse from derived key to master key
/// - app_key acts as context, ensuring different apps get different keys
pub fn derive_app_key(master_key: &[u8; 32], app_key: &str) -> [u8; 32] {
    // Note: sha2 is already imported at module level via hmac_sha256

    // HKDF-Extract: PRK = HMAC-SHA256(salt, IKM)
    // Using a fixed salt for domain separation
    let salt = b"beewallet-9s-v1";

    // Simple HKDF implementation using HMAC-SHA256
    // Extract phase
    let mut hmac_key = [0u8; 64]; // SHA256 block size
    if salt.len() <= 64 {
        hmac_key[..salt.len()].copy_from_slice(salt);
    }

    let prk = hmac_sha256(&hmac_key[..salt.len()], master_key);

    // Expand phase: OKM = HMAC-SHA256(PRK, info || 0x01)
    let mut info_with_counter = Vec::with_capacity(app_key.len() + 1);
    info_with_counter.extend_from_slice(app_key.as_bytes());
    info_with_counter.push(0x01);

    hmac_sha256(&prk, &info_with_counter)
}

/// HMAC-SHA256 implementation
fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    use sha2::{Sha256, Digest};

    const BLOCK_SIZE: usize = 64;
    const IPAD: u8 = 0x36;
    const OPAD: u8 = 0x5c;

    // If key is longer than block size, hash it first
    let key_block: [u8; BLOCK_SIZE] = if key.len() > BLOCK_SIZE {
        let mut hasher = Sha256::new();
        hasher.update(key);
        let hash = hasher.finalize();
        let mut block = [0u8; BLOCK_SIZE];
        block[..32].copy_from_slice(&hash);
        block
    } else {
        let mut block = [0u8; BLOCK_SIZE];
        block[..key.len()].copy_from_slice(key);
        block
    };

    // Inner hash: H((K XOR ipad) || data)
    let mut inner_key = [0u8; BLOCK_SIZE];
    for i in 0..BLOCK_SIZE {
        inner_key[i] = key_block[i] ^ IPAD;
    }

    let mut inner_hasher = Sha256::new();
    inner_hasher.update(&inner_key);
    inner_hasher.update(data);
    let inner_hash = inner_hasher.finalize();

    // Outer hash: H((K XOR opad) || inner_hash)
    let mut outer_key = [0u8; BLOCK_SIZE];
    for i in 0..BLOCK_SIZE {
        outer_key[i] = key_block[i] ^ OPAD;
    }

    let mut outer_hasher = Sha256::new();
    outer_hasher.update(&outer_key);
    outer_hasher.update(&inner_hash);
    let result = outer_hasher.finalize();

    let mut output = [0u8; 32];
    output.copy_from_slice(&result);
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seal_unseal() {
        let key = [42u8; 32];
        let plaintext = b"secret data";

        let sealed = seal(&key, plaintext).unwrap();
        let decrypted = unseal(&key, &sealed).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_wrong_key_fails() {
        let key1 = [1u8; 32];
        let key2 = [2u8; 32];
        let plaintext = b"secret";

        let sealed = seal(&key1, plaintext).unwrap();
        assert!(unseal(&key2, &sealed).is_err());
    }

    #[test]
    fn test_passphrase_hash_verify() {
        let passphrase = "my-secret-passphrase";
        let hash = hash_passphrase(passphrase).unwrap();

        assert!(verify_passphrase(passphrase, &hash).unwrap());
        assert!(!verify_passphrase("wrong", &hash).unwrap());
    }

    #[test]
    fn test_derive_key() {
        let salt = generate_salt();
        let key1 = derive_key("password", &salt).unwrap();
        let key2 = derive_key("password", &salt).unwrap();
        let key3 = derive_key("different", &salt).unwrap();

        // Same passphrase + salt = same key
        assert_eq!(key1, key2);
        // Different passphrase = different key
        assert_ne!(key1, key3);
    }

    // ========================================================================
    // App Key Derivation Tests (HKDF)
    // ========================================================================

    #[test]
    fn test_derive_app_key_deterministic() {
        let master = [42u8; 32];

        // Same inputs = same output
        let key1 = derive_app_key(&master, "beewallet");
        let key2 = derive_app_key(&master, "beewallet");
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_derive_app_key_different_apps() {
        let master = [42u8; 32];

        // Different app_keys = different derived keys
        let key_beewallet = derive_app_key(&master, "beewallet");
        let key_nostr = derive_app_key(&master, "nostr");
        let key_lightning = derive_app_key(&master, "lightning");

        assert_ne!(key_beewallet, key_nostr);
        assert_ne!(key_beewallet, key_lightning);
        assert_ne!(key_nostr, key_lightning);
    }

    #[test]
    fn test_derive_app_key_different_masters() {
        // Different master keys = different derived keys (same app)
        let master1 = [1u8; 32];
        let master2 = [2u8; 32];

        let key1 = derive_app_key(&master1, "beewallet");
        let key2 = derive_app_key(&master2, "beewallet");

        assert_ne!(key1, key2);
    }

    #[test]
    fn test_derive_app_key_not_truncated_master() {
        // Derived key should NOT be the master key truncated/modified trivially
        let master = [42u8; 32];
        let derived = derive_app_key(&master, "app");

        assert_ne!(derived, master);
    }

    #[test]
    fn test_derive_app_key_cryptographic_isolation() {
        // This simulates two users with different master keys
        // using the same app - they cannot decrypt each other's data
        let user_a_master = [0xAA; 32];
        let user_b_master = [0xBB; 32];

        let user_a_app_key = derive_app_key(&user_a_master, "shared-app");
        let user_b_app_key = derive_app_key(&user_b_master, "shared-app");

        // Different users get different keys even for the same app
        assert_ne!(user_a_app_key, user_b_app_key);

        // User A encrypts data
        let plaintext = b"user A secret data";
        let sealed = seal(&user_a_app_key, plaintext).unwrap();

        // User A can decrypt
        let decrypted = unseal(&user_a_app_key, &sealed).unwrap();
        assert_eq!(decrypted, plaintext);

        // User B cannot decrypt User A's data
        let result = unseal(&user_b_app_key, &sealed);
        assert!(result.is_err());
    }

    #[test]
    fn test_hmac_sha256_basic() {
        // Basic HMAC test - verify it produces 32-byte output
        let key = b"test-key";
        let data = b"test-data";
        let result = hmac_sha256(key, data);
        assert_eq!(result.len(), 32);

        // Same inputs = same output
        let result2 = hmac_sha256(key, data);
        assert_eq!(result, result2);

        // Different inputs = different output
        let result3 = hmac_sha256(key, b"different-data");
        assert_ne!(result, result3);
    }
}
