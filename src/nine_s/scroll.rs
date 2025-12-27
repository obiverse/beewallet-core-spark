//! Scroll - The universal data envelope in 9S
//!
//! Everything flows through Scrolls. No parallel type systems.
//! Domain semantics come from type_ field and metadata.
//!
//! # Structure
//!
//! - `key`: Path/address (encoded ontology)
//! - `type_`: Schema hint ("domain/entity@version")
//! - `metadata`: Timestamps + linguistic + taxonomic + extensions
//! - `data`: Payload (opaque to Kernel)

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Scroll - The universal data envelope
///
/// Every piece of data in 9S is a Scroll.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Scroll {
    /// Unique address (encoded ontology)
    /// Examples: "/vault/notes/abc123", "/ln/balance", "/wallet/txs/xyz"
    pub key: String,

    /// Schema hint: "domain/entity@version"
    /// Examples: "vault/note@v1", "ln/balance@v1", "wallet/tx@v1"
    #[serde(rename = "type", default = "default_type")]
    pub type_: String,

    /// Semantic metadata + timestamps
    #[serde(default)]
    pub metadata: Metadata,

    /// Payload (opaque to Kernel)
    /// The Kernel stores and retrieves this; never interprets it.
    pub data: Value,
}

fn default_type() -> String {
    "scroll/generic@v1".to_string()
}

/// Metadata attached to every Scroll
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    // ========================================================================
    // Timestamps (ISO 8601)
    // ========================================================================

    /// Creation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,

    /// Last update timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,

    /// Last sync timestamp (OIOI layer)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synced_at: Option<String>,

    /// TTL for ephemeral scrolls
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,

    // ========================================================================
    // Lifecycle
    // ========================================================================

    /// Soft delete flag
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted: Option<bool>,

    /// Version number (increments on each write)
    #[serde(default)]
    pub version: u64,

    /// Content hash (SHA-256 hex of key + type + data)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,

    // ========================================================================
    // Linguistic Model (Subject-Verb-Object)
    // ========================================================================

    /// Who/what acts (mobinumber, "wallet:master", "ln:{node}")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,

    /// Action taken ("owns", "sends", "receives", "creates", "updates", "deletes")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verb: Option<String>,

    /// Target of action (scroll key, amount, pubkey)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<String>,

    /// Temporal aspect
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tense: Option<Tense>,

    // ========================================================================
    // Taxonomic Model (Kingdom-Phylum-Class)
    // ========================================================================

    /// Broadest category: "financial", "content", "security", "system", "directory"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kingdom: Option<String>,

    /// Major division within kingdom
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phylum: Option<String>,

    /// Common characteristics: "transaction", "invoice", "note", "policy"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,

    // ========================================================================
    // Domain-specific extensions
    // ========================================================================

    /// Extensible key-value pairs for domain-specific metadata
    #[serde(flatten)]
    pub extensions: HashMap<String, Value>,
}

/// Temporal tense for linguistic model
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Tense {
    Past,
    Present,
    Future,
}

impl Scroll {
    /// Create a new Scroll with key and data
    pub fn new(key: impl Into<String>, data: Value) -> Self {
        Self {
            key: key.into(),
            type_: "scroll/generic@v1".to_string(),
            metadata: Metadata::default(),
            data,
        }
    }

    /// Create a Scroll with just a key (empty data)
    /// Backwards compat: allows `Scroll::empty("/path").with_data(...)`
    pub fn empty(key: impl Into<String>) -> Self {
        Self::new(key, Value::Null)
    }

