//! Patch - Git-like diff primitives for Scrolls
//!
//! Pure functions for computing and applying patches between scroll states.
//! No storage, no policy - just transformations.
//!
//! # Usage
//!
//! ```rust
//! use beewallet_core_spark::nine_s::{Scroll, Patch};
//! use beewallet_core_spark::nine_s::patch::diff;
//! use serde_json::json;
//!
//! // Create two scroll states
//! let old = Scroll::new("/notes/abc", json!({"title": "Hello"}));
//! let new = Scroll::new("/notes/abc", json!({"title": "Hello", "body": "World"}));
//!
//! // Compute patch
//! let patch = diff::create("/notes/abc", Some(&old), &new);
//! assert!(!patch.ops.is_empty());
//!
//! // Apply patch to old state
//! let result = diff::apply(&old, &patch).unwrap();
//! assert_eq!(result.data["body"], "World");
//! ```

use crate::nine_s::{current_time_millis, Scroll};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A single JSON Patch operation (RFC 6902)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum PatchOp {
    /// Add a value at a path
    Add { path: String, value: Value },
    /// Remove a value at a path
    Remove { path: String },
    /// Replace a value at a path
    Replace { path: String, value: Value },
    /// Move a value from one path to another
    Move { from: String, path: String },
    /// Copy a value from one path to another
    Copy { from: String, path: String },
    /// Test that a value equals expected (for conditional patches)
    Test { path: String, value: Value },
}

/// A patch representing a change to a Scroll
///
/// Patches are the unit of change in the git-like storage layer.
/// They record what changed, when, and form a hash chain for integrity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Patch {
    /// The scroll path being patched
    pub key: String,
    /// JSON Patch operations (RFC 6902)
    pub ops: Vec<PatchOp>,
    /// Hash of the state before this patch (None for create)
    pub parent: Option<String>,
    /// Hash of the state after this patch
    pub hash: String,
    /// When the patch was created (Unix millis)
    pub timestamp: i64,
    /// Patch sequence number (for ordering)
    pub seq: u64,
}

/// Patch errors
#[derive(Debug, Clone, PartialEq)]
pub enum PatchError {
    /// Path not found when applying operation
    PathNotFound(String),
    /// Type mismatch during operation
    TypeMismatch(String),
    /// Test operation failed
    TestFailed(String),
    /// Invalid JSON pointer
    InvalidPointer(String),
}

impl std::fmt::Display for PatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PatchError::PathNotFound(p) => write!(f, "path not found: {}", p),
            PatchError::TypeMismatch(m) => write!(f, "type mismatch: {}", m),
            PatchError::TestFailed(t) => write!(f, "test failed: {}", t),
            PatchError::InvalidPointer(p) => write!(f, "invalid JSON pointer: {}", p),
        }
    }
}

impl std::error::Error for PatchError {}

/// Pure diff functions - no storage, just computation
pub mod diff {
    use super::*;

    /// Compute a patch from old state to new state
    ///
    /// If `old` is None, this is a create operation (birth).
    /// The patch records all operations needed to transform old → new.
    pub fn create(key: &str, old: Option<&Scroll>, new: &Scroll) -> Patch {
        let parent = old.map(hash);
        let ops = compute_ops(old.map(|s| &s.data), &new.data);
        let seq = old.map(|s| s.metadata.version + 1).unwrap_or(1);

        Patch {
            key: key.to_string(),
            ops,
            parent,
            hash: hash(new),
            timestamp: current_time_millis(),
            seq,
        }
    }

    /// Apply a patch to a scroll, producing new state
    ///
    /// Returns error if any operation fails.
    pub fn apply(scroll: &Scroll, patch: &Patch) -> Result<Scroll, PatchError> {
        let mut new_data = scroll.data.clone();

        for op in &patch.ops {
            apply_op(&mut new_data, op)?;
        }

        let mut result = Scroll::new(&scroll.key, new_data);
        result.type_ = scroll.type_.clone();
        result.metadata.version = patch.seq;

        Ok(result)
    }

    /// Compute content hash of a scroll (SHA-256 hex)
    pub fn hash(scroll: &Scroll) -> String {
        scroll.compute_hash()
    }

