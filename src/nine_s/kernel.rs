//! Kernel - Namespace composition via mount table
//!
//! "Everything is a file" → "Everything is a Scroll"
//! Plan 9 mount table for namespace composition.
//! Longest prefix match routing with segment boundary checks.
//!
//! # Security
//! - Mount paths are matched on segment boundaries (no cross-namespace leaks)
//! - `/foo` does NOT match `/foobar` (only `/foo` or `/foo/...`)

#[cfg(feature = "std-channel")]
use super::channel::{channel, Receiver};
#[cfg(not(feature = "std-channel"))]
use super::namespace::Receiver;

use super::namespace::{Error, Namespace, Result};
use super::scroll::Scroll;
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

/// Check if a path matches a mount point on segment boundaries.
///
/// # Security
/// This prevents cross-namespace leaks where mounting `/foo` would
/// incorrectly capture `/foobar`.
///
/// Returns true if:
/// - path == mount_path (exact match)
/// - path starts with mount_path followed by '/' (proper child)
/// - mount_path == "/" (root mount matches everything)
fn is_path_under_mount(path: &str, mount_path: &str) -> bool {
    if mount_path == "/" {
        return path.starts_with('/');
    }

    if path == mount_path {
        return true;
    }

    // Check for segment boundary: path must continue with '/'
    if path.starts_with(mount_path) {
        let remainder = &path[mount_path.len()..];
        return remainder.starts_with('/');
    }

    false
}

/// Normalize a mount path for storage
///
/// - Ensures path starts with '/'
/// - Removes trailing slashes (except for root "/")
fn normalize_mount_path(path: &str) -> String {
    let mut normalized = path.to_string();

    // Ensure starts with /
    if !normalized.starts_with('/') {
        normalized = format!("/{}", normalized);
    }

    // Remove trailing slashes (except for root)
    while normalized.len() > 1 && normalized.ends_with('/') {
        normalized.pop();
    }

    normalized
}

/// Kernel - Namespace composition via mount table
///
/// Mount namespaces at paths. Operations are routed by longest prefix match.
pub struct Kernel {
    mounts: Arc<RwLock<BTreeMap<String, Arc<dyn Namespace>>>>,
}

impl Kernel {
    /// Create a new kernel with empty mount table
    pub fn new() -> Self {
        Self {
            mounts: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }

    /// Mount a namespace at a path
    ///
    /// All operations on paths starting with `path` are routed to `ns`.
    /// The namespace sees paths with `path` prefix stripped.
    ///
    /// # Path Normalization
    /// - Paths are normalized (leading `/` added, trailing `/` removed)
    /// - `/foo/` becomes `/foo`
    pub fn mount(&self, path: impl Into<String>, ns: Arc<dyn Namespace>) {
        let path = normalize_mount_path(&path.into());
        self.mounts.write().unwrap().insert(path, ns);
    }

    /// Mount a namespace (convenience method that wraps in Arc)
    pub fn mount_box(&self, path: impl Into<String>, ns: impl Namespace + 'static) {
        self.mount(path, Arc::new(ns));
    }

    /// Unmount a namespace from a path
    ///
    /// Returns the previously mounted namespace, or None if nothing was mounted.
    pub fn unmount(&self, path: &str) -> Option<Arc<dyn Namespace>> {
        self.mounts.write().unwrap().remove(path)
    }

    /// Find the namespace and translated path for a given path
    ///
    /// Uses longest prefix match with segment boundary checking.
    ///
    /// # Security
    /// - Only matches on segment boundaries (`/foo` matches `/foo/bar` but NOT `/foobar`)
    /// - Prevents cross-namespace data leakage
    fn resolve(&self, path: &str) -> Result<(Arc<dyn Namespace>, String)> {
        let mounts = self.mounts.read().unwrap();

        // Find longest matching prefix with segment boundary check
        let mut best_match: Option<(&String, &Arc<dyn Namespace>)> = None;

        for (mount_path, ns) in mounts.iter() {
            // Use segment-boundary-aware matching
            if is_path_under_mount(path, mount_path) {
                if let Some((current_best, _)) = best_match {
                    if mount_path.len() > current_best.len() {
                        best_match = Some((mount_path, ns));
                    }
                } else {
                    best_match = Some((mount_path, ns));
                }
            }
        }

        match best_match {
            Some((mount_path, ns)) => {
                // Strip prefix
                let stripped = if mount_path == "/" {
                    path.to_string()
                } else if path == mount_path {
                    "/".to_string()
                } else {
                    path[mount_path.len()..].to_string()
                };

                Ok((ns.clone(), stripped))
            }
            None => Err(Error::NotFound(format!(
                "no namespace mounted for path: {}",
                path
            ))),
        }
    }
}

impl Default for Kernel {
    fn default() -> Self {
        Self::new()
    }
}

impl Namespace for Kernel {
    fn read(&self, path: &str) -> Result<Option<Scroll>> {
        let (ns, stripped) = self.resolve(path)?;
        let scroll = ns.read(&stripped)?;

        // Restore original path if scroll exists
        Ok(scroll.map(|mut s| {
            s.key = path.to_string();
            s
        }))
    }

