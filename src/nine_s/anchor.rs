//! Anchor - Immutable checkpoints for Scrolls
//!
//! An anchor is a snapshot of a scroll's state at a point in time.
//! Anchors are immutable - once created, they cannot be modified.
//!
//! # Usage
//!
//! ```rust
//! use beewallet_core_spark::nine_s::{Scroll, Anchor};
//! use beewallet_core_spark::nine_s::anchor;
//! use serde_json::json;
//!
//! // Create a scroll
//! let scroll = Scroll::new("/notes/abc", json!({"title": "Important"}));
//!
//! // Create an anchor (checkpoint)
//! let anchor = anchor::create(&scroll, Some("v1.0"));
//!
//! // Verify integrity
//! assert!(anchor::verify(&anchor));
//!
//! // The anchor contains the full scroll state
//! assert_eq!(anchor.scroll.data["title"], "Important");
//! ```

use crate::nine_s::{current_time_millis, Scroll};
use serde::{Deserialize, Serialize};

/// An immutable checkpoint of a scroll's state
///
/// Anchors freeze a scroll at a specific point in time.
/// They include the full scroll content and a hash for integrity verification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Anchor {
    /// Unique anchor ID (hash prefix + timestamp)
    pub id: String,
    /// The full scroll state at this point
    pub scroll: Scroll,
    /// Content hash (SHA-256 hex) for verification
    pub hash: String,
    /// When the anchor was created (Unix millis)
    pub timestamp: i64,
    /// Optional human-readable label
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Create an anchor from a scroll
///
/// The anchor captures the scroll's current state immutably.
/// An optional label can be provided for human reference.
pub fn create(scroll: &Scroll, label: Option<&str>) -> Anchor {
    use rand::Rng;

    let hash = scroll.compute_hash();
    let timestamp = current_time_millis();
    // Add random suffix for uniqueness when creating multiple anchors in same millisecond
    let suffix: u16 = rand::thread_rng().gen();
    let id = format!("{}-{}-{:04x}", &hash[..8], timestamp, suffix);

    Anchor {
        id,
        scroll: scroll.clone(),
        hash,
        timestamp,
        label: label.map(String::from),
        description: None,
    }
}

/// Create an anchor with a description
pub fn create_with_description(scroll: &Scroll, label: Option<&str>, description: &str) -> Anchor {
    let mut anchor = create(scroll, label);
    anchor.description = Some(description.to_string());
    anchor
}

/// Verify an anchor's integrity
///
/// Returns true if the scroll content matches the stored hash.
pub fn verify(anchor: &Anchor) -> bool {
    anchor.scroll.compute_hash() == anchor.hash
}

/// Check if two anchors represent the same state
///
/// Two anchors are equivalent if their content hashes match,
/// even if they have different IDs or timestamps.
pub fn equivalent(a: &Anchor, b: &Anchor) -> bool {
    a.hash == b.hash
}

/// Extract just the scroll from an anchor
///
/// Useful when restoring from a checkpoint.
pub fn extract(anchor: &Anchor) -> Scroll {
    anchor.scroll.clone()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_create_anchor() {
        let scroll = Scroll::new("/notes/abc", json!({"title": "Test"}));
        let anchor = create(&scroll, Some("v1.0"));

        assert!(!anchor.id.is_empty());
        assert_eq!(anchor.label, Some("v1.0".to_string()));
        assert_eq!(anchor.scroll.data["title"], "Test");
        assert!(!anchor.hash.is_empty());
    }

    #[test]
    fn test_create_anchor_no_label() {
        let scroll = Scroll::new("/notes/abc", json!({"title": "Test"}));
        let anchor = create(&scroll, None);

        assert!(anchor.label.is_none());
    }

    #[test]
    fn test_verify_anchor() {
        let scroll = Scroll::new("/notes/abc", json!({"title": "Test"}));
        let anchor = create(&scroll, None);

        assert!(verify(&anchor));
    }

    #[test]
    fn test_verify_tampered_anchor() {
        let scroll = Scroll::new("/notes/abc", json!({"title": "Test"}));
        let mut anchor = create(&scroll, None);

        // Tamper with the scroll
        anchor.scroll.data = json!({"title": "Tampered"});

        // Verification should fail
        assert!(!verify(&anchor));
    }

    #[test]
    fn test_equivalent_anchors() {
        let scroll = Scroll::new("/notes/abc", json!({"title": "Test"}));

        let anchor1 = create(&scroll, Some("first"));
        // Sleep would change timestamp, but hash should be same
        let anchor2 = Anchor {
            id: "different-id".to_string(),
            scroll: scroll.clone(),
            hash: scroll.compute_hash(),
            timestamp: anchor1.timestamp + 1000,
            label: Some("second".to_string()),
            description: None,
        };

        assert!(equivalent(&anchor1, &anchor2));
    }

    #[test]
    fn test_non_equivalent_anchors() {
        let scroll1 = Scroll::new("/notes/abc", json!({"title": "Test1"}));
        let scroll2 = Scroll::new("/notes/abc", json!({"title": "Test2"}));

        let anchor1 = create(&scroll1, None);
        let anchor2 = create(&scroll2, None);

        assert!(!equivalent(&anchor1, &anchor2));
    }

    #[test]
    fn test_extract_scroll() {
        let scroll = Scroll::new("/notes/abc", json!({"title": "Test"}));
        let anchor = create(&scroll, None);

        let extracted = extract(&anchor);
        assert_eq!(extracted.data, scroll.data);
        assert_eq!(extracted.key, scroll.key);
    }

    #[test]
    fn test_anchor_serialization() {
        let scroll = Scroll::new("/notes/abc", json!({"title": "Test"}));
        let anchor = create(&scroll, Some("v1.0"));

        let json = serde_json::to_string(&anchor).unwrap();
        let parsed: Anchor = serde_json::from_str(&json).unwrap();

        assert_eq!(anchor, parsed);
    }

    #[test]
    fn test_anchor_id_format() {
        let scroll = Scroll::new("/notes/abc", json!({"title": "Test"}));
        let anchor = create(&scroll, None);

        // ID should be hash_prefix-timestamp-random
        assert!(anchor.id.contains('-'));
        let parts: Vec<&str> = anchor.id.split('-').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0].len(), 8); // 8-char hash prefix
        assert_eq!(parts[2].len(), 4); // 4-char hex random suffix
    }

    #[test]
    fn test_create_with_description() {
        let scroll = Scroll::new("/notes/abc", json!({"title": "Test"}));
        let anchor = create_with_description(&scroll, Some("v1.0"), "Initial release");

        assert_eq!(anchor.label, Some("v1.0".to_string()));
        assert_eq!(anchor.description, Some("Initial release".to_string()));
    }
}