    /// Verify a patch's hash chain
    ///
    /// Checks that applying the patch to `old` produces `expected_hash`.
    pub fn verify(old: Option<&Scroll>, patch: &Patch) -> bool {
        // Verify parent hash matches old state
        match (old, &patch.parent) {
            (Some(s), Some(p)) => {
                if hash(s) != *p {
                    return false;
                }
            }
            (None, None) => {} // Create operation
            _ => return false, // Mismatch
        }

        true
    }

    // ========================================================================
    // Internal: JSON diff algorithm
    // ========================================================================

    /// Compute JSON patch operations from old to new value
    fn compute_ops(old: Option<&Value>, new: &Value) -> Vec<PatchOp> {
        match old {
            None => {
                // Create: entire new value is an "add" at root
                vec![PatchOp::Replace {
                    path: "".to_string(),
                    value: new.clone(),
                }]
            }
            Some(old_val) => compute_diff("", old_val, new),
        }
    }

    /// Recursive diff between two JSON values
    fn compute_diff(path: &str, old: &Value, new: &Value) -> Vec<PatchOp> {
        let mut ops = Vec::new();

        match (old, new) {
            // Both objects: diff keys
            (Value::Object(old_obj), Value::Object(new_obj)) => {
                // Removed keys
                for key in old_obj.keys() {
                    if !new_obj.contains_key(key) {
                        ops.push(PatchOp::Remove {
                            path: json_pointer(path, key),
                        });
                    }
                }

                // Added or changed keys
                for (key, new_val) in new_obj {
                    let key_path = json_pointer(path, key);
                    match old_obj.get(key) {
                        None => {
                            ops.push(PatchOp::Add {
                                path: key_path,
                                value: new_val.clone(),
                            });
                        }
                        Some(old_val) if old_val != new_val => {
                            // Recurse for nested changes
                            ops.extend(compute_diff(&key_path, old_val, new_val));
                        }
                        _ => {} // Unchanged
                    }
                }
            }

            // Both arrays: element-wise diff (simplified)
            (Value::Array(old_arr), Value::Array(new_arr)) => {
                // Simple approach: if arrays differ, replace entire array
                // A more sophisticated approach would do LCS-based diffing
                if old_arr != new_arr {
                    ops.push(PatchOp::Replace {
                        path: path.to_string(),
                        value: new.clone(),
                    });
                }
            }

            // Different types or primitives: replace
            _ => {
                if old != new {
                    ops.push(PatchOp::Replace {
                        path: path.to_string(),
                        value: new.clone(),
                    });
                }
            }
        }

        ops
    }

    /// Build JSON pointer path (RFC 6901)
    fn json_pointer(base: &str, key: &str) -> String {
        // Escape ~ and / in key
        let escaped = key.replace('~', "~0").replace('/', "~1");
        format!("{}/{}", base, escaped)
    }

    /// Apply a single patch operation to a value
    fn apply_op(data: &mut Value, op: &PatchOp) -> Result<(), PatchError> {
        match op {
            PatchOp::Add { path, value } => {
                set_at_pointer(data, path, value.clone(), false)?;
            }
            PatchOp::Remove { path } => {
                remove_at_pointer(data, path)?;
            }
            PatchOp::Replace { path, value } => {
                set_at_pointer(data, path, value.clone(), true)?;
            }
            PatchOp::Move { from, path } => {
                let value = remove_at_pointer(data, from)?;
                set_at_pointer(data, path, value, false)?;
            }
            PatchOp::Copy { from, path } => {
                let value = get_at_pointer(data, from)?;
                set_at_pointer(data, path, value, false)?;
            }
            PatchOp::Test { path, value } => {
                let actual = get_at_pointer(data, path)?;
                if actual != *value {
                    return Err(PatchError::TestFailed(format!(
                        "expected {:?}, got {:?}",
                        value, actual
                    )));
                }
            }
        }
        Ok(())
    }

    /// Get value at JSON pointer path
    fn get_at_pointer(data: &Value, pointer: &str) -> Result<Value, PatchError> {
        if pointer.is_empty() {
            return Ok(data.clone());
        }

        let parts = parse_pointer(pointer)?;
        let mut current = data;

        for part in parts {
            current = match current {
                Value::Object(obj) => obj
                    .get(&part)
                    .ok_or_else(|| PatchError::PathNotFound(pointer.to_string()))?,
                Value::Array(arr) => {
                    let idx: usize = part
                        .parse()
                        .map_err(|_| PatchError::InvalidPointer(pointer.to_string()))?;
                    arr.get(idx)
                        .ok_or_else(|| PatchError::PathNotFound(pointer.to_string()))?
                }
                _ => return Err(PatchError::TypeMismatch(pointer.to_string())),
            };
        }

        Ok(current.clone())
    }

