//! Sealed Scrolls - Shareable encrypted content
//!
//! Scrolls can be sealed for secure sharing. A sealed scroll is a self-contained
//! encrypted envelope that can be shared via URI and unsealed with the correct password.
//!
//! ## Design Philosophy
//!
//! A scroll is to 9S what a file is to Plan 9. Just as files can be encrypted and
//! shared in Unix, scrolls can be sealed and shared in 9S.
//!
//! Sealing is a scroll operation, not a domain-specific operation:
//! - `scroll.seal(password?)` → SealedScroll
//! - `SealedScroll::unseal(password?)` → Scroll
//! - `sealed.to_uri()` → "beescroll://v1/..."
//! - `SealedScroll::from_uri("beescroll://...")` → SealedScroll
//!
//! ## Usage
//!
//! ```rust,ignore
//! use beewallet_core_spark::nine_s::{Scroll, SealedScroll};
//!
//! // Create and seal a scroll
//! let scroll = Scroll::new("/notes/secret", json!({"content": "Hello"}));
//! let sealed = scroll.seal(Some("password123"))?;
//!
//! // Share as URI
//! let uri = sealed.to_uri();
//! // => "beescroll://v1/eyJrZXkiOi..."
//!
//! // Unseal on another device
//! let sealed = SealedScroll::from_uri(&uri)?;
//! let scroll = sealed.unseal(Some("password123"))?;
//! ```

use crate::vault::{derive_key, generate_salt, seal, unseal, CryptoError, SealedValue};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD as BASE64_URL, Engine};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use zeroize::Zeroize;

use super::Scroll;

/// Maximum content size for sealed scrolls (64 KB)
/// Larger content should be chunked or stored differently
pub const MAX_SEALED_SIZE: usize = 65536;

/// Sealed scroll envelope for sharing
///
/// Contains everything needed to unseal the original scroll:
/// - Encrypted scroll JSON
/// - Nonce for AES-GCM
/// - Salt for password derivation (if password-protected)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealedScroll {
    /// Format version (currently 1)
    pub version: u8,
    /// Base64-encoded ciphertext (encrypted scroll JSON)
    pub ciphertext: String,
    /// Base64-encoded nonce
    pub nonce: String,
    /// Base64-encoded salt (only present if password protected)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub salt: Option<String>,
    /// Whether a password is required to unseal
    pub has_password: bool,
    /// Unix timestamp when sealed
    pub sealed_at: i64,
    /// Original scroll type (for display before unsealing)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scroll_type: Option<String>,
}

const SEALED_VERSION: u8 = 1;

/// Default key for scrolls without password (deterministic obfuscation)
/// This is NOT secure - it just makes the blob opaque to casual observation
fn default_key() -> [u8; 32] {
    // SHA256("beescroll:no-password") - deterministic default
    [
        0x7b, 0x2f, 0x4c, 0x8d, 0x3a, 0x9e, 0x5f, 0x7a, 0xd5, 0x9d, 0x2b, 0x4e, 0x8f, 0x3a, 0xac,
        0x5d, 0x7b, 0xf5, 0x9f, 0x2c, 0x4d, 0x8e, 0x3c, 0xaf, 0x5e, 0x7c, 0xb5, 0x9e, 0x2d, 0x4f,
        0x8a, 0x3d,
    ]
}

impl SealedScroll {
    /// Unseal this envelope, recovering the original scroll
    ///
    /// # Arguments
    /// * `password` - Password if the scroll is password-protected
    ///
    /// # Errors
    /// - `CryptoError::DecryptionFailed` if password is wrong or data corrupted
    /// - `CryptoError::InvalidData` if format is invalid
    pub fn unseal(&self, password: Option<&str>) -> Result<Scroll, CryptoError> {
        if self.version != SEALED_VERSION {
            return Err(CryptoError::InvalidData(format!(
                "Unsupported sealed version: {}",
                self.version
            )));
        }

        let mut key = if self.has_password {
            let pwd = password.ok_or_else(|| {
                CryptoError::DecryptionFailed("Password required but not provided".to_string())
            })?;

            let salt_b64 = self.salt.as_ref().ok_or_else(|| {
                CryptoError::InvalidData("Password-protected scroll missing salt".to_string())
            })?;

            let salt = BASE64_URL
                .decode(salt_b64)
                .map_err(|e| CryptoError::InvalidData(format!("Invalid salt: {}", e)))?;

            derive_key(pwd, &salt)?
        } else {
            default_key()
        };

        let sealed = SealedValue {
            version: 1,
            nonce: self.nonce.clone(),
            ciphertext: self.ciphertext.clone(),
        };

        let plaintext_bytes = unseal(&key, &sealed)?;
        key.zeroize();

        let scroll_json = String::from_utf8(plaintext_bytes)
            .map_err(|e| CryptoError::DecryptionFailed(format!("Invalid UTF-8: {}", e)))?;

        serde_json::from_str(&scroll_json)
            .map_err(|e| CryptoError::InvalidData(format!("Invalid scroll JSON: {}", e)))
    }

