//! Namespace - The universal interface
//!
//! Five operations. Frozen. Never a sixth.
//!
//! Ken Thompson: "Simplicity is the ultimate sophistication."
//! SICP: "Data abstraction - Use ≠ Representation"

#[cfg(feature = "std-channel")]
pub use super::channel::Receiver;

use super::scroll::Scroll;
use serde_json::Value;
use thiserror::Error;

/// Stub receiver for WASM builds (watch not supported)
#[cfg(not(feature = "std-channel"))]
pub struct Receiver<T>(std::marker::PhantomData<T>);

#[cfg(not(feature = "std-channel"))]
impl<T> Receiver<T> {
    /// Always returns None in WASM builds
    pub fn recv(&mut self) -> Option<T> {
        None
    }
}

/// Error types following 9S protocol specification
#[derive(Debug, Error)]
pub enum Error {
    #[error("path not found: {0}")]
    NotFound(String),

    #[error("invalid path: {0}")]
    InvalidPath(String),

    #[error("invalid data: {0}")]
    InvalidData(String),

    #[error("permission denied: {0}")]
    Permission(String),

    #[error("namespace is closed")]
    Closed,

    #[error("operation timed out")]
    Timeout,

    #[error("connection error: {0}")]
    Connection(String),

    #[error("service unavailable: {0}")]
    Unavailable(String),

    #[error("internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Namespace - The 5 frozen operations
///
/// All functionality in 9S emerges from these five operations.
/// Extensions are new Namespace implementations, not new operations.
pub trait Namespace: Send + Sync {
    /// Read a scroll by path
    ///
    /// Returns None if path doesn't exist (NOT an error)
    /// Returns Error on failure (permission, I/O, connection, etc.)
    fn read(&self, path: &str) -> Result<Option<Scroll>>;

    /// Write data at a path
    ///
    /// Creates or updates the Scroll at path.
    /// Returns the written Scroll with computed meta (hash, version, time).
    fn write(&self, path: &str, data: Value) -> Result<Scroll>;

    /// Write a full scroll (with schema hint)
    ///
    /// The schema in scroll.meta is preserved.
    /// Hash, version, and time are computed by the namespace.
    fn write_scroll(&self, scroll: Scroll) -> Result<Scroll> {
        // Default implementation - can be overridden
        self.write(&scroll.key, scroll.data)
    }

    /// List all paths under a prefix
    ///
    /// Returns empty vec if no matches (NOT an error)
    fn list(&self, prefix: &str) -> Result<Vec<String>>;

    /// Watch for changes matching a pattern
    ///
    /// Returns a channel that receives Scrolls when matching paths change.
    /// Supports glob patterns: * (single segment), ** (any suffix)
    fn watch(&self, pattern: &str) -> Result<Receiver<Scroll>>;

    /// Close the namespace and release resources
    ///
    /// Cancels all active watches (channels close).
    /// Subsequent operations return Error::Closed.
    /// Idempotent - safe to call multiple times.
    fn close(&self) -> Result<()>;
}

/// Validate path syntax per 9S spec
///
/// # Security
/// - Rejects path traversal attempts (`..` and `.` segments)
/// - Only allows alphanumeric, underscore, hyphen, and dot (within names)
/// - Glob wildcards (`*`) only allowed at end of path for watch patterns
pub fn validate_path(path: &str) -> Result<()> {
    if path.is_empty() {
        return Err(Error::InvalidPath("path cannot be empty".to_string()));
    }

    if !path.starts_with('/') {
        return Err(Error::InvalidPath("path must start with /".to_string()));
    }

    // Check for valid characters and reject path traversal
    for segment in path.split('/').skip(1) {
        if segment.is_empty() && path != "/" {
            continue; // Allow trailing slash
        }

        // SECURITY: Reject path traversal attempts
        if segment == "." || segment == ".." {
            return Err(Error::InvalidPath(
                "path traversal not allowed (. or .. segments)".to_string(),
            ));
        }

        for c in segment.chars() {
            if !c.is_alphanumeric() && c != '_' && c != '.' && c != '-' && c != '*' {
                return Err(Error::InvalidPath(format!(
                    "invalid character '{}' in path",
                    c
                )));
            }
        }
    }

    Ok(())
}

/// Check if a path matches a pattern (supports * and **)
pub fn path_matches(path: &str, pattern: &str) -> bool {
    // Exact match
    if path == pattern {
        return true;
    }

    // Single wildcard: /foo/* matches /foo/bar but not /foo/bar/baz
    if pattern.ends_with("/*") {
        let prefix = &pattern[..pattern.len() - 1];
        if path.starts_with(prefix) {
            let remainder = &path[prefix.len()..];
            // Should not contain any more slashes
            return !remainder.contains('/');
        }
        return false;
    }

    // Recursive wildcard: /foo/** matches /foo/bar, /foo/bar/baz, etc.
    if pattern.ends_with("/**") {
        let prefix = &pattern[..pattern.len() - 2];
        return path.starts_with(prefix);
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_path_valid() {
        assert!(validate_path("/").is_ok());
        assert!(validate_path("/test").is_ok());
        assert!(validate_path("/foo/bar").is_ok());
        assert!(validate_path("/foo/bar/baz").is_ok());
        assert!(validate_path("/foo_bar").is_ok());
        assert!(validate_path("/foo-bar").is_ok());
        assert!(validate_path("/foo.bar").is_ok());
    }

    #[test]
    fn validate_path_invalid() {
        assert!(validate_path("").is_err());
        assert!(validate_path("foo").is_err());
        assert!(validate_path("foo/bar").is_err());
    }

    #[test]
    fn validate_path_traversal_security() {
        // SECURITY: These must all fail to prevent path traversal attacks
        assert!(validate_path("/..").is_err(), "Single .. must be rejected");
        assert!(validate_path("/../etc").is_err(), "Leading .. must be rejected");
        assert!(validate_path("/foo/..").is_err(), "Trailing .. must be rejected");
        assert!(validate_path("/foo/../bar").is_err(), "Mid-path .. must be rejected");
        assert!(validate_path("/foo/../../etc/passwd").is_err(), "Multiple .. must be rejected");
        assert!(validate_path("/.").is_err(), "Single . must be rejected");
        assert!(validate_path("/./foo").is_err(), "Leading . must be rejected");
        assert!(validate_path("/foo/.").is_err(), "Trailing . must be rejected");
        assert!(validate_path("/foo/./bar").is_err(), "Mid-path . must be rejected");

        // These should still work (dots in names, not as segments)
        assert!(validate_path("/foo.bar").is_ok(), "Dots in names allowed");
        assert!(validate_path("/foo.bar.baz").is_ok(), "Multiple dots in names allowed");
        assert!(validate_path("/.hidden").is_ok(), "Leading dot in name allowed");
        assert!(validate_path("/foo/.hidden").is_ok(), "Hidden file names allowed");
    }

    #[test]
    fn path_matches_exact() {
        assert!(path_matches("/foo", "/foo"));
        assert!(!path_matches("/foo", "/bar"));
    }

    #[test]
    fn path_matches_wildcard() {
        assert!(path_matches("/foo/bar", "/foo/*"));
        assert!(!path_matches("/foo/bar/baz", "/foo/*"));
        assert!(!path_matches("/foo", "/foo/*"));
    }

    #[test]
    fn path_matches_recursive() {
        assert!(path_matches("/foo/bar", "/foo/**"));
        assert!(path_matches("/foo/bar/baz", "/foo/**"));
        assert!(path_matches("/foo/bar/baz/qux", "/foo/**"));
        assert!(!path_matches("/bar/foo", "/foo/**"));
    }
}
