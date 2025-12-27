//! FileNamespace - Persistent 9S backend using filesystem
//!
//! Each scroll is stored as a JSON file at its path.
//! Simple, portable, no dependencies beyond std.
//!
//! # Layout
//! ```text
//! base_dir/
//!   _scrolls/
//!     foo/
//!       bar.json      <- /foo/bar scroll
//!       baz.json      <- /foo/baz scroll
//!     root.json       <- / scroll (if written)
//! ```
//!
//! # Security
//! - Path traversal prevented (no .. allowed)
//! - Segment boundary matching (same as MemoryNamespace)

use super::super::namespace::{path_matches, validate_path, Error, Namespace, Receiver, Result};
use super::super::scroll::{current_iso_time, Scroll};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

#[cfg(feature = "std-channel")]
use super::super::channel::{channel, Sender};

const MAX_WATCHERS: usize = 1024;

/// Check if a path is under a prefix on segment boundaries.
fn is_path_under_prefix(path: &str, prefix: &str) -> bool {
    if prefix == "/" {
        return path.starts_with('/');
    }
    if path == prefix {
        return true;
    }
    if path.starts_with(prefix) {
        let remainder = &path[prefix.len()..];
        return remainder.starts_with('/');
    }
    false
}

/// Convert a 9S path to a filesystem path
fn scroll_path_to_fs(base: &Path, scroll_path: &str) -> PathBuf {
    let mut fs_path = base.join("_scrolls");

    // Handle root path specially
    if scroll_path == "/" {
        fs_path.push("root.json");
        return fs_path;
    }

    // Split path and build filesystem path
    // /foo/bar -> _scrolls/foo/bar.json
    let parts: Vec<&str> = scroll_path.split('/').filter(|s| !s.is_empty()).collect();

    if parts.is_empty() {
        fs_path.push("root.json");
        return fs_path;
    }

    // All but last part become directories
    for part in &parts[..parts.len() - 1] {
        fs_path.push(part);
    }

    // Last part becomes filename.json
    let filename = format!("{}.json", parts[parts.len() - 1]);
    fs_path.push(filename);

    fs_path
}

/// Convert a filesystem path back to a 9S path
fn fs_path_to_scroll(base: &Path, fs_path: &Path) -> Option<String> {
    // Strip base/_scrolls/ prefix
    let scrolls_dir = base.join("_scrolls");
    let relative = fs_path.strip_prefix(&scrolls_dir).ok()?;

    // Handle root.json specially
    if relative == Path::new("root.json") {
        return Some("/".to_string());
    }

    // Convert path components to 9S path
    let mut parts: Vec<&str> = Vec::new();
    for component in relative.components() {
        if let std::path::Component::Normal(s) = component {
            parts.push(s.to_str()?);
        }
    }

    if parts.is_empty() {
        return None;
    }

    // Strip .json from last part
    let last = parts.pop()?;
    let last = last.strip_suffix(".json")?;
    parts.push(last);

    Some(format!("/{}", parts.join("/")))
}

struct Watcher {
    pattern: String,
    tx: Sender<Scroll>,
    alive: Arc<AtomicBool>,
    #[allow(dead_code)]
    dropped: Arc<AtomicU64>,
}

/// FileNamespace - Persistent storage using filesystem
pub struct FileNamespace {
    base_dir: PathBuf,
    watchers: RwLock<Vec<Watcher>>,
    // Cache version numbers to avoid reading file for every write
    versions: RwLock<HashMap<String, u64>>,
    closed: RwLock<bool>,
}

impl FileNamespace {
    /// Create a new FileNamespace at the given directory
    ///
    /// Creates the directory if it doesn't exist.
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Result<Self> {
        let base = base_dir.as_ref().to_path_buf();
        let scrolls_dir = base.join("_scrolls");

        fs::create_dir_all(&scrolls_dir)
            .map_err(|e| Error::Internal(format!("Failed to create directory: {}", e)))?;

        Ok(Self {
            base_dir: base,
            watchers: RwLock::new(Vec::new()),
            versions: RwLock::new(HashMap::new()),
            closed: RwLock::new(false),
        })
    }