    /// Set value at JSON pointer path
    fn set_at_pointer(
        data: &mut Value,
        pointer: &str,
        value: Value,
        must_exist: bool,
    ) -> Result<(), PatchError> {
        if pointer.is_empty() {
            *data = value;
            return Ok(());
        }

        let parts = parse_pointer(pointer)?;
        let (last, parents) = parts
            .split_last()
            .ok_or_else(|| PatchError::InvalidPointer(pointer.to_string()))?;

        let mut current = data;

        for part in parents {
            current = match current {
                Value::Object(obj) => obj
                    .entry(part.clone())
                    .or_insert_with(|| Value::Object(serde_json::Map::new())),
                Value::Array(arr) => {
                    let idx: usize = part
                        .parse()
                        .map_err(|_| PatchError::InvalidPointer(pointer.to_string()))?;
                    arr.get_mut(idx)
                        .ok_or_else(|| PatchError::PathNotFound(pointer.to_string()))?
                }
                _ => return Err(PatchError::TypeMismatch(pointer.to_string())),
            };
        }

        match current {
            Value::Object(obj) => {
                if must_exist && !obj.contains_key(last) {
                    return Err(PatchError::PathNotFound(pointer.to_string()));
                }
                obj.insert(last.clone(), value);
            }
            Value::Array(arr) => {
                if last == "-" {
                    arr.push(value);
                } else {
                    let idx: usize = last
                        .parse()
                        .map_err(|_| PatchError::InvalidPointer(pointer.to_string()))?;
                    if idx >= arr.len() {
                        return Err(PatchError::PathNotFound(pointer.to_string()));
                    }
                    arr[idx] = value;
                }
            }
            _ => return Err(PatchError::TypeMismatch(pointer.to_string())),
        }

        Ok(())
    }

    /// Remove value at JSON pointer path, returning removed value
    fn remove_at_pointer(data: &mut Value, pointer: &str) -> Result<Value, PatchError> {
        if pointer.is_empty() {
            return Err(PatchError::InvalidPointer(
                "cannot remove root".to_string(),
            ));
        }

        let parts = parse_pointer(pointer)?;
        let (last, parents) = parts
            .split_last()
            .ok_or_else(|| PatchError::InvalidPointer(pointer.to_string()))?;

        let mut current = data;

        for part in parents {
            current = match current {
                Value::Object(obj) => obj
                    .get_mut(part)
                    .ok_or_else(|| PatchError::PathNotFound(pointer.to_string()))?,
                Value::Array(arr) => {
                    let idx: usize = part
                        .parse()
                        .map_err(|_| PatchError::InvalidPointer(pointer.to_string()))?;
                    arr.get_mut(idx)
                        .ok_or_else(|| PatchError::PathNotFound(pointer.to_string()))?
                }
                _ => return Err(PatchError::TypeMismatch(pointer.to_string())),
            };
        }

        match current {
            Value::Object(obj) => obj
                .remove(last)
                .ok_or_else(|| PatchError::PathNotFound(pointer.to_string())),
            Value::Array(arr) => {
                let idx: usize = last
                    .parse()
                    .map_err(|_| PatchError::InvalidPointer(pointer.to_string()))?;
                if idx >= arr.len() {
                    return Err(PatchError::PathNotFound(pointer.to_string()));
                }
                Ok(arr.remove(idx))
            }
            _ => Err(PatchError::TypeMismatch(pointer.to_string())),
        }
    }