    /// Create a Scroll with key, data, and type (constructor)
    pub fn typed(key: impl Into<String>, data: Value, type_: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            type_: type_.into(),
            metadata: Metadata::default(),
            data,
        }
    }

    /// Backwards compat: Scroll::with_schema is now typed()
    #[deprecated(note = "use Scroll::typed() instead")]
    pub fn with_schema(key: impl Into<String>, data: Value, schema: impl Into<String>) -> Self {
        Self::typed(key, data, schema)
    }

    /// Set the type (builder pattern) - backwards compat for .with_type()
    pub fn with_type(mut self, type_: impl Into<String>) -> Self {
        self.type_ = type_.into();
        self
    }

    /// Set the type (builder pattern) - alias for with_type
    pub fn set_type(mut self, type_: impl Into<String>) -> Self {
        self.type_ = type_.into();
        self
    }

    /// Set the data payload (builder pattern)
    pub fn set_data(mut self, data: Value) -> Self {
        self.data = data;
        self
    }

    /// Set the data payload (builder pattern) - alias for set_data
    pub fn with_data(mut self, data: Value) -> Self {
        self.data = data;
        self
    }

    // ========================================================================
    // Linguistic setters
    // ========================================================================

    pub fn with_subject(mut self, subject: impl Into<String>) -> Self {
        self.metadata.subject = Some(subject.into());
        self
    }

    pub fn with_verb(mut self, verb: impl Into<String>) -> Self {
        self.metadata.verb = Some(verb.into());
        self
    }

    pub fn with_object(mut self, object: impl Into<String>) -> Self {
        self.metadata.object = Some(object.into());
        self
    }

    pub fn with_tense(mut self, tense: Tense) -> Self {
        self.metadata.tense = Some(tense);
        self
    }

    // ========================================================================
    // Taxonomic setters
    // ========================================================================

    pub fn with_kingdom(mut self, kingdom: impl Into<String>) -> Self {
        self.metadata.kingdom = Some(kingdom.into());
        self
    }

    pub fn with_phylum(mut self, phylum: impl Into<String>) -> Self {
        self.metadata.phylum = Some(phylum.into());
        self
    }

    pub fn with_class(mut self, class: impl Into<String>) -> Self {
        self.metadata.class = Some(class.into());
        self
    }

    // ========================================================================
    // Extension setter
    // ========================================================================

    /// Add a domain-specific extension
    pub fn with_extension(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.extensions.insert(key.into(), value);
        self
    }

    // ========================================================================
    // Lifecycle setters
    // ========================================================================

    pub fn with_expires_at(mut self, expires_at: impl Into<String>) -> Self {
        self.metadata.expires_at = Some(expires_at.into());
        self
    }

    pub fn mark_deleted(mut self) -> Self {
        self.metadata.deleted = Some(true);
        self
    }

    /// Clear deleted flag (restore)
    pub fn unmark_deleted(mut self) -> Self {
        self.metadata.deleted = None;
        self
    }

    /// Check if scroll is soft-deleted
    pub fn is_deleted(&self) -> bool {
        self.metadata.deleted.unwrap_or(false)
    }

    // ========================================================================
    // Data field accessors (convenience for common patterns)
    // ========================================================================

    /// Get a string field from data
    pub fn get_str(&self, field: &str) -> Option<&str> {
        self.data.get(field).and_then(|v| v.as_str())
    }

    /// Get a string field or default
    pub fn get_str_or<'a>(&'a self, field: &str, default: &'a str) -> &'a str {
        self.get_str(field).unwrap_or(default)
    }

    /// Get an i64 field from data
    pub fn get_i64(&self, field: &str) -> Option<i64> {
        self.data.get(field).and_then(|v| v.as_i64())
    }

    /// Get a bool field from data
    pub fn get_bool(&self, field: &str) -> Option<bool> {
        self.data.get(field).and_then(|v| v.as_bool())
    }

    /// Get a bool extension
    pub fn get_ext_bool(&self, key: &str) -> bool {
        self.metadata.extensions.get(key)
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    /// Get a string extension
    pub fn get_ext_str(&self, key: &str) -> Option<&str> {
        self.metadata.extensions.get(key)
            .and_then(|v| v.as_str())
    }

    // ========================================================================
    // Computed properties
    // ========================================================================

    /// Compute content hash (SHA-256 of key + type + JSON(data))
    pub fn compute_hash(&self) -> String {
        let content = format!(
            "{}{}{}",
            self.key,
            self.type_,
            serde_json::to_string(&self.data).unwrap_or_default()
        );
        let hash = Sha256::digest(content.as_bytes());
        format!("{:x}", hash)
    }

    /// Finalize the scroll with computed hash and timestamps
    /// Called before writing to namespace
    pub fn finalize(mut self) -> Self {
        let now = current_iso_time();

        // Set timestamps
        if self.metadata.created_at.is_none() {
            self.metadata.created_at = Some(now.clone());
        }
        self.metadata.updated_at = Some(now);

        // Compute hash
        self.metadata.hash = Some(self.compute_hash());

        self
    }

    /// Increment version (for updates)
    pub fn increment_version(mut self) -> Self {
        self.metadata.version += 1;
        self
    }
}

// ============================================================================
// Time utilities
// ============================================================================

/// Get current time as Unix milliseconds string
pub fn current_iso_time() -> String {
    current_time_millis().to_string()
}

/// Get current Unix timestamp in milliseconds
pub fn current_time_millis() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

/// Convert Unix milliseconds to string representation
pub fn unix_millis_to_iso(millis: i64) -> String {
    millis.to_string()
}

/// Parse string to Unix milliseconds
pub fn iso_to_unix_millis(iso: &str) -> i64 {
    iso.parse().unwrap_or(current_time_millis())
}

// ============================================================================
// Well-known scroll types
// ============================================================================

/// Standard scroll type constants
pub mod types {
    // Core
    pub const GENERIC: &str = "scroll/generic@v1";
    pub const BLOB: &str = "scroll/blob@v1";
    pub const REF: &str = "scroll/ref@v1";
    pub const ACCESS: &str = "scroll/access@v1";
    pub const NOTIFY: &str = "scroll/notify@v1";
    pub const ACK: &str = "scroll/ack@v1";
    pub const PROCESSOR: &str = "scroll/processor@v1";

    // Vault (encrypted content)
    pub const NOTE: &str = "vault/note@v1";
    pub const SEALED_NOTE: &str = "vault/sealed-note@v1";