    /// Get the base directory path
    pub fn path(&self) -> &Path {
        &self.base_dir
    }

    fn check_closed(&self) -> Result<()> {
        if *self.closed.read().unwrap() {
            return Err(Error::Closed);
        }
        Ok(())
    }

    fn notify_watchers(&self, scroll: &Scroll) {
        let watchers = self.watchers.read().unwrap();
        for watcher in watchers.iter() {
            if !watcher.alive.load(Ordering::Acquire) {
                continue;
            }
            if path_matches(&scroll.key, &watcher.pattern) {
                let _ = watcher.tx.try_send(scroll.clone());
            }
        }
    }

    #[allow(dead_code)]
    fn prune_dead_watchers(&self) {
        let mut watchers = self.watchers.write().unwrap();
        watchers.retain(|w| w.alive.load(Ordering::Acquire));
    }

    /// Get cached version or read from file
    fn get_version(&self, path: &str) -> u64 {
        // Check cache first
        if let Some(&v) = self.versions.read().unwrap().get(path) {
            return v;
        }

        // Read from file
        let fs_path = scroll_path_to_fs(&self.base_dir, path);
        if let Ok(file) = File::open(&fs_path) {
            if let Ok(scroll) = serde_json::from_reader::<_, Scroll>(BufReader::new(file)) {
                return scroll.metadata.version;
            }
        }

        0
    }

    /// Update version cache
    fn set_version(&self, path: &str, version: u64) {
        self.versions.write().unwrap().insert(path.to_string(), version);
    }
}

impl Namespace for FileNamespace {
    fn read(&self, path: &str) -> Result<Option<Scroll>> {
        self.check_closed()?;
        validate_path(path)?;

        let fs_path = scroll_path_to_fs(&self.base_dir, path);

        if !fs_path.exists() {
            return Ok(None);
        }

        let file = File::open(&fs_path)
            .map_err(|e| Error::Internal(format!("Failed to open file: {}", e)))?;

        let scroll: Scroll = serde_json::from_reader(BufReader::new(file))
            .map_err(|e| Error::InvalidData(format!("Failed to parse scroll: {}", e)))?;

        Ok(Some(scroll))
    }