    /// Encode as a shareable URI
    ///
    /// Format: `beescroll://v1/{base64url_encoded_json}`
    pub fn to_uri(&self) -> String {
        let json = serde_json::to_string(self).unwrap_or_default();
        let encoded = BASE64_URL.encode(json);
        format!("beescroll://v1/{}", encoded)
    }

    /// Parse a sealed scroll from URI
    ///
    /// Accepts:
    /// - `beescroll://v1/{base64url_encoded_json}`
    /// - Raw JSON
    pub fn from_uri(input: &str) -> Result<Self, CryptoError> {
        let trimmed = input.trim();

        // Handle beescroll:// URI
        if let Some(stripped) = trimmed.strip_prefix("beescroll://v1/") {
            let json_bytes = BASE64_URL
                .decode(stripped)
                .map_err(|e| CryptoError::InvalidData(format!("Invalid base64 URI: {}", e)))?;

            let json = String::from_utf8(json_bytes)
                .map_err(|e| CryptoError::InvalidData(format!("Invalid UTF-8 in URI: {}", e)))?;

            return serde_json::from_str(&json)
                .map_err(|e| CryptoError::InvalidData(format!("Invalid sealed scroll: {}", e)));
        }

        // Handle legacy beenote:// URI for backwards compatibility
        if let Some(stripped) = trimmed.strip_prefix("beenote://v1/") {
            let json_bytes = BASE64_URL
                .decode(stripped)
                .map_err(|e| CryptoError::InvalidData(format!("Invalid base64 URI: {}", e)))?;

            let json = String::from_utf8(json_bytes)
                .map_err(|e| CryptoError::InvalidData(format!("Invalid UTF-8 in URI: {}", e)))?;

            // Legacy format had different structure, try to adapt
            return serde_json::from_str(&json)
                .map_err(|e| CryptoError::InvalidData(format!("Invalid sealed scroll: {}", e)));
        }

        // Assume raw JSON
        if trimmed.starts_with('{') {
            return serde_json::from_str(trimmed)
                .map_err(|e| CryptoError::InvalidData(format!("Invalid scroll JSON: {}", e)));
        }

        Err(CryptoError::InvalidData(
            "Input must be a beescroll:// URI or JSON".to_string(),
        ))
    }

    /// Check if this sealed scroll requires a password
    pub fn requires_password(&self) -> bool {
        self.has_password
    }
}

// ============================================================================
// Scroll extension methods for sealing
// ============================================================================