    // Contacts (directory of identities)
    pub const CONTACT: &str = "contacts/card@v1";

    // Chat (encrypted messages)
    pub const MESSAGE: &str = "chat/message@v1";
    pub const THREAD: &str = "chat/thread@v1";

    // Wallet
    pub const UTXO: &str = "wallet/utxo@v1";
    pub const TX: &str = "wallet/tx@v1";
    pub const BALANCE: &str = "wallet/balance@v1";

    // Lightning
    pub const LN_STATUS: &str = "ln/status@v1";
    pub const LN_BALANCE: &str = "ln/balance@v1";
    pub const LN_INVOICE: &str = "ln/invoice@v1";
    pub const LN_PAYMENT: &str = "ln/payment@v1";
    pub const LN_CMD: &str = "ln/cmd@v1";
}

/// Standard kingdom constants
pub mod kingdoms {
    pub const FINANCIAL: &str = "financial";
    pub const CONTENT: &str = "content";
    pub const SECURITY: &str = "security";
    pub const SYSTEM: &str = "system";
    pub const DIRECTORY: &str = "directory";
}

/// Standard verb constants
pub mod verbs {
    // CRUD-like
    pub const CREATES: &str = "creates";
    pub const READS: &str = "reads";
    pub const UPDATES: &str = "updates";
    pub const DELETES: &str = "deletes";
    pub const WRITES: &str = "writes";

    // Ownership/transfer
    pub const OWNS: &str = "owns";
    pub const SENDS: &str = "sends";
    pub const RECEIVES: &str = "receives";

    // Communication
    pub const NOTIFIES: &str = "notifies";
    pub const EMITS: &str = "emits";

    // Control
    pub const CONTROLS: &str = "controls";
    pub const EXECUTES: &str = "executes";

    // Security/encryption
    pub const SEALS: &str = "seals";
    pub const UNSEALS: &str = "unseals";

    // Lifecycle
    pub const ENDS: &str = "ends";
    pub const CLEARS: &str = "clears";
    pub const DISCONNECTS: &str = "disconnects";
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn scroll_new() {
        let scroll = Scroll::new("/test", json!({"foo": "bar"}));
        assert_eq!(scroll.key, "/test");
        assert_eq!(scroll.type_, "scroll/generic@v1");
        assert_eq!(scroll.data, json!({"foo": "bar"}));
    }

    #[test]
    fn scroll_typed() {
        let scroll = Scroll::typed("/test", json!({"foo": "bar"}), "test/type@v1");
        assert_eq!(scroll.key, "/test");
        assert_eq!(scroll.type_, "test/type@v1");
    }

    #[test]
    fn scroll_builder() {
        let scroll = Scroll::new("/vault/notes/abc123", json!({"title": "Test"}))
            .set_type("vault/note@v1")
            .with_subject("user:local")
            .with_verb("creates")
            .with_tense(Tense::Past)
            .with_kingdom("content")
            .with_class("note");

        assert_eq!(scroll.key, "/vault/notes/abc123");
        assert_eq!(scroll.type_, "vault/note@v1");
        assert_eq!(scroll.metadata.subject, Some("user:local".to_string()));
        assert_eq!(scroll.metadata.verb, Some("creates".to_string()));
        assert_eq!(scroll.metadata.tense, Some(Tense::Past));
        assert_eq!(scroll.metadata.kingdom, Some("content".to_string()));
        assert_eq!(scroll.metadata.class, Some("note".to_string()));
    }

    #[test]
    fn scroll_finalize() {
        let scroll = Scroll::new("/test", json!({"foo": "bar"}))
            .set_type("test/type@v1")
            .finalize();

        assert!(scroll.metadata.created_at.is_some());
        assert!(scroll.metadata.updated_at.is_some());
        assert!(scroll.metadata.hash.is_some());
    }

    #[test]
    fn scroll_compute_hash() {
        let scroll = Scroll::new("/test", json!({"foo": "bar"}));
        let hash = scroll.compute_hash();
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // SHA-256 hex is 64 chars
    }

    #[test]
    fn scroll_serialize_roundtrip() {
        let scroll = Scroll::new("/vault/notes/abc123", json!({"title": "Test"}))
            .set_type("vault/note@v1")
            .with_subject("user:local")
            .finalize();

        let json = serde_json::to_string(&scroll).unwrap();
        let parsed: Scroll = serde_json::from_str(&json).unwrap();

        assert_eq!(scroll.key, parsed.key);
        assert_eq!(scroll.type_, parsed.type_);
        assert_eq!(scroll.metadata.subject, parsed.metadata.subject);
    }

    #[test]
    fn scroll_extensions() {
        let scroll = Scroll::new("/vault/notes/abc123", json!({}))
            .with_extension("pinned", json!(true))
            .with_extension("folder", json!("work"));

        assert_eq!(scroll.metadata.extensions.get("pinned"), Some(&json!(true)));
        assert_eq!(scroll.metadata.extensions.get("folder"), Some(&json!("work")));
    }
}