    fn write(&self, path: &str, data: Value) -> Result<Scroll> {
        self.check_closed()?;
        validate_path(path)?;

        let fs_path = scroll_path_to_fs(&self.base_dir, path);

        // Ensure parent directory exists
        if let Some(parent) = fs_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| Error::Internal(format!("Failed to create directory: {}", e)))?;
        }

        // Get previous version
        let prev_version = self.get_version(path);

        // Create scroll with rich metadata
        let mut scroll = Scroll::new(path, data);
        scroll.metadata.version = prev_version + 1;
        scroll.metadata.hash = Some(scroll.compute_hash());
        scroll.metadata.created_at = Some(current_iso_time());
        scroll.metadata.updated_at = scroll.metadata.created_at.clone();

        // Write to file
        let file = File::create(&fs_path)
            .map_err(|e| Error::Internal(format!("Failed to create file: {}", e)))?;

        serde_json::to_writer_pretty(BufWriter::new(file), &scroll)
            .map_err(|e| Error::Internal(format!("Failed to write scroll: {}", e)))?;

        // Update version cache
        self.set_version(path, prev_version + 1);

        // Notify watchers
        self.notify_watchers(&scroll);

        Ok(scroll)
    }

    fn write_scroll(&self, scroll: Scroll) -> Result<Scroll> {
        self.check_closed()?;
        validate_path(&scroll.key)?;

        let fs_path = scroll_path_to_fs(&self.base_dir, &scroll.key);

        // Ensure parent directory exists
        if let Some(parent) = fs_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| Error::Internal(format!("Failed to create directory: {}", e)))?;
        }

        // Get previous version
        let prev_version = self.get_version(&scroll.key);

        // Create new scroll preserving type and other metadata from input
        let mut new_scroll = scroll.clone();
        new_scroll.metadata.version = prev_version + 1;
        new_scroll.metadata.hash = Some(new_scroll.compute_hash());
        if new_scroll.metadata.created_at.is_none() {
            new_scroll.metadata.created_at = Some(current_iso_time());
        }
        new_scroll.metadata.updated_at = Some(current_iso_time());

        // Write to file
        let file = File::create(&fs_path)
            .map_err(|e| Error::Internal(format!("Failed to create file: {}", e)))?;

        serde_json::to_writer_pretty(BufWriter::new(file), &new_scroll)
            .map_err(|e| Error::Internal(format!("Failed to write scroll: {}", e)))?;

        // Update version cache
        self.set_version(&scroll.key, prev_version + 1);

        // Notify watchers
        self.notify_watchers(&new_scroll);

        Ok(new_scroll)
    }

    fn list(&self, prefix: &str) -> Result<Vec<String>> {
        self.check_closed()?;
        validate_path(prefix)?;

        let scrolls_dir = self.base_dir.join("_scrolls");
        let mut paths = Vec::new();

        // Walk the directory tree
        fn walk_dir(dir: &Path, base: &Path, paths: &mut Vec<String>) -> std::io::Result<()> {
            if !dir.exists() {
                return Ok(());
            }

            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    walk_dir(&path, base, paths)?;
                } else if path.extension().map_or(false, |e| e == "json") {
                    if let Some(scroll_path) = fs_path_to_scroll(base, &path) {
                        paths.push(scroll_path);
                    }
                }
            }
            Ok(())
        }

        walk_dir(&scrolls_dir, &self.base_dir, &mut paths)
            .map_err(|e| Error::Internal(format!("Failed to list directory: {}", e)))?;

        // Filter by prefix with segment boundary check
        let filtered: Vec<String> = paths
            .into_iter()
            .filter(|p| is_path_under_prefix(p, prefix))
            .collect();

        Ok(filtered)
    }

    fn watch(&self, pattern: &str) -> Result<Receiver<Scroll>> {
        self.check_closed()?;
        validate_path(pattern)?;

        let (tx, rx) = channel(16);
        let alive = Arc::new(AtomicBool::new(true));

        let watcher = Watcher {
            pattern: pattern.to_string(),
            tx,
            alive,
            dropped: Arc::new(AtomicU64::new(0)),
        };

        // Atomic check-and-insert under write lock to prevent TOCTOU race
        {
            let mut watchers = self.watchers.write().unwrap();

            // Prune dead watchers first
            watchers.retain(|w| w.alive.load(Ordering::Acquire));

            // Check limit after pruning, under the same lock
            if watchers.len() >= MAX_WATCHERS {
                return Err(Error::Unavailable("too many watchers".to_string()));
            }

            watchers.push(watcher);
        }

        Ok(rx)
    }

    fn close(&self) -> Result<()> {
        let mut closed = self.closed.write().unwrap();
        *closed = true;
        self.watchers.write().unwrap().clear();
        Ok(())
    }
}