    fn write(&self, path: &str, data: Value) -> Result<Scroll> {
        let (ns, stripped) = self.resolve(path)?;
        let mut scroll = ns.write(&stripped, data)?;

        // Restore original path
        scroll.key = path.to_string();
        Ok(scroll)
    }

    fn write_scroll(&self, scroll: Scroll) -> Result<Scroll> {
        let (ns, stripped) = self.resolve(&scroll.key)?;

        // Create scroll with stripped path for the namespace
        let mut stripped_scroll = scroll.clone();
        stripped_scroll.key = stripped;

        let mut result = ns.write_scroll(stripped_scroll)?;

        // Restore original path
        result.key = scroll.key;
        Ok(result)
    }

    fn list(&self, prefix: &str) -> Result<Vec<String>> {
        let (ns, stripped) = self.resolve(prefix)?;
        let paths = ns.list(&stripped)?;

        // Restore mount prefix to paths
        let mount_prefix = if prefix == "/" {
            "".to_string()
        } else {
            let stripped_len = stripped.len();
            if prefix.len() >= stripped_len {
                prefix[..prefix.len() - stripped_len].to_string()
            } else {
                "".to_string()
            }
        };

        Ok(paths
            .into_iter()
            .map(|p| {
                if mount_prefix.is_empty() {
                    p
                } else if p == "/" {
                    mount_prefix.clone()
                } else {
                    format!("{}{}", mount_prefix, p)
                }
            })
            .collect())
    }

    fn watch(&self, pattern: &str) -> Result<Receiver<Scroll>> {
        let (ns, stripped) = self.resolve(pattern)?;
        let mut rx = ns.watch(&stripped)?;

        // Create new channel to translate paths back
        let (tx, output_rx) = channel(16);
        let pattern_owned = pattern.to_string();
        let stripped_owned = stripped.clone();

        // Spawn thread to translate scroll keys
        std::thread::spawn(move || {
            while let Some(mut scroll) = rx.recv() {
                // Restore original path prefix
                if stripped_owned == "/" {
                    scroll.key = format!(
                        "{}{}",
                        pattern_owned.trim_end_matches("/**").trim_end_matches("/*"),
                        scroll.key
                    );
                } else {
                    let mount_prefix =
                        &pattern_owned[..pattern_owned.len() - stripped_owned.len()];
                    scroll.key = if scroll.key == "/" {
                        mount_prefix.to_string()
                    } else {
                        format!("{}{}", mount_prefix, scroll.key)
                    };
                }

                if tx.send(scroll).is_err() {
                    break; // Receiver dropped
                }
            }
        });

        Ok(output_rx)
    }

    fn close(&self) -> Result<()> {
        let mounts = self.mounts.read().unwrap();
        for ns in mounts.values() {
            let _ = ns.close();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kernel_new() {
        let kernel = Kernel::new();
        let result = kernel.read("/test");
        assert!(matches!(result, Err(Error::NotFound(_))));
    }

    #[test]
    fn test_is_path_under_mount_exact() {
        assert!(is_path_under_mount("/foo", "/foo"));
        assert!(is_path_under_mount("/foo/bar", "/foo/bar"));
    }

    #[test]
    fn test_is_path_under_mount_child() {
        assert!(is_path_under_mount("/foo/bar", "/foo"));
        assert!(is_path_under_mount("/foo/bar/baz", "/foo"));
        assert!(is_path_under_mount("/foo/bar/baz", "/foo/bar"));
    }

    #[test]
    fn test_is_path_under_mount_root() {
        assert!(is_path_under_mount("/foo", "/"));
        assert!(is_path_under_mount("/foo/bar", "/"));
        assert!(is_path_under_mount("/", "/"));
    }

    #[test]
    fn test_is_path_under_mount_boundary_security() {
        // CRITICAL SECURITY TEST: /foo should NOT match /foobar
        assert!(!is_path_under_mount("/foobar", "/foo"));
        assert!(!is_path_under_mount("/foobar/baz", "/foo"));

        // But /foo/bar should still match /foo
        assert!(is_path_under_mount("/foo/bar", "/foo"));
    }

    #[test]
    fn test_is_path_under_mount_no_match() {
        assert!(!is_path_under_mount("/bar", "/foo"));
        assert!(!is_path_under_mount("/bar/baz", "/foo"));
    }

    #[test]
    fn test_normalize_mount_path() {
        assert_eq!(normalize_mount_path("/foo"), "/foo");
        assert_eq!(normalize_mount_path("/foo/"), "/foo");
        assert_eq!(normalize_mount_path("/foo//"), "/foo");
        assert_eq!(normalize_mount_path("/"), "/");
        assert_eq!(normalize_mount_path("foo"), "/foo");
        assert_eq!(normalize_mount_path("foo/"), "/foo");
    }

    // Integration tests with MemoryNamespace in backends/memory.rs
}