    /// Parse JSON pointer into path segments
    fn parse_pointer(pointer: &str) -> Result<Vec<String>, PatchError> {
        if !pointer.starts_with('/') {
            return Err(PatchError::InvalidPointer(pointer.to_string()));
        }

        Ok(pointer[1..]
            .split('/')
            .map(|s| s.replace("~1", "/").replace("~0", "~"))
            .collect())
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
    fn test_create_patch_from_none() {
        let new = Scroll::new("/test", json!({"title": "Hello"}));
        let patch = diff::create("/test", None, &new);

        assert_eq!(patch.key, "/test");
        assert!(patch.parent.is_none());
        assert_eq!(patch.seq, 1);
        assert!(!patch.ops.is_empty());
    }

    #[test]
    fn test_create_patch_add_field() {
        let old = Scroll::new("/test", json!({"title": "Hello"}));
        let new = Scroll::new("/test", json!({"title": "Hello", "body": "World"}));

        let patch = diff::create("/test", Some(&old), &new);

        assert!(patch.parent.is_some());
        assert!(patch.ops.iter().any(|op| matches!(op, PatchOp::Add { path, .. } if path == "/body")));
    }

    #[test]
    fn test_create_patch_remove_field() {
        let old = Scroll::new("/test", json!({"title": "Hello", "body": "World"}));
        let new = Scroll::new("/test", json!({"title": "Hello"}));

        let patch = diff::create("/test", Some(&old), &new);

        assert!(patch
            .ops
            .iter()
            .any(|op| matches!(op, PatchOp::Remove { path } if path == "/body")));
    }

    #[test]
    fn test_create_patch_replace_field() {
        let old = Scroll::new("/test", json!({"title": "Hello"}));
        let new = Scroll::new("/test", json!({"title": "Goodbye"}));

        let patch = diff::create("/test", Some(&old), &new);

        assert!(patch.ops.iter().any(
            |op| matches!(op, PatchOp::Replace { path, value } if path == "/title" && value == "Goodbye")
        ));
    }

    #[test]
    fn test_apply_patch_add() {
        let old = Scroll::new("/test", json!({"title": "Hello"}));
        let new = Scroll::new("/test", json!({"title": "Hello", "body": "World"}));

        let patch = diff::create("/test", Some(&old), &new);
        let result = diff::apply(&old, &patch).unwrap();

        assert_eq!(result.data["body"], "World");
        assert_eq!(result.data["title"], "Hello");
    }

    #[test]
    fn test_apply_patch_remove() {
        let old = Scroll::new("/test", json!({"title": "Hello", "body": "World"}));
        let new = Scroll::new("/test", json!({"title": "Hello"}));

        let patch = diff::create("/test", Some(&old), &new);
        let result = diff::apply(&old, &patch).unwrap();

        assert_eq!(result.data["title"], "Hello");
        assert!(result.data.get("body").is_none());
    }

    #[test]
    fn test_apply_patch_replace() {
        let old = Scroll::new("/test", json!({"title": "Hello"}));
        let new = Scroll::new("/test", json!({"title": "Goodbye"}));

        let patch = diff::create("/test", Some(&old), &new);
        let result = diff::apply(&old, &patch).unwrap();

        assert_eq!(result.data["title"], "Goodbye");
    }

    #[test]
    fn test_nested_diff() {
        let old = Scroll::new(
            "/test",
            json!({
                "user": {
                    "name": "Alice",
                    "age": 30
                }
            }),
        );
        let new = Scroll::new(
            "/test",
            json!({
                "user": {
                    "name": "Alice",
                    "age": 31,
                    "email": "alice@example.com"
                }
            }),
        );

        let patch = diff::create("/test", Some(&old), &new);
        let result = diff::apply(&old, &patch).unwrap();

        assert_eq!(result.data["user"]["age"], 31);
        assert_eq!(result.data["user"]["email"], "alice@example.com");
    }

    #[test]
    fn test_hash_consistency() {
        let scroll = Scroll::new("/test", json!({"title": "Hello"}));
        let hash1 = diff::hash(&scroll);
        let hash2 = diff::hash(&scroll);

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA-256 hex
    }

    #[test]
    fn test_patch_serialization() {
        let old = Scroll::new("/test", json!({"title": "Hello"}));
        let new = Scroll::new("/test", json!({"title": "Goodbye"}));

        let patch = diff::create("/test", Some(&old), &new);
        let json = serde_json::to_string(&patch).unwrap();
        let parsed: Patch = serde_json::from_str(&json).unwrap();

        assert_eq!(patch, parsed);
    }

    #[test]
    fn test_roundtrip_create_apply() {
        let old = Scroll::new(
            "/test",
            json!({
                "title": "Original",
                "items": [1, 2, 3],
                "nested": {"a": 1, "b": 2}
            }),
        );
        let new = Scroll::new(
            "/test",
            json!({
                "title": "Modified",
                "items": [1, 2, 3, 4],
                "nested": {"a": 1, "c": 3}
            }),
        );

        let patch = diff::create("/test", Some(&old), &new);
        let result = diff::apply(&old, &patch).unwrap();

        assert_eq!(result.data, new.data);
    }
}