impl FileNamespace {
    /// Delete all scrolls in this namespace (DANGER: destroys all data)
    ///
    /// This physically removes all files in the _scrolls directory.
    /// Used for vault reset operations.
    pub fn delete_all(&self) -> Result<()> {
        self.check_closed()?;

        let scrolls_dir = self.base_dir.join("_scrolls");

        // Remove all contents of the scrolls directory
        if scrolls_dir.exists() {
            fs::remove_dir_all(&scrolls_dir)
                .map_err(|e| Error::Internal(format!("Failed to delete scrolls: {}", e)))?;
        }

        // Recreate the empty scrolls directory
        fs::create_dir_all(&scrolls_dir)
            .map_err(|e| Error::Internal(format!("Failed to recreate scrolls dir: {}", e)))?;

        // Clear version cache
        self.versions.write().unwrap().clear();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn file_new_creates_dir() {
        let dir = tempdir().unwrap();
        let ns = FileNamespace::new(dir.path()).unwrap();
        assert!(dir.path().join("_scrolls").exists());
        drop(ns);
    }

    #[test]
    fn file_write_read() {
        let dir = tempdir().unwrap();
        let ns = FileNamespace::new(dir.path()).unwrap();

        let scroll = ns.write("/test", json!({"foo": "bar"})).unwrap();
        assert_eq!(scroll.key, "/test");
        assert_eq!(scroll.data, json!({"foo": "bar"}));
        assert_eq!(scroll.metadata.version, 1);

        let read = ns.read("/test").unwrap().unwrap();
        assert_eq!(read.key, scroll.key);
        assert_eq!(read.data, scroll.data);
    }

    #[test]
    fn file_nested_paths() {
        let dir = tempdir().unwrap();
        let ns = FileNamespace::new(dir.path()).unwrap();

        ns.write("/a/b/c", json!(1)).unwrap();

        let read = ns.read("/a/b/c").unwrap().unwrap();
        assert_eq!(read.data, json!(1));

        // Verify file structure
        assert!(dir.path().join("_scrolls/a/b/c.json").exists());
    }

    #[test]
    fn file_version_increments() {
        let dir = tempdir().unwrap();
        let ns = FileNamespace::new(dir.path()).unwrap();

        let s1 = ns.write("/test", json!(1)).unwrap();
        assert_eq!(s1.metadata.version, 1);

        let s2 = ns.write("/test", json!(2)).unwrap();
        assert_eq!(s2.metadata.version, 2);

        let s3 = ns.write("/test", json!(3)).unwrap();
        assert_eq!(s3.metadata.version, 3);
    }

    #[test]
    fn file_list() {
        let dir = tempdir().unwrap();
        let ns = FileNamespace::new(dir.path()).unwrap();

        ns.write("/a", json!(1)).unwrap();
        ns.write("/a/b", json!(2)).unwrap();
        ns.write("/a/b/c", json!(3)).unwrap();
        ns.write("/x", json!(4)).unwrap();

        let paths = ns.list("/a").unwrap();
        assert_eq!(paths.len(), 3);
        assert!(paths.contains(&"/a".to_string()));
        assert!(paths.contains(&"/a/b".to_string()));
        assert!(paths.contains(&"/a/b/c".to_string()));
    }

    #[test]
    fn file_list_boundary_security() {
        let dir = tempdir().unwrap();
        let ns = FileNamespace::new(dir.path()).unwrap();

        ns.write("/wallet/user", json!(1)).unwrap();
        ns.write("/wallet/user/data", json!(2)).unwrap();
        ns.write("/wallet/user_archive", json!(3)).unwrap();

        let paths = ns.list("/wallet/user").unwrap();

        assert!(paths.contains(&"/wallet/user".to_string()));
        assert!(paths.contains(&"/wallet/user/data".to_string()));
        assert!(!paths.contains(&"/wallet/user_archive".to_string()));
        assert_eq!(paths.len(), 2);
    }

    #[test]
    fn file_persistence_across_instances() {
        let dir = tempdir().unwrap();

        // Write with first instance
        {
            let ns = FileNamespace::new(dir.path()).unwrap();
            ns.write("/persist", json!({"saved": true})).unwrap();
        }

        // Read with second instance
        {
            let ns = FileNamespace::new(dir.path()).unwrap();
            let scroll = ns.read("/persist").unwrap().unwrap();
            assert_eq!(scroll.data, json!({"saved": true}));
        }
    }

    #[test]
    fn file_version_persists() {
        let dir = tempdir().unwrap();

        // Write v1 and v2
        {
            let ns = FileNamespace::new(dir.path()).unwrap();
            ns.write("/test", json!(1)).unwrap();
            ns.write("/test", json!(2)).unwrap();
        }

        // New instance should continue from v3
        {
            let ns = FileNamespace::new(dir.path()).unwrap();
            let s = ns.write("/test", json!(3)).unwrap();
            assert_eq!(s.metadata.version, 3);
        }
    }

    #[test]
    fn file_root_scroll() {
        let dir = tempdir().unwrap();
        let ns = FileNamespace::new(dir.path()).unwrap();

        ns.write("/", json!({"root": true})).unwrap();

        let scroll = ns.read("/").unwrap().unwrap();
        assert_eq!(scroll.key, "/");
        assert_eq!(scroll.data, json!({"root": true}));

        // Should be in root.json
        assert!(dir.path().join("_scrolls/root.json").exists());
    }

    #[test]
    fn file_watch() {
        let dir = tempdir().unwrap();
        let ns = FileNamespace::new(dir.path()).unwrap();

        let mut rx = ns.watch("/test/**").unwrap();

        ns.write("/test/foo", json!(1)).unwrap();
        ns.write("/test/bar", json!(2)).unwrap();
        ns.write("/other", json!(3)).unwrap();

        let s1 = rx.try_recv().unwrap();
        assert_eq!(s1.key, "/test/foo");

        let s2 = rx.try_recv().unwrap();
        assert_eq!(s2.key, "/test/bar");

        assert!(rx.try_recv().is_none());
    }

    #[test]
    fn file_close() {
        let dir = tempdir().unwrap();
        let ns = FileNamespace::new(dir.path()).unwrap();

        ns.write("/test", json!(1)).unwrap();
        ns.close().unwrap();

        assert!(matches!(ns.read("/test"), Err(Error::Closed)));
    }

    #[test]
    fn scroll_path_conversion() {
        let base = Path::new("/tmp/test");

        // Normal paths
        assert_eq!(
            scroll_path_to_fs(base, "/foo"),
            PathBuf::from("/tmp/test/_scrolls/foo.json")
        );
        assert_eq!(
            scroll_path_to_fs(base, "/foo/bar"),
            PathBuf::from("/tmp/test/_scrolls/foo/bar.json")
        );
        assert_eq!(
            scroll_path_to_fs(base, "/foo/bar/baz"),
            PathBuf::from("/tmp/test/_scrolls/foo/bar/baz.json")
        );

        // Root
        assert_eq!(
            scroll_path_to_fs(base, "/"),
            PathBuf::from("/tmp/test/_scrolls/root.json")
        );
    }

    #[test]
    fn file_path_traversal_blocked() {
        // SECURITY TEST: Path traversal must be blocked at validation layer
        let dir = tempdir().unwrap();
        let ns = FileNamespace::new(dir.path()).unwrap();

        // All of these must fail with InvalidPath error
        let traversal_paths = [
            "/..",
            "/../etc",
            "/foo/..",
            "/foo/../bar",
            "/foo/../../etc/passwd",
            "/.",
            "/./foo",
            "/foo/.",
            "/foo/./bar",
        ];

        for path in &traversal_paths {
            let result = ns.write(path, json!({"exploit": true}));
            assert!(
                matches!(result, Err(Error::InvalidPath(_))),
                "Path traversal '{}' should be rejected, got: {:?}",
                path,
                result
            );
        }

        // Verify nothing was written outside the sandbox
        let parent = dir.path().parent();
        if let Some(parent_dir) = parent {
            // Check that no files leaked to parent
            let leaked = parent_dir.join("etc");
            assert!(!leaked.exists(), "Path traversal leaked to parent directory!");
        }

        // Verify normal paths still work
        assert!(ns.write("/safe/path", json!(1)).is_ok());
        assert!(ns.write("/foo.bar", json!(2)).is_ok());
        assert!(ns.write("/.hidden", json!(3)).is_ok());
    }

    #[test]
    fn file_delete_all() {
        let dir = tempdir().unwrap();
        let ns = FileNamespace::new(dir.path()).unwrap();

        // Write some data
        ns.write("/test", json!(1)).unwrap();
        ns.write("/foo/bar", json!(2)).unwrap();
        ns.write("/foo/baz", json!(3)).unwrap();

        // Verify data exists
        assert!(ns.read("/test").unwrap().is_some());
        assert!(ns.read("/foo/bar").unwrap().is_some());
        assert_eq!(ns.list("/").unwrap().len(), 3);

        // Delete all
        ns.delete_all().unwrap();

        // Verify data is gone
        assert!(ns.read("/test").unwrap().is_none());
        assert!(ns.read("/foo/bar").unwrap().is_none());
        assert_eq!(ns.list("/").unwrap().len(), 0);

        // Verify we can still write new data
        ns.write("/new", json!(42)).unwrap();
        assert!(ns.read("/new").unwrap().is_some());
        // Version should start from 1 again
        let scroll = ns.read("/new").unwrap().unwrap();
        assert_eq!(scroll.metadata.version, 1);
    }
}