impl Scroll {
    /// Seal this scroll for sharing
    ///
    /// Creates an encrypted envelope that can be shared via URI.
    /// Optional password provides additional security.
    ///
    /// # Arguments
    /// * `password` - Optional password for decryption
    ///
    /// # Example
    /// ```rust,ignore
    /// let scroll = Scroll::new("/notes/secret", json!({"content": "Hello"}));
    /// let sealed = scroll.seal(Some("password123"))?;
    /// let uri = sealed.to_uri();
    /// ```
    pub fn seal(&self, password: Option<&str>) -> Result<SealedScroll, CryptoError> {
        // Serialize scroll to JSON
        let scroll_json = serde_json::to_string(self)
            .map_err(|e| CryptoError::EncryptionFailed(format!("Failed to serialize: {}", e)))?;

        if scroll_json.len() > MAX_SEALED_SIZE {
            return Err(CryptoError::InvalidData(format!(
                "Scroll exceeds maximum sealed size of {} bytes",
                MAX_SEALED_SIZE
            )));
        }

        let (mut key, salt) = match password {
            Some(pwd) if !pwd.is_empty() => {
                let salt = generate_salt();
                let key = derive_key(pwd, &salt)?;
                (key, Some(salt))
            }
            _ => (default_key(), None),
        };

        let sealed = seal(&key, scroll_json.as_bytes())?;
        key.zeroize();

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        Ok(SealedScroll {
            version: SEALED_VERSION,
            ciphertext: sealed.ciphertext,
            nonce: sealed.nonce,
            salt: salt.map(|s| BASE64_URL.encode(s)),
            has_password: password.is_some() && !password.unwrap_or_default().is_empty(),
            sealed_at: timestamp,
            scroll_type: Some(self.type_.clone()),
        })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_seal_unseal_no_password() {
        let scroll = Scroll::new("/test/note", json!({"title": "Hello", "content": "World"}));
        let sealed = scroll.seal(None).unwrap();
        let unsealed = sealed.unseal(None).unwrap();

        assert_eq!(unsealed.key, scroll.key);
        assert_eq!(unsealed.data, scroll.data);
    }

    #[test]
    fn test_seal_unseal_with_password() {
        let scroll = Scroll::new("/secret", json!({"data": "classified"}));
        let sealed = scroll.seal(Some("password123")).unwrap();

        // Correct password works
        let unsealed = sealed.unseal(Some("password123")).unwrap();
        assert_eq!(unsealed.data["data"], "classified");

        // Wrong password fails
        assert!(sealed.unseal(Some("wrong")).is_err());

        // No password fails
        assert!(sealed.unseal(None).is_err());
    }

    #[test]
    fn test_sealed_has_password_flag() {
        let scroll = Scroll::new("/test", json!({}));

        let no_pwd = scroll.seal(None).unwrap();
        assert!(!no_pwd.has_password);
        assert!(no_pwd.salt.is_none());

        let with_pwd = scroll.seal(Some("password")).unwrap();
        assert!(with_pwd.has_password);
        assert!(with_pwd.salt.is_some());
    }

    #[test]
    fn test_uri_roundtrip() {
        let scroll = Scroll::new("/vault/notes/abc", json!({
            "title": "Secret Note",
            "content": "Very secret content"
        })).set_type("vault/note@v1");

        let sealed = scroll.seal(Some("test")).unwrap();
        let uri = sealed.to_uri();

        assert!(uri.starts_with("beescroll://v1/"));

        let parsed = SealedScroll::from_uri(&uri).unwrap();
        let unsealed = parsed.unseal(Some("test")).unwrap();

        assert_eq!(unsealed.key, scroll.key);
        assert_eq!(unsealed.type_, scroll.type_);
        assert_eq!(unsealed.data, scroll.data);
    }

    #[test]
    fn test_from_uri_accepts_json() {
        let scroll = Scroll::new("/test", json!({"x": 1}));
        let sealed = scroll.seal(None).unwrap();
        let json = serde_json::to_string(&sealed).unwrap();

        let parsed = SealedScroll::from_uri(&json).unwrap();
        let unsealed = parsed.unseal(None).unwrap();
        assert_eq!(unsealed.data["x"], 1);
    }

    #[test]
    fn test_empty_password_treated_as_none() {
        let scroll = Scroll::new("/test", json!({}));
        let sealed = scroll.seal(Some("")).unwrap();

        assert!(!sealed.has_password);

        // Can unseal without password
        let unsealed = sealed.unseal(None).unwrap();
        assert_eq!(unsealed.key, scroll.key);
    }

    #[test]
    fn test_scroll_type_preserved() {
        let scroll = Scroll::new("/vault/notes/abc", json!({"title": "Test"}))
            .set_type("vault/note@v1");

        let sealed = scroll.seal(None).unwrap();
        assert_eq!(sealed.scroll_type, Some("vault/note@v1".to_string()));

        let unsealed = sealed.unseal(None).unwrap();
        assert_eq!(unsealed.type_, "vault/note@v1");
    }

    #[test]
    fn test_metadata_preserved() {
        use crate::nine_s::{kingdoms, verbs, Tense};

        let scroll = Scroll::new("/vault/notes/abc", json!({"title": "Test"}))
            .set_type("vault/note@v1")
            .with_kingdom(kingdoms::CONTENT)
            .with_verb(verbs::CREATES)
            .with_tense(Tense::Past)
            .with_extension("pinned", json!(true));

        let sealed = scroll.seal(None).unwrap();
        let unsealed = sealed.unseal(None).unwrap();

        assert_eq!(unsealed.metadata.kingdom, Some("content".to_string()));
        assert_eq!(unsealed.metadata.verb, Some("creates".to_string()));
        assert_eq!(unsealed.metadata.tense, Some(Tense::Past));
        assert_eq!(unsealed.metadata.extensions.get("pinned"), Some(&json!(true)));
    }
}
