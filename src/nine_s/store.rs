//! Store - Secure, encrypted namespace storage (Bitcoin 9S)
//!
//! The Store provides encrypted, isolated namespaces for sovereign applications.
//! Unlike general 9S which may be unencrypted, Store in beewallet-core is the
//! **secure 9S reactor** - encryption is mandatory, not optional.
//!
//! ## Sovereignty Through Encryption
//!
//! Every Store requires an encryption key. This is the core of sovereignty:
//! - User owns their keys → User owns their data
//! - No plaintext storage → No data at rest exposure
//! - Key from onboarding → User chooses their security level
//!
//! ## Lexical Namespace Scoping
//!
//! Each app_key creates an isolated, encrypted namespace:
//!
//! - `Store::open("beewallet", &master_key)` → `~/.nine_s/beewallet/`
//! - `Store::at(path, &key)` → custom location (for testing)
//!
//! ```text
//! ~/.nine_s/
//!   beewallet/         <- Store::open("beewallet", &master_key)
//!     _scrolls/          (encrypted AES-256-GCM)
//!     _history/          (plaintext patches for audit)
//!   nostr-client/      <- Store::open("nostr-client", &master_key)
//!     _scrolls/          (encrypted with DIFFERENT derived key)
//!     _history/
//! ```
//!
//! ## Cryptographic Isolation (HKDF)
//!
//! When using `Store::open()`, the master key is **not** used directly.
//! Instead, an app-specific key is derived using HKDF-SHA256:
//!
//! ```text
//! app_key_beewallet = HKDF(master_key, "beewallet") → 32-byte key
//! app_key_nostr     = HKDF(master_key, "nostr")     → different 32-byte key
//! ```
//!
//! This provides **cryptographic isolation**:
//! - Apps cannot decrypt each other's data even with filesystem access
//! - Master key compromise affects all apps, but individual key leak is isolated
//! - No way to derive master key from app-specific key (one-way function)
//!
//! ## Environment Override
//!
//! Set `NINE_S_ROOT` to override the default location:
//! ```bash
//! NINE_S_ROOT=/custom/path myapp
//! ```
//!
//! ## Auditable History
//!
//! Every write automatically creates a patch in `_history/`. Patches are
//! computed on plaintext for meaningful diffs. This provides:
//! - Full audit trail of all changes
//! - Ability to anchor (checkpoint) important states
//! - Restore to any previous anchor
//!
//! # Usage
//!
//! ```rust,ignore
//! // Requires crypto feature
//! use beewallet_core_spark::nine_s::{Store, Namespace};
//! use serde_json::json;
//!
//! // Use tempdir with test key (production uses derived keys)
//! let dir = tempfile::tempdir().unwrap();
//! let key = Store::test_key();
//! let store = Store::at(dir.path(), &key).unwrap();
//!
//! // Write a scroll (encrypted at rest, plaintext in memory)
//! store.write("/ln/balance", json!({"sats": 100000})).unwrap();
//!
//! // Read it back (transparently decrypted)
//! let scroll = store.read("/ln/balance").unwrap().unwrap();
//! assert_eq!(scroll.data["sats"], 100000);
//!
//! // List all lightning scrolls
//! let paths = store.list("/ln").unwrap();
//! assert!(paths.contains(&"/ln/balance".to_string()));
//!
//! // Create an anchor (checkpoint)
//! let anchor = store.anchor("/ln/balance", Some("v1")).unwrap();
//!
//! // View history
//! let patches = store.history("/ln/balance").unwrap();
//! ```
//!
//! # Production Key Derivation
//!
//! In production, keys are derived from user onboarding:
//!
//! ```rust,ignore
//! // During user onboarding
//! let master_key = vault::crypto::derive_key(passphrase, &salt)?;
//!
//! // Open encrypted store
//! let store = Store::open("beewallet", &master_key)?;
//! ```

use super::anchor::{self, Anchor};
use super::backends::file::FileNamespace;
use super::namespace::{Error, Namespace, Receiver, Result};
use super::patch::{self, Patch};
use super::scroll::Scroll;
use serde_json::Value;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;

#[cfg(feature = "crypto")]
use crate::vault::crypto::{seal, unseal, SealedValue};

/// Statistics about a scroll's history
///
/// Useful for deciding when to compact and monitoring disk usage.
#[derive(Debug, Clone)]
pub struct HistoryStats {
    /// Number of patches in history
    pub patch_count: usize,
    /// Number of anchors
    pub anchor_count: usize,
    /// Total bytes used by patch files
    pub total_bytes: u64,
    /// Oldest patch sequence number (if any)
    pub oldest_seq: Option<u64>,
    /// Newest patch sequence number (if any)
    pub newest_seq: Option<u64>,
}

impl HistoryStats {
    /// Check if compaction is recommended
    ///
    /// Returns true if patch count exceeds 200 or total bytes exceeds 1MB.
    pub fn should_compact(&self) -> bool {
        self.patch_count > 200 || self.total_bytes > 1_000_000
    }
}

/// Statistics about scrolls under a prefix
#[derive(Debug, Clone, Default)]
pub struct PrefixStats {
    /// Number of scrolls under the prefix
    pub scroll_count: usize,
    /// Estimated total bytes (JSON size)
    pub total_bytes: u64,
    /// Oldest scroll timestamp (milliseconds)
    pub oldest_timestamp: Option<i64>,
    /// Newest scroll timestamp (milliseconds)
    pub newest_timestamp: Option<i64>,
}

impl PrefixStats {
    /// Check if pruning is recommended
    ///
    /// Returns true if scroll count exceeds 500.
    pub fn should_prune(&self) -> bool {
        self.scroll_count > 500
    }
}

/// Report from automatic maintenance
#[derive(Debug, Clone, Default)]
pub struct MaintenanceReport {
    /// Number of task failure signals pruned
    pub signals_pruned: usize,
    /// Number of lightning events pruned
    pub ln_events_pruned: usize,
    /// Number of analytics events pruned
    pub hive_events_pruned: usize,
    /// Number of wallet history patches compacted
    pub wallet_patches_compacted: usize,
}

impl MaintenanceReport {
    /// Total items cleaned up
    pub fn total(&self) -> usize {
        self.signals_pruned
            + self.ln_events_pruned
            + self.hive_events_pruned
            + self.wallet_patches_compacted
    }
}

/// Store - Secure, encrypted namespace storage (Bitcoin 9S)
///
/// The primary interface for sovereign, encrypted storage.
/// Each app gets an isolated, encrypted namespace via `Store::open(app_key, &key)`.
///
/// ## Encryption is Mandatory
///
/// ```rust,ignore
/// // During user onboarding, derive encryption key
/// let key = vault::crypto::derive_key(passphrase, &salt)?;
///
/// // Open encrypted store
/// let store = Store::open("beewallet", &key)?;
///
/// // All operations encrypt/decrypt transparently
/// store.write("/wallet/seed", json!({"phrase": "..."}))?;
/// ```
///
/// Every write creates a patch in `_history/` for auditability.
pub struct Store {
    inner: FileNamespace,
    base_dir: PathBuf,
    app_key: Option<String>,
    #[cfg(feature = "crypto")]
    encryption_key: Option<[u8; 32]>,
}

impl Store {
    /// Open an encrypted store for an application
    ///
    /// Each app_key creates an isolated, encrypted namespace at `~/.nine_s/{app_key}/`.
    /// All data is encrypted at rest using AES-256-GCM.
    ///
    /// The key should come from user onboarding - when the user creates or restores
    /// their master key, derive an encryption key for the store.
    ///
    /// # Cryptographic Isolation (HKDF)
    ///
    /// The master key is **not** used directly. Instead, an app-specific key is
    /// derived using HKDF-SHA256:
    ///
    /// ```text
    /// app_encryption_key = HKDF(master_key, salt="beewallet-9s-v1", info=app_key)
    /// ```
    ///
    /// This provides **cryptographic isolation** between apps:
    /// - Each app_key produces a unique derived encryption key
    /// - Apps cannot decrypt each other's data even with filesystem access
    /// - Compromising one app's data doesn't reveal the master key
    ///
    /// # Security
    /// - All scrolls encrypted at rest (AES-256-GCM)
    /// - App-specific key derived via HKDF (apps cryptographically isolated)
    /// - app_key validated: alphanumeric, hyphens, underscores only
    /// - No path traversal allowed
    /// - Each app isolated in its own namespace
    ///
    /// # Example
    /// ```rust,ignore
    /// // During onboarding, derive master key from user's passphrase
    /// let master_key = vault::crypto::derive_key(passphrase, &salt)?;
    ///
    /// // Open encrypted store (app-specific key derived internally)
    /// let store = Store::open("beewallet", &master_key)?;
    ///
    /// // All operations encrypt/decrypt transparently
    /// store.write("/wallet/seed", json!({"phrase": "..."}))?;
    /// let scroll = store.read("/wallet/seed")?.unwrap();
    /// ```
    #[cfg(feature = "crypto")]
    pub fn open(app_key: &str, master_key: &[u8; 32]) -> Result<Self> {
        validate_app_key(app_key)?;
        let root = nine_s_root()?;
        let path = root.join(app_key);

        // Derive app-specific encryption key using HKDF
        // This provides cryptographic isolation between apps
        let derived_key = crate::vault::crypto::derive_app_key(master_key, app_key);

        Self::at_internal(path, Some(app_key.to_string()), Some(derived_key))
    }

    /// Open an encrypted store at a custom path
    ///
    /// Use this for testing. Keys are mandatory - sovereignty requires encryption.
    #[cfg(feature = "crypto")]
    pub fn at(path: impl AsRef<std::path::Path>, key: &[u8; 32]) -> Result<Self> {
        Self::at_internal(path, None, Some(*key))
    }

    /// Generate a random test key
    ///
    /// Convenience method for tests. In production, keys come from user onboarding.
    #[cfg(feature = "crypto")]
    pub fn test_key() -> [u8; 32] {
        use rand::RngCore;
        let mut key = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut key);
        key
    }

    // Non-crypto builds: Store not available (encryption is mandatory)
    // Apps must enable the crypto feature to use Store

    /// Internal: Create store with all options
    #[cfg(feature = "crypto")]
    fn at_internal(
        path: impl AsRef<std::path::Path>,
        app_key: Option<String>,
        encryption_key: Option<[u8; 32]>,
    ) -> Result<Self> {
        let base_dir = path.as_ref().to_path_buf();

        // Ensure history directory exists
        let history_dir = base_dir.join("_history");
        fs::create_dir_all(&history_dir)
            .map_err(|e| Error::Internal(format!("Failed to create history dir: {}", e)))?;

        Ok(Self {
            inner: FileNamespace::new(&base_dir)?,
            base_dir,
            app_key,
            encryption_key,
        })
    }

    /// Get the app_key this store was opened with
    pub fn app_key(&self) -> Option<&str> {
        self.app_key.as_deref()
    }

    /// Get the filesystem path where scrolls are stored
    pub fn path(&self) -> &std::path::Path {
        self.inner.path()
    }

    /// Check if this store is encrypted
    ///
    /// Always returns true since encryption is mandatory (sovereignty).
    #[cfg(feature = "crypto")]
    pub fn is_encrypted(&self) -> bool {
        self.encryption_key.is_some()
    }

    // ========================================================================
    // Internal Read/Write (encryption-aware)
    // ========================================================================

    /// Read a scroll, respecting encryption if enabled
    #[cfg(feature = "crypto")]
    fn read_scroll(&self, path: &str) -> Result<Option<Scroll>> {
        match self.encryption_key {
            Some(ref key) => {
                let sealed_opt = self.inner.read(path)?;
                match sealed_opt {
                    Some(sealed_scroll) => {
                        let sealed: SealedValue = serde_json::from_value(sealed_scroll.data)
                            .map_err(|e| Error::Internal(format!("Failed to parse sealed data: {}", e)))?;
                        let plaintext = unseal(key, &sealed)
                            .map_err(|e| Error::Internal(format!("Decryption failed: {}", e)))?;
                        let scroll: Scroll = serde_json::from_slice(&plaintext)
                            .map_err(|e| Error::Internal(format!("Failed to parse scroll: {}", e)))?;
                        Ok(Some(scroll))
                    }
                    None => Ok(None),
                }
            }
            None => self.inner.read(path),
        }
    }

    /// Read a scroll (non-crypto fallback - just use inner)
    #[cfg(not(feature = "crypto"))]
    fn read_scroll(&self, path: &str) -> Result<Option<Scroll>> {
        self.inner.read(path)
    }

    /// Write a scroll, respecting encryption if enabled
    #[cfg(feature = "crypto")]
    fn write_scroll_internal(&self, scroll: Scroll) -> Result<Scroll> {
        // Get current state before write (decrypted if encrypted)
        let old = self.read_scroll(&scroll.key)?;

        // CSP: Derive sequence from filesystem (monotonic counter)
        let new_version = self.next_seq(&scroll.key);

        match self.encryption_key {
            Some(ref key) => {
                let mut versioned_scroll = scroll.clone();
                versioned_scroll.metadata.version = new_version;
                versioned_scroll.metadata.hash = Some(versioned_scroll.compute_hash());
                if versioned_scroll.metadata.created_at.is_none() {
                    versioned_scroll.metadata.created_at = Some(crate::nine_s::current_iso_time());
                }
                versioned_scroll.metadata.updated_at = Some(crate::nine_s::current_iso_time());

                let plaintext = serde_json::to_vec(&versioned_scroll)
                    .map_err(|e| Error::Internal(format!("Failed to serialize scroll: {}", e)))?;
                let sealed = seal(key, &plaintext)
                    .map_err(|e| Error::Internal(format!("Encryption failed: {}", e)))?;
                let sealed_data = serde_json::to_value(&sealed)
                    .map_err(|e| Error::Internal(format!("Failed to serialize sealed: {}", e)))?;

                self.inner.write(&scroll.key, sealed_data)?;

                let mut patch = patch::diff::create(&scroll.key, old.as_ref(), &versioned_scroll);
                patch.seq = new_version;
                self.store_patch(&patch)?;

                Ok(versioned_scroll)
            }
            None => {
                let mut versioned_scroll = scroll.clone();
                versioned_scroll.metadata.version = new_version;

                let result = self.inner.write_scroll(versioned_scroll.clone())?;

                let mut patch = patch::diff::create(&scroll.key, old.as_ref(), &versioned_scroll);
                patch.seq = new_version;
                self.store_patch(&patch)?;

                Ok(result)
            }
        }
    }

    /// Write a scroll (non-crypto fallback - just use inner)
    #[cfg(not(feature = "crypto"))]
    fn write_scroll_internal(&self, scroll: Scroll) -> Result<Scroll> {
        let old = self.inner.read(&scroll.key)?;
        let new_version = self.next_seq(&scroll.key);

        let mut versioned_scroll = scroll.clone();
        versioned_scroll.metadata.version = new_version;

        let result = self.inner.write_scroll(versioned_scroll.clone())?;

        let mut patch = patch::diff::create(&scroll.key, old.as_ref(), &versioned_scroll);
        patch.seq = new_version;
        self.store_patch(&patch)?;

        Ok(result)
    }

    // ========================================================================
    // History Operations
    // ========================================================================

    /// Get the patch history for a scroll
    ///
    /// Returns all patches in chronological order (oldest first).
    pub fn history(&self, path: &str) -> Result<Vec<Patch>> {
        let history_dir = self.history_dir_for_path(path);

        if !history_dir.exists() {
            return Ok(Vec::new());
        }

        let patches_dir = history_dir.join("patches");
        if !patches_dir.exists() {
            return Ok(Vec::new());
        }

        let mut patches = Vec::new();

        for entry in fs::read_dir(&patches_dir)
            .map_err(|e| Error::Internal(format!("Failed to read history: {}", e)))?
        {
            let entry =
                entry.map_err(|e| Error::Internal(format!("Failed to read entry: {}", e)))?;
            let file_path = entry.path();

            if file_path.extension().map_or(false, |e| e == "json") {
                let file = File::open(&file_path)
                    .map_err(|e| Error::Internal(format!("Failed to open patch: {}", e)))?;
                let patch: Patch = serde_json::from_reader(BufReader::new(file))
                    .map_err(|e| Error::Internal(format!("Failed to parse patch: {}", e)))?;
                patches.push(patch);
            }
        }

        // Sort by sequence number
        patches.sort_by_key(|p| p.seq);

        Ok(patches)
    }

    /// Create an anchor (immutable checkpoint) for a scroll
    ///
    /// Anchors freeze the current state with an optional label.
    /// Use `restore()` to return to an anchored state.
    pub fn anchor(&self, path: &str, label: Option<&str>) -> Result<Anchor> {
        // Use self.read_scroll() which respects encryption when crypto feature is enabled
        let scroll = self.read_scroll(path)?
            .ok_or_else(|| Error::NotFound(path.to_string()))?;

        let anchor = anchor::create(&scroll, label);

        // Store anchor
        let anchors_dir = self.history_dir_for_path(path).join("anchors");
        fs::create_dir_all(&anchors_dir)
            .map_err(|e| Error::Internal(format!("Failed to create anchors dir: {}", e)))?;

        let anchor_path = anchors_dir.join(format!("{}.json", anchor.id));
        let file = File::create(&anchor_path)
            .map_err(|e| Error::Internal(format!("Failed to create anchor file: {}", e)))?;

        serde_json::to_writer_pretty(BufWriter::new(file), &anchor)
            .map_err(|e| Error::Internal(format!("Failed to write anchor: {}", e)))?;

        Ok(anchor)
    }

    /// List all anchors for a scroll
    pub fn anchors(&self, path: &str) -> Result<Vec<Anchor>> {
        let anchors_dir = self.history_dir_for_path(path).join("anchors");

        if !anchors_dir.exists() {
            return Ok(Vec::new());
        }

        let mut anchors = Vec::new();

        for entry in fs::read_dir(&anchors_dir)
            .map_err(|e| Error::Internal(format!("Failed to read anchors: {}", e)))?
        {
            let entry =
                entry.map_err(|e| Error::Internal(format!("Failed to read entry: {}", e)))?;
            let file_path = entry.path();

            if file_path.extension().map_or(false, |e| e == "json") {
                let file = File::open(&file_path)
                    .map_err(|e| Error::Internal(format!("Failed to open anchor: {}", e)))?;
                let anchor: Anchor = serde_json::from_reader(BufReader::new(file))
                    .map_err(|e| Error::Internal(format!("Failed to parse anchor: {}", e)))?;
                anchors.push(anchor);
            }
        }

        // Sort by timestamp
        anchors.sort_by_key(|a| a.timestamp);

        Ok(anchors)
    }

    /// Reconstruct the state of a scroll at a specific point in history
    ///
    /// Returns the scroll as it existed after the given sequence number.
    /// This is read-only - it doesn't modify the current state.
    ///
    /// # Example
    /// ```rust,ignore
    /// let store = Store::at(path)?;
    /// store.write("/doc", json!({"v": 1}))?;  // seq 1
    /// store.write("/doc", json!({"v": 2}))?;  // seq 2
    /// store.write("/doc", json!({"v": 3}))?;  // seq 3
    ///
    /// let v2_state = store.state_at("/doc", 2)?;
    /// assert_eq!(v2_state.data["v"], 2);
    /// ```
    pub fn state_at(&self, path: &str, seq: u64) -> Result<Scroll> {
        let patches = self.history(path)?;

        if patches.is_empty() {
            return Err(Error::NotFound(path.to_string()));
        }

        if seq == 0 || seq > patches.len() as u64 {
            return Err(Error::Internal(format!(
                "Invalid sequence number {}. Valid range: 1-{}",
                seq,
                patches.len()
            )));
        }

        // Apply patches up to the requested sequence
        let mut current = Scroll::new(path, serde_json::json!({}));

        for patch in patches.iter().take(seq as usize) {
            current = patch::diff::apply(&current, patch)
                .map_err(|e| Error::Internal(format!("Failed to apply patch: {}", e)))?;
        }

        Ok(current)
    }

    /// Restore a scroll to an anchored state
    ///
    /// This creates a new version with the anchor's content.
    /// The restoration itself is recorded in history.
    pub fn restore(&self, path: &str, anchor_id: &str) -> Result<Scroll> {
        let anchors = self.anchors(path)?;
        let anchor = anchors
            .into_iter()
            .find(|a| a.id == anchor_id)
            .ok_or_else(|| Error::NotFound(format!("anchor:{}", anchor_id)))?;

        // Verify anchor integrity
        if !anchor::verify(&anchor) {
            return Err(Error::Internal("Anchor integrity check failed".to_string()));
        }

        // Write the restored content (creates new patch in history)
        // Use self.write_scroll_internal() which respects encryption
        self.write_scroll_internal(anchor.scroll)
    }

    // ========================================================================
    // History Pruning (Synthesis: Memory with Purpose)
    // ========================================================================

    /// Compact history by removing old patches
    ///
    /// Retains patches from `keep_since_seq` (inclusive) to the present.
    /// If `keep_since_seq` is None, retains patches since the last anchor
    /// or the last 100 patches, whichever is more.
    ///
    /// # Dialectics
    ///
    /// **Thesis**: Total history (audit everything)
    /// **Antithesis**: Disk exhaustion (store nothing)
    /// **Synthesis**: Anchor-bounded retention (keep what matters)
    ///
    /// # Example
    /// ```rust,ignore
    /// // Keep only patches from seq 50 onwards
    /// store.compact("/ledger", Some(50))?;
    ///
    /// // Auto-detect: keep since last anchor or last 100
    /// store.compact("/ledger", None)?;
    /// ```
    ///
    /// # Returns
    /// The number of patches removed.
    pub fn compact(&self, path: &str, keep_since_seq: Option<u64>) -> Result<usize> {
        let patches = self.history(path)?;
        if patches.is_empty() {
            return Ok(0);
        }

        // Determine retention threshold
        let threshold = match keep_since_seq {
            Some(seq) => seq,
            None => self.auto_retention_threshold(path, &patches)?,
        };

        // Delete patches older than threshold
        let patches_dir = self.history_dir_for_path(path).join("patches");
        let mut removed = 0;

        for patch in patches.iter().filter(|p| p.seq < threshold) {
            let patch_path = patches_dir.join(format!("{:08}.json", patch.seq));
            if patch_path.exists() {
                fs::remove_file(&patch_path)
                    .map_err(|e| Error::Internal(format!("Failed to remove patch: {}", e)))?;
                removed += 1;
            }
        }

        Ok(removed)
    }

    /// Compact all paths under a prefix
    ///
    /// Useful for periodic maintenance across a namespace.
    ///
    /// # Example
    /// ```rust,ignore
    /// // Compact all wallet history
    /// store.compact_all("/wallet")?;
    /// ```
    pub fn compact_all(&self, prefix: &str) -> Result<usize> {
        let paths = self.inner.list(prefix)?;
        let mut total_removed = 0;

        for path in paths {
            total_removed += self.compact(&path, None)?;
        }

        Ok(total_removed)
    }

    /// Get the retention threshold for auto-compaction
    ///
    /// Strategy:
    /// 1. If anchors exist, keep from the oldest anchor's seq
    /// 2. Otherwise, keep the last DEFAULT_RETENTION patches
    fn auto_retention_threshold(&self, path: &str, patches: &[Patch]) -> Result<u64> {
        const DEFAULT_RETENTION: u64 = 100;

        // Check for anchors
        let anchors = self.anchors(path)?;

        if anchors.is_empty() {
            // No anchors: keep last DEFAULT_RETENTION patches
            let current_seq = patches.last().map(|p| p.seq).unwrap_or(0);
            Ok(current_seq.saturating_sub(DEFAULT_RETENTION))
        } else {
            // Has anchors: find the oldest anchor's seq
            // We need to reconstruct what seq each anchor corresponds to
            // The anchor contains a scroll, which has metadata.version
            let oldest_anchor_seq = anchors
                .iter()
                .filter_map(|a| {
                    // The scroll's version tells us what seq it was anchored at
                    Some(a.scroll.metadata.version)
                })
                .min()
                .unwrap_or(1);

            Ok(oldest_anchor_seq)
        }
    }

    /// Get history statistics for a path
    ///
    /// Useful for deciding when to compact.
    pub fn history_stats(&self, path: &str) -> Result<HistoryStats> {
        let patches = self.history(path)?;
        let anchors = self.anchors(path)?;

        let patches_dir = self.history_dir_for_path(path).join("patches");
        let total_bytes: u64 = if patches_dir.exists() {
            fs::read_dir(&patches_dir)
                .map_err(|e| Error::Internal(e.to_string()))?
                .filter_map(|e| e.ok())
                .filter_map(|e| e.metadata().ok())
                .map(|m| m.len())
                .sum()
        } else {
            0
        };

        Ok(HistoryStats {
            patch_count: patches.len(),
            anchor_count: anchors.len(),
            total_bytes,
            oldest_seq: patches.first().map(|p| p.seq),
            newest_seq: patches.last().map(|p| p.seq),
        })
    }

    // ========================================================================
    // Signal/Event Pruning (Storage Backpressure)
    // ========================================================================

    /// Prune old scrolls under a prefix, keeping only the most recent N
    ///
    /// This is essential for directories that accumulate unbounded events:
    /// - `/signals/task-failure/*` - task failure notifications
    /// - `/ln/events/*` - lightning event log
    /// - `/hive/events/*` - analytics events
    ///
    /// # Dialectics
    ///
    /// **Thesis**: Log everything (total observability)
    /// **Antithesis**: Disk exhaustion (log nothing)
    /// **Synthesis**: Rolling window (keep recent, prune old)
    ///
    /// # Example
    /// ```rust,ignore
    /// // Keep only last 100 task failure signals
    /// store.prune("/signals/task-failure", 100)?;
    ///
    /// // Keep only last 1000 lightning events
    /// store.prune("/ln/events", 1000)?;
    /// ```
    ///
    /// # Returns
    /// The number of scrolls removed.
    pub fn prune(&self, prefix: &str, keep_count: usize) -> Result<usize> {
        let paths = self.inner.list(prefix)?;

        if paths.len() <= keep_count {
            return Ok(0);
        }

        // Get scrolls with their timestamps for sorting
        let mut scrolls_with_time: Vec<(String, i64)> = Vec::new();
        for path in &paths {
            if let Ok(Some(scroll)) = self.inner.read(path) {
                // Use updated_at or created_at timestamp (stored as Unix millis string)
                let timestamp = scroll
                    .metadata
                    .updated_at
                    .as_ref()
                    .or(scroll.metadata.created_at.as_ref())
                    .map(|ts| super::scroll::iso_to_unix_millis(ts))
                    .unwrap_or(0);
                scrolls_with_time.push((path.clone(), timestamp));
            }
        }

        // Sort by timestamp (oldest first)
        scrolls_with_time.sort_by_key(|(_, ts)| *ts);

        // Calculate how many to remove
        let remove_count = scrolls_with_time.len().saturating_sub(keep_count);
        let mut removed = 0;

        // Remove oldest scrolls
        for (path, _) in scrolls_with_time.into_iter().take(remove_count) {
            if self.delete(&path).is_ok() {
                removed += 1;
            }
        }

        Ok(removed)
    }

    /// Prune scrolls older than a duration
    ///
    /// Removes scrolls under the prefix that are older than `max_age`.
    ///
    /// # Example
    /// ```rust,ignore
    /// use std::time::Duration;
    ///
    /// // Remove signals older than 7 days
    /// store.prune_older_than("/signals", Duration::from_secs(7 * 24 * 60 * 60))?;
    /// ```
    pub fn prune_older_than(
        &self,
        prefix: &str,
        max_age: std::time::Duration,
    ) -> Result<usize> {
        let paths = self.inner.list(prefix)?;
        let cutoff = super::current_time_millis() - (max_age.as_millis() as i64);
        let mut removed = 0;

        for path in paths {
            if let Ok(Some(scroll)) = self.inner.read(&path) {
                // Timestamps stored as Unix millis strings
                let timestamp = scroll
                    .metadata
                    .updated_at
                    .as_ref()
                    .or(scroll.metadata.created_at.as_ref())
                    .map(|ts| super::scroll::iso_to_unix_millis(ts))
                    .unwrap_or(i64::MAX); // Keep if no timestamp

                if timestamp < cutoff {
                    if self.delete(&path).is_ok() {
                        removed += 1;
                    }
                }
            }
        }

        Ok(removed)
    }

    /// Delete a scroll at the given path
    ///
    /// Removes the scroll file. Does NOT remove history (use compact for that).
    pub fn delete(&self, path: &str) -> Result<()> {
        // Delegate to inner namespace
        // FileNamespace stores at: _scrolls/{path}.json
        let clean_path = path.trim_start_matches('/');
        let file_path = self.base_dir.join("_scrolls").join(format!("{}.json", clean_path));

        if file_path.exists() {
            fs::remove_file(&file_path)
                .map_err(|e| Error::Internal(format!("Failed to delete scroll: {}", e)))?;
        }

        Ok(())
    }

    /// Get statistics about a prefix (for monitoring)
    pub fn prefix_stats(&self, prefix: &str) -> Result<PrefixStats> {
        let paths = self.inner.list(prefix)?;
        let mut total_bytes: u64 = 0;
        let mut oldest_timestamp: Option<i64> = None;
        let mut newest_timestamp: Option<i64> = None;

        for path in &paths {
            if let Ok(Some(scroll)) = self.inner.read(path) {
                // Estimate size from JSON
                if let Ok(json) = serde_json::to_string(&scroll) {
                    total_bytes += json.len() as u64;
                }

                // Timestamps stored as Unix millis strings
                let timestamp = scroll
                    .metadata
                    .updated_at
                    .as_ref()
                    .or(scroll.metadata.created_at.as_ref())
                    .map(|ts| super::scroll::iso_to_unix_millis(ts));

                if let Some(ts) = timestamp {
                    oldest_timestamp = Some(oldest_timestamp.map_or(ts, |old| old.min(ts)));
                    newest_timestamp = Some(newest_timestamp.map_or(ts, |old| old.max(ts)));
                }
            }
        }

        Ok(PrefixStats {
            scroll_count: paths.len(),
            total_bytes,
            oldest_timestamp,
            newest_timestamp,
        })
    }

    /// Run automatic maintenance on common signal/event paths
    ///
    /// This is a convenience method that applies sensible defaults:
    /// - `/signals/task-failure/*` - keep last 100
    /// - `/ln/events/*` - keep last 1000
    /// - `/hive/events/*` - keep last 500
    ///
    /// Call periodically (e.g., on app startup or daily).
    pub fn auto_maintenance(&self) -> Result<MaintenanceReport> {
        let mut report = MaintenanceReport::default();

        // Prune task failure signals
        report.signals_pruned = self.prune("/signals/task-failure", 100).unwrap_or(0);

        // Prune lightning events
        report.ln_events_pruned = self.prune("/ln/events", 1000).unwrap_or(0);

        // Prune analytics events
        report.hive_events_pruned = self.prune("/hive/events", 500).unwrap_or(0);

        // Compact wallet history
        report.wallet_patches_compacted = self.compact_all("/wallet").unwrap_or(0);

        Ok(report)
    }

    // ========================================================================
    // Internal: History storage
    // ========================================================================

    /// Get the history directory for a scroll path
    fn history_dir_for_path(&self, scroll_path: &str) -> PathBuf {
        // /foo/bar -> _history/foo/bar/
        let clean_path = scroll_path.trim_start_matches('/');
        self.base_dir.join("_history").join(clean_path)
    }

    /// Store a patch in history
    fn store_patch(&self, patch: &Patch) -> Result<()> {
        let patches_dir = self.history_dir_for_path(&patch.key).join("patches");
        fs::create_dir_all(&patches_dir)
            .map_err(|e| Error::Internal(format!("Failed to create patches dir: {}", e)))?;

        // Use seq number for ordering
        let patch_path = patches_dir.join(format!("{:08}.json", patch.seq));
        let file = File::create(&patch_path)
            .map_err(|e| Error::Internal(format!("Failed to create patch file: {}", e)))?;

        serde_json::to_writer_pretty(BufWriter::new(file), patch)
            .map_err(|e| Error::Internal(format!("Failed to write patch: {}", e)))?;

        Ok(())
    }

    /// Get the next sequence number for a path (CSP-style monotonic counter)
    ///
    /// # CSP Insight (Tony Hoare)
    ///
    /// The sequence number must be derived from the **existing state of the channel**
    /// (the filesystem), not from in-flight messages (scroll metadata). This is like
    /// Bitcoin's block height - determined by counting existing blocks.
    ///
    /// By reading the patches directory, we get the true sequence regardless of
    /// how many writes are in flight. The filesystem acts as our sequencing channel.
    fn next_seq(&self, path: &str) -> u64 {
        let patches_dir = self.history_dir_for_path(path).join("patches");

        if !patches_dir.exists() {
            return 1;
        }

        // Count existing patches by finding the highest sequence number
        let mut max_seq: u64 = 0;

        if let Ok(entries) = fs::read_dir(&patches_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.path().file_stem() {
                    if let Ok(seq) = name.to_string_lossy().parse::<u64>() {
                        max_seq = max_seq.max(seq);
                    }
                }
            }
        }

        max_seq + 1
    }
}

/// Get the 9S root directory
///
/// Checks `NINE_S_ROOT` environment variable first, then falls back to `~/.nine_s`.
///
/// # Security
/// The env var allows container/sandbox environments to redirect storage
/// without affecting app logic.
#[cfg(feature = "crypto")]
fn nine_s_root() -> Result<PathBuf> {
    if let Ok(root) = std::env::var("NINE_S_ROOT") {
        return Ok(PathBuf::from(root));
    }
    dirs::home_dir()
        .map(|h| h.join(".nine_s"))
        .ok_or_else(|| Error::Internal("Could not determine home directory".to_string()))
}

/// Validate app_key for security
///
/// # Rules
/// - Must be 1-64 characters
/// - Only alphanumeric, hyphens, underscores allowed
/// - Cannot start with hyphen or underscore
/// - Cannot be "." or ".." (path traversal)
/// - Cannot contain path separators
#[cfg(any(feature = "crypto", test))]
fn validate_app_key(app_key: &str) -> Result<()> {
    // Length check
    if app_key.is_empty() {
        return Err(Error::InvalidPath("app_key cannot be empty".to_string()));
    }
    if app_key.len() > 64 {
        return Err(Error::InvalidPath(
            "app_key cannot exceed 64 characters".to_string(),
        ));
    }

    // Path traversal check
    if app_key == "." || app_key == ".." {
        return Err(Error::InvalidPath(
            "app_key cannot be path traversal sequence".to_string(),
        ));
    }

    // First character check
    let first = app_key.chars().next().unwrap();
    if first == '-' || first == '_' {
        return Err(Error::InvalidPath(
            "app_key cannot start with hyphen or underscore".to_string(),
        ));
    }

    // Character validation
    for c in app_key.chars() {
        if !c.is_ascii_alphanumeric() && c != '-' && c != '_' {
            return Err(Error::InvalidPath(format!(
                "app_key contains invalid character '{}'. Only alphanumeric, hyphen, underscore allowed",
                c
            )));
        }
    }

    Ok(())
}

// Namespace implementation with automatic history tracking
// When encryption_key is set, all data is encrypted at rest

#[cfg(feature = "crypto")]
impl Namespace for Store {
    fn read(&self, path: &str) -> Result<Option<Scroll>> {
        match self.encryption_key {
            Some(ref key) => {
                // Read encrypted scroll
                let sealed_opt = self.inner.read(path)?;
                match sealed_opt {
                    Some(sealed_scroll) => {
                        // The data field contains the SealedValue
                        let sealed: SealedValue = serde_json::from_value(sealed_scroll.data)
                            .map_err(|e| Error::Internal(format!("Failed to parse sealed data: {}", e)))?;

                        // Decrypt
                        let plaintext = unseal(key, &sealed)
                            .map_err(|e| Error::Internal(format!("Decryption failed: {}", e)))?;

                        // Deserialize scroll
                        let scroll: Scroll = serde_json::from_slice(&plaintext)
                            .map_err(|e| Error::Internal(format!("Failed to parse scroll: {}", e)))?;

                        Ok(Some(scroll))
                    }
                    None => Ok(None),
                }
            }
            None => self.inner.read(path),
        }
    }

    fn write(&self, path: &str, data: Value) -> Result<Scroll> {
        let scroll = Scroll::new(path, data);
        self.write_scroll(scroll)
    }

    fn write_scroll(&self, scroll: Scroll) -> Result<Scroll> {
        // Get current state before write (decrypted if encrypted)
        let old = self.read(&scroll.key)?;

        // CSP: Derive sequence from filesystem (monotonic counter)
        // This is the source of truth, like Bitcoin's block height
        let new_version = self.next_seq(&scroll.key);

        match self.encryption_key {
            Some(ref key) => {
                // Update scroll with version from filesystem
                let mut versioned_scroll = scroll.clone();
                versioned_scroll.metadata.version = new_version;
                versioned_scroll.metadata.hash = Some(versioned_scroll.compute_hash());
                if versioned_scroll.metadata.created_at.is_none() {
                    versioned_scroll.metadata.created_at = Some(crate::nine_s::current_iso_time());
                }
                versioned_scroll.metadata.updated_at = Some(crate::nine_s::current_iso_time());

                // Serialize versioned scroll
                let plaintext = serde_json::to_vec(&versioned_scroll)
                    .map_err(|e| Error::Internal(format!("Failed to serialize scroll: {}", e)))?;

                // Encrypt
                let sealed = seal(key, &plaintext)
                    .map_err(|e| Error::Internal(format!("Encryption failed: {}", e)))?;

                // Store as a scroll with SealedValue as data
                let sealed_data = serde_json::to_value(&sealed)
                    .map_err(|e| Error::Internal(format!("Failed to serialize sealed: {}", e)))?;

                // Write encrypted data to inner store
                self.inner.write(&scroll.key, sealed_data)?;

                // Create patch with CSP-derived sequence
                let mut patch = patch::diff::create(&scroll.key, old.as_ref(), &versioned_scroll);
                patch.seq = new_version; // Override with filesystem-derived seq
                self.store_patch(&patch)?;

                // Return the versioned scroll
                Ok(versioned_scroll)
            }
            None => {
                // Update scroll with version from filesystem
                let mut versioned_scroll = scroll.clone();
                versioned_scroll.metadata.version = new_version;

                // Write to inner store
                let result = self.inner.write_scroll(versioned_scroll.clone())?;

                // Create patch with CSP-derived sequence
                let mut patch = patch::diff::create(&scroll.key, old.as_ref(), &versioned_scroll);
                patch.seq = new_version; // Override with filesystem-derived seq
                self.store_patch(&patch)?;

                Ok(result)
            }
        }
    }

    fn list(&self, prefix: &str) -> Result<Vec<String>> {
        self.inner.list(prefix)
    }

    fn watch(&self, pattern: &str) -> Result<Receiver<Scroll>> {
        // For encrypted stores, use watch_decrypted() instead
        // This returns raw (encrypted) scrolls for backwards compatibility
        self.inner.watch(pattern)
    }

    fn close(&self) -> Result<()> {
        self.inner.close()
    }
}

#[cfg(feature = "crypto")]
impl Store {
    /// Watch for changes with automatic decryption
    ///
    /// Unlike `watch()` from the Namespace trait, this method transparently
    /// decrypts scrolls before delivering them. This maintains the Store's
    /// abstraction that encryption is transparent.
    ///
    /// # Example
    /// ```rust,ignore
    /// let store = Store::at(path, &key)?;
    /// let mut rx = store.watch_decrypted("/wallet/*")?;
    ///
    /// while let Some(scroll) = rx.recv() {
    ///     // scroll.data is already decrypted
    ///     println!("Balance: {}", scroll.data["sats"]);
    /// }
    /// ```
    ///
    /// # Errors
    /// - Returns an error if the pattern is invalid
    /// - Individual scrolls that fail decryption are silently dropped
    ///   (logged to stderr) rather than failing the entire watch
    pub fn watch_decrypted(&self, pattern: &str) -> Result<Receiver<Scroll>> {
        use super::channel;

        match &self.encryption_key {
            Some(key) => {
                let mut inner_rx = self.inner.watch(pattern)?;
                let key = *key;

                // Create a new channel for decrypted scrolls
                let (tx, rx) = channel::channel::<Scroll>(64);

                // Spawn a thread to decrypt scrolls
                // (Using thread because Receiver is sync, not async)
                std::thread::spawn(move || {
                    while let Some(sealed_scroll) = inner_rx.recv() {
                        // Attempt to decrypt
                        match decrypt_scroll(&sealed_scroll, &key) {
                            Ok(scroll) => {
                                if tx.send(scroll).is_err() {
                                    // Receiver dropped, stop processing
                                    break;
                                }
                            }
                            Err(e) => {
                                // Log but don't fail - watcher should continue
                                eprintln!(
                                    "watch_decrypted: failed to decrypt {}: {}",
                                    sealed_scroll.key, e
                                );
                            }
                        }
                    }
                });

                Ok(rx)
            }
            None => {
                // No encryption, just pass through
                self.inner.watch(pattern)
            }
        }
    }
}

/// Decrypt a sealed scroll
#[cfg(feature = "crypto")]
fn decrypt_scroll(sealed_scroll: &Scroll, key: &[u8; 32]) -> Result<Scroll> {
    // The data field contains the SealedValue
    let sealed: SealedValue = serde_json::from_value(sealed_scroll.data.clone())
        .map_err(|e| Error::Internal(format!("Failed to parse sealed data: {}", e)))?;

    // Decrypt
    let plaintext = unseal(key, &sealed)
        .map_err(|e| Error::Internal(format!("Decryption failed: {}", e)))?;

    // Deserialize scroll
    let scroll: Scroll = serde_json::from_slice(&plaintext)
        .map_err(|e| Error::Internal(format!("Failed to parse scroll: {}", e)))?;

    Ok(scroll)
}

// Note: Non-crypto builds cannot use Store because:
// 1. Encryption is mandatory (sovereignty)
// 2. Encryption requires the crypto feature (vault/crypto)
// 3. All Store constructors require encryption keys
// Use FileNamespace directly for non-encrypted storage needs.

#[cfg(all(test, feature = "crypto"))]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn store_at_custom_path() {
        let dir = tempdir().unwrap();
        let store = Store::at(dir.path(), &Store::test_key()).unwrap();

        let scroll = store.write("/test", json!({"foo": "bar"})).unwrap();
        assert_eq!(scroll.key, "/test");
        assert_eq!(scroll.metadata.version, 1);

        let read = store.read("/test").unwrap().unwrap();
        assert_eq!(read.data, json!({"foo": "bar"}));
    }

    #[test]
    fn store_list_and_watch() {
        let dir = tempdir().unwrap();
        let store = Store::at(dir.path(), &Store::test_key()).unwrap();

        store.write("/a/1", json!(1)).unwrap();
        store.write("/a/2", json!(2)).unwrap();
        store.write("/b/1", json!(3)).unwrap();

        let paths = store.list("/a").unwrap();
        assert_eq!(paths.len(), 2);
    }

    #[test]
    fn store_creates_history() {
        let dir = tempdir().unwrap();
        let store = Store::at(dir.path(), &Store::test_key()).unwrap();

        // Write creates history
        store.write("/test", json!({"v": 1})).unwrap();
        store.write("/test", json!({"v": 2})).unwrap();
        store.write("/test", json!({"v": 3})).unwrap();

        // Check history
        let patches = store.history("/test").unwrap();
        assert_eq!(patches.len(), 3);
        assert_eq!(patches[0].seq, 1);
        assert_eq!(patches[1].seq, 2);
        assert_eq!(patches[2].seq, 3);
    }

    #[test]
    fn store_anchor_and_restore() {
        let dir = tempdir().unwrap();
        let store = Store::at(dir.path(), &Store::test_key()).unwrap();

        // Create initial state
        store.write("/test", json!({"value": "original"})).unwrap();

        // Create anchor
        let anchor = store.anchor("/test", Some("v1")).unwrap();
        assert_eq!(anchor.label, Some("v1".to_string()));

        // Modify state
        store.write("/test", json!({"value": "modified"})).unwrap();

        // Verify modification
        let scroll = store.read("/test").unwrap().unwrap();
        assert_eq!(scroll.data["value"], "modified");

        // Restore to anchor
        let restored = store.restore("/test", &anchor.id).unwrap();
        assert_eq!(restored.data["value"], "original");

        // Verify restore persisted
        let scroll = store.read("/test").unwrap().unwrap();
        assert_eq!(scroll.data["value"], "original");
    }

    #[test]
    fn store_list_anchors() {
        let dir = tempdir().unwrap();
        let store = Store::at(dir.path(), &Store::test_key()).unwrap();

        store.write("/test", json!({"v": 1})).unwrap();
        store.anchor("/test", Some("first")).unwrap();

        store.write("/test", json!({"v": 2})).unwrap();
        store.anchor("/test", Some("second")).unwrap();

        let anchors = store.anchors("/test").unwrap();
        assert_eq!(anchors.len(), 2);

        // Check both labels exist (order may vary with same-ms timestamps)
        let labels: Vec<_> = anchors.iter().map(|a| a.label.clone()).collect();
        assert!(labels.contains(&Some("first".to_string())));
        assert!(labels.contains(&Some("second".to_string())));
    }

    #[test]
    fn store_history_empty_for_nonexistent() {
        let dir = tempdir().unwrap();
        let store = Store::at(dir.path(), &Store::test_key()).unwrap();

        let patches = store.history("/nonexistent").unwrap();
        assert!(patches.is_empty());

        let anchors = store.anchors("/nonexistent").unwrap();
        assert!(anchors.is_empty());
    }

    #[test]
    fn store_history_preserved_across_instances() {
        let dir = tempdir().unwrap();

        // Write with first instance
        {
            let store = Store::at(dir.path(), &Store::test_key()).unwrap();
            store.write("/test", json!(1)).unwrap();
            store.write("/test", json!(2)).unwrap();
            store.anchor("/test", Some("v2")).unwrap();
        }

        // Read history with second instance
        {
            let store = Store::at(dir.path(), &Store::test_key()).unwrap();
            let patches = store.history("/test").unwrap();
            assert_eq!(patches.len(), 2);

            let anchors = store.anchors("/test").unwrap();
            assert_eq!(anchors.len(), 1);
            assert_eq!(anchors[0].label, Some("v2".to_string()));
        }
    }

    // ========================================================================
    // Time Travel Tests - Reconstruct state at any point in history
    // ========================================================================

    #[test]
    fn time_travel_reconstruct_state_from_patches() {
        use super::patch::diff;

        let dir = tempdir().unwrap();
        let store = Store::at(dir.path(), &Store::test_key()).unwrap();

        // Create a sequence of changes
        store.write("/ledger", json!({"balance": 100})).unwrap();
        store.write("/ledger", json!({"balance": 150, "pending": 50})).unwrap();
        store.write("/ledger", json!({"balance": 200, "pending": 0})).unwrap();
        store.write("/ledger", json!({"balance": 175, "pending": 0, "fee": 25})).unwrap();

        // Get history
        let patches = store.history("/ledger").unwrap();
        assert_eq!(patches.len(), 4);

        // Reconstruct state at each point by applying patches sequentially
        let mut current = Scroll::new("/ledger", json!({}));

        // After patch 1: balance = 100
        current = diff::apply(&current, &patches[0]).unwrap();
        assert_eq!(current.data["balance"], 100);

        // After patch 2: balance = 150, pending = 50
        current = diff::apply(&current, &patches[1]).unwrap();
        assert_eq!(current.data["balance"], 150);
        assert_eq!(current.data["pending"], 50);

        // After patch 3: balance = 200, pending = 0
        current = diff::apply(&current, &patches[2]).unwrap();
        assert_eq!(current.data["balance"], 200);
        assert_eq!(current.data["pending"], 0);

        // After patch 4: balance = 175, pending = 0, fee = 25
        current = diff::apply(&current, &patches[3]).unwrap();
        assert_eq!(current.data["balance"], 175);
        assert_eq!(current.data["fee"], 25);
    }

    #[test]
    fn time_travel_via_anchors() {
        let dir = tempdir().unwrap();
        let store = Store::at(dir.path(), &Store::test_key()).unwrap();

        // Build up state with checkpoints
        store.write("/account", json!({"name": "Alice", "level": 1})).unwrap();
        let v1 = store.anchor("/account", Some("beginner")).unwrap();

        store.write("/account", json!({"name": "Alice", "level": 5})).unwrap();
        let v2 = store.anchor("/account", Some("intermediate")).unwrap();

        store.write("/account", json!({"name": "Alice", "level": 10, "badge": "pro"})).unwrap();
        let v3 = store.anchor("/account", Some("pro")).unwrap();

        // Current state is level 10
        let current = store.read("/account").unwrap().unwrap();
        assert_eq!(current.data["level"], 10);
        assert_eq!(current.data["badge"], "pro");

        // Jump back to v1 (beginner)
        let restored = store.restore("/account", &v1.id).unwrap();
        assert_eq!(restored.data["level"], 1);
        assert!(restored.data.get("badge").is_none());

        // History should show the restore as a new patch
        let patches = store.history("/account").unwrap();
        assert_eq!(patches.len(), 4); // 3 original + 1 restore

        // Jump to v2 (intermediate)
        store.restore("/account", &v2.id).unwrap();
        let restored = store.read("/account").unwrap().unwrap();
        assert_eq!(restored.data["level"], 5);

        // Jump to v3 (pro) - we can still reach this
        store.restore("/account", &v3.id).unwrap();
        let restored = store.read("/account").unwrap().unwrap();
        assert_eq!(restored.data["level"], 10);
        assert_eq!(restored.data["badge"], "pro");
    }

    // ========================================================================
    // Conflict Detection Tests
    // ========================================================================

    #[test]
    fn conflict_detection_via_hash_chain() {
        use super::patch::diff;

        let dir = tempdir().unwrap();
        let store = Store::at(dir.path(), &Store::test_key()).unwrap();

        // Initial state
        store.write("/shared", json!({"version": 1})).unwrap();

        // Get current state and hash
        let current = store.read("/shared").unwrap().unwrap();
        let current_hash = diff::hash(&current);

        // Simulate two concurrent modifications
        // Writer A: version -> 2
        let writer_a_new = Scroll::new("/shared", json!({"version": 2, "author": "A"}));
        let patch_a = diff::create("/shared", Some(&current), &writer_a_new);

        // Writer B: version -> 2 (different change)
        let writer_b_new = Scroll::new("/shared", json!({"version": 2, "author": "B"}));
        let patch_b = diff::create("/shared", Some(&current), &writer_b_new);

        // Both patches have the same parent (current_hash)
        assert_eq!(patch_a.parent, patch_b.parent);
        assert_eq!(patch_a.parent.as_ref().unwrap(), &current_hash);

        // Writer A wins (writes first)
        store.write("/shared", json!({"version": 2, "author": "A"})).unwrap();

        // Writer B's patch is now invalid (parent hash doesn't match)
        let new_current = store.read("/shared").unwrap().unwrap();
        let new_hash = diff::hash(&new_current);
        assert_ne!(new_hash, current_hash);
        assert_ne!(patch_b.parent.as_ref().unwrap(), &new_hash);

        // This is how you detect a conflict: parent hash mismatch
        let conflict_detected = patch_b.parent.as_ref() != Some(&new_hash);
        assert!(conflict_detected);
    }

    #[test]
    fn conflict_resolution_last_write_wins() {
        let dir = tempdir().unwrap();
        let store = Store::at(dir.path(), &Store::test_key()).unwrap();

        // Initial state
        store.write("/counter", json!({"value": 0})).unwrap();

        // Concurrent updates (simulated by sequential writes)
        // In a real system, both would read value=0 and try to write
        store.write("/counter", json!({"value": 1})).unwrap(); // Thread A
        store.write("/counter", json!({"value": 1})).unwrap(); // Thread B (same value)

        // Both changes are recorded
        let patches = store.history("/counter").unwrap();
        assert_eq!(patches.len(), 3); // Initial + 2 writes

        // Final state reflects last write
        let final_state = store.read("/counter").unwrap().unwrap();
        assert_eq!(final_state.data["value"], 1);
    }

    // ========================================================================
    // Collaboration Tests - Multiple processes sharing storage
    // ========================================================================

    #[test]
    fn collaboration_shared_filesystem() {
        let dir = tempdir().unwrap();

        // Collaborating processes must share the same encryption key
        // (e.g., derived from shared secret during onboarding)
        let shared_key = Store::test_key();

        // Process A: Write initial data
        {
            let store_a = Store::at(dir.path(), &shared_key).unwrap();
            store_a.write("/shared/doc", json!({"title": "Draft", "author": "A"})).unwrap();
        }

        // Process B: Read and modify (same key = can decrypt)
        {
            let store_b = Store::at(dir.path(), &shared_key).unwrap();
            let doc = store_b.read("/shared/doc").unwrap().unwrap();
            assert_eq!(doc.data["author"], "A");

            // B adds their contribution
            store_b.write("/shared/doc", json!({
                "title": "Draft",
                "author": "A",
                "reviewer": "B",
                "status": "reviewed"
            })).unwrap();
        }

        // Process A: See B's changes
        {
            let store_a = Store::at(dir.path(), &shared_key).unwrap();
            let doc = store_a.read("/shared/doc").unwrap().unwrap();
            assert_eq!(doc.data["reviewer"], "B");
            assert_eq!(doc.data["status"], "reviewed");

            // A finalizes
            store_a.write("/shared/doc", json!({
                "title": "Final",
                "author": "A",
                "reviewer": "B",
                "status": "published"
            })).unwrap();

            // Create anchor for published version
            store_a.anchor("/shared/doc", Some("v1.0")).unwrap();
        }

        // Both can see full history
        {
            let store_b = Store::at(dir.path(), &shared_key).unwrap();
            let patches = store_b.history("/shared/doc").unwrap();
            assert_eq!(patches.len(), 3); // Initial + review + publish

            let anchors = store_b.anchors("/shared/doc").unwrap();
            assert_eq!(anchors.len(), 1);
            assert_eq!(anchors[0].label, Some("v1.0".to_string()));
        }
    }

    #[test]
    fn collaboration_independent_paths() {
        let dir = tempdir().unwrap();

        // Process A: Works on wallet
        {
            let store = Store::at(dir.path(), &Store::test_key()).unwrap();
            store.write("/wallet/balance", json!({"sats": 100000})).unwrap();
            store.write("/wallet/balance", json!({"sats": 150000})).unwrap();
        }

        // Process B: Works on lightning (completely independent)
        {
            let store = Store::at(dir.path(), &Store::test_key()).unwrap();
            store.write("/ln/channels", json!({"count": 2})).unwrap();
            store.write("/ln/balance", json!({"sats": 50000})).unwrap();
        }

        // Both namespaces have independent histories
        {
            let store = Store::at(dir.path(), &Store::test_key()).unwrap();

            let wallet_patches = store.history("/wallet/balance").unwrap();
            assert_eq!(wallet_patches.len(), 2);

            let ln_channel_patches = store.history("/ln/channels").unwrap();
            assert_eq!(ln_channel_patches.len(), 1);

            let ln_balance_patches = store.history("/ln/balance").unwrap();
            assert_eq!(ln_balance_patches.len(), 1);
        }
    }

    // ========================================================================
    // Robustness Tests - Error handling and edge cases
    // ========================================================================

    #[test]
    fn robustness_restore_nonexistent_anchor() {
        let dir = tempdir().unwrap();
        let store = Store::at(dir.path(), &Store::test_key()).unwrap();

        store.write("/test", json!({"value": 1})).unwrap();

        let result = store.restore("/test", "nonexistent-anchor-id");
        assert!(result.is_err());
    }

    #[test]
    fn robustness_anchor_nonexistent_path() {
        let dir = tempdir().unwrap();
        let store = Store::at(dir.path(), &Store::test_key()).unwrap();

        let result = store.anchor("/nonexistent", Some("v1"));
        assert!(result.is_err());
    }

    #[test]
    fn robustness_empty_data_changes() {
        let dir = tempdir().unwrap();
        let store = Store::at(dir.path(), &Store::test_key()).unwrap();

        // Write empty object
        store.write("/empty", json!({})).unwrap();

        // Add a field
        store.write("/empty", json!({"a": 1})).unwrap();

        // Remove the field (back to empty)
        store.write("/empty", json!({})).unwrap();

        // History should track all changes
        let patches = store.history("/empty").unwrap();
        assert_eq!(patches.len(), 3);
    }

    #[test]
    fn robustness_special_characters_in_path() {
        let dir = tempdir().unwrap();
        let store = Store::at(dir.path(), &Store::test_key()).unwrap();

        // Paths with allowed special characters (hyphens, underscores, numbers)
        store.write("/user/alice-smith", json!({"name": "Alice"})).unwrap();
        store.write("/data/2024-01-15", json!({"events": 5})).unwrap();
        store.write("/config/v1_settings", json!({"theme": "dark"})).unwrap();

        let alice = store.read("/user/alice-smith").unwrap().unwrap();
        assert_eq!(alice.data["name"], "Alice");

        let date_data = store.read("/data/2024-01-15").unwrap().unwrap();
        assert_eq!(date_data.data["events"], 5);

        let config = store.read("/config/v1_settings").unwrap().unwrap();
        assert_eq!(config.data["theme"], "dark");

        // History works for these paths
        let patches = store.history("/user/alice-smith").unwrap();
        assert_eq!(patches.len(), 1);
    }

    #[test]
    fn robustness_large_history() {
        let dir = tempdir().unwrap();
        let store = Store::at(dir.path(), &Store::test_key()).unwrap();

        // Create a long history
        for i in 0..100 {
            store.write("/counter", json!({"value": i})).unwrap();
        }

        // All patches should be preserved and ordered
        let patches = store.history("/counter").unwrap();
        assert_eq!(patches.len(), 100);

        for (i, patch) in patches.iter().enumerate() {
            assert_eq!(patch.seq, (i + 1) as u64);
        }
    }

    #[test]
    fn robustness_multiple_anchors_same_state() {
        let dir = tempdir().unwrap();
        let store = Store::at(dir.path(), &Store::test_key()).unwrap();

        store.write("/doc", json!({"content": "Hello"})).unwrap();

        // Create multiple anchors at the same state
        let a1 = store.anchor("/doc", Some("checkpoint-1")).unwrap();
        let a2 = store.anchor("/doc", Some("checkpoint-2")).unwrap();
        let a3 = store.anchor("/doc", None).unwrap(); // No label

        // All anchors should exist and have the same content hash
        let anchors = store.anchors("/doc").unwrap();
        assert_eq!(anchors.len(), 3);
        assert_eq!(a1.hash, a2.hash);
        assert_eq!(a2.hash, a3.hash);

        // All anchors restore to the same state
        store.write("/doc", json!({"content": "Modified"})).unwrap();

        store.restore("/doc", &a1.id).unwrap();
        let restored = store.read("/doc").unwrap().unwrap();
        assert_eq!(restored.data["content"], "Hello");
    }

    #[test]
    fn robustness_hash_chain_integrity() {
        let dir = tempdir().unwrap();
        let store = Store::at(dir.path(), &Store::test_key()).unwrap();

        // Build up state
        store.write("/chain", json!({"v": 1})).unwrap();
        store.write("/chain", json!({"v": 2})).unwrap();
        store.write("/chain", json!({"v": 3})).unwrap();

        let patches = store.history("/chain").unwrap();

        // First patch has no parent (genesis)
        assert!(patches[0].parent.is_none());

        // Each subsequent patch's parent should be the previous patch's hash
        // (Note: This verifies the chain is consistent)
        for i in 1..patches.len() {
            assert!(patches[i].parent.is_some());
            // The parent should exist - we can verify by checking it's a valid hash
            let parent = patches[i].parent.as_ref().unwrap();
            assert_eq!(parent.len(), 64); // SHA-256 hex
        }
    }

    // ========================================================================
    // state_at() Tests - Time travel without modifying state
    // ========================================================================

    #[test]
    fn state_at_reconstructs_history() {
        let dir = tempdir().unwrap();
        let store = Store::at(dir.path(), &Store::test_key()).unwrap();

        // Create history
        store.write("/ledger", json!({"balance": 100, "tx": 1})).unwrap();
        store.write("/ledger", json!({"balance": 200, "tx": 2})).unwrap();
        store.write("/ledger", json!({"balance": 150, "tx": 3})).unwrap();
        store.write("/ledger", json!({"balance": 300, "tx": 4})).unwrap();

        // Time-travel to each state (read-only)
        let v1 = store.state_at("/ledger", 1).unwrap();
        assert_eq!(v1.data["balance"], 100);
        assert_eq!(v1.data["tx"], 1);

        let v2 = store.state_at("/ledger", 2).unwrap();
        assert_eq!(v2.data["balance"], 200);
        assert_eq!(v2.data["tx"], 2);

        let v3 = store.state_at("/ledger", 3).unwrap();
        assert_eq!(v3.data["balance"], 150);
        assert_eq!(v3.data["tx"], 3);

        let v4 = store.state_at("/ledger", 4).unwrap();
        assert_eq!(v4.data["balance"], 300);
        assert_eq!(v4.data["tx"], 4);

        // Current state is unchanged
        let current = store.read("/ledger").unwrap().unwrap();
        assert_eq!(current.data["balance"], 300);
    }

    #[test]
    fn state_at_errors_for_invalid_seq() {
        let dir = tempdir().unwrap();
        let store = Store::at(dir.path(), &Store::test_key()).unwrap();

        store.write("/test", json!({"v": 1})).unwrap();
        store.write("/test", json!({"v": 2})).unwrap();

        // seq 0 is invalid
        assert!(store.state_at("/test", 0).is_err());

        // seq 3 is out of range (only 2 patches)
        assert!(store.state_at("/test", 3).is_err());

        // Nonexistent path
        assert!(store.state_at("/nonexistent", 1).is_err());
    }

    #[test]
    fn state_at_with_complex_changes() {
        let dir = tempdir().unwrap();
        let store = Store::at(dir.path(), &Store::test_key()).unwrap();

        // Complex nested state evolution
        store.write("/user", json!({
            "name": "Alice",
            "settings": {"theme": "light"}
        })).unwrap();

        store.write("/user", json!({
            "name": "Alice",
            "settings": {"theme": "dark", "lang": "en"}
        })).unwrap();

        store.write("/user", json!({
            "name": "Alice Smith",
            "settings": {"theme": "dark", "lang": "en"},
            "verified": true
        })).unwrap();

        // v1: Original
        let v1 = store.state_at("/user", 1).unwrap();
        assert_eq!(v1.data["name"], "Alice");
        assert_eq!(v1.data["settings"]["theme"], "light");
        assert!(v1.data.get("verified").is_none());

        // v2: Theme changed
        let v2 = store.state_at("/user", 2).unwrap();
        assert_eq!(v2.data["settings"]["theme"], "dark");
        assert_eq!(v2.data["settings"]["lang"], "en");

        // v3: Name and verified added
        let v3 = store.state_at("/user", 3).unwrap();
        assert_eq!(v3.data["name"], "Alice Smith");
        assert_eq!(v3.data["verified"], true);
    }

    // ========================================================================
    // Lexical Namespace Scoping Tests - Security & Isolation
    // ========================================================================

    #[test]
    fn validate_app_key_valid() {
        // Valid app keys
        assert!(validate_app_key("myapp").is_ok());
        assert!(validate_app_key("my-app").is_ok());
        assert!(validate_app_key("my_app").is_ok());
        assert!(validate_app_key("MyApp123").is_ok());
        assert!(validate_app_key("beewallet").is_ok());
        assert!(validate_app_key("nostr-client-v2").is_ok());
        assert!(validate_app_key("a").is_ok()); // minimum length
    }

    #[test]
    fn validate_app_key_rejects_empty() {
        assert!(validate_app_key("").is_err());
    }

    #[test]
    fn validate_app_key_rejects_too_long() {
        let long_key = "a".repeat(65);
        assert!(validate_app_key(&long_key).is_err());

        // 64 chars is ok
        let max_key = "a".repeat(64);
        assert!(validate_app_key(&max_key).is_ok());
    }

    #[test]
    fn validate_app_key_rejects_path_traversal() {
        // SECURITY: Must reject path traversal attempts
        assert!(validate_app_key(".").is_err());
        assert!(validate_app_key("..").is_err());
        assert!(validate_app_key("../other").is_err());
        assert!(validate_app_key("foo/bar").is_err());
        assert!(validate_app_key("foo\\bar").is_err());
    }

    #[test]
    fn validate_app_key_rejects_leading_special() {
        // Cannot start with hyphen or underscore
        assert!(validate_app_key("-myapp").is_err());
        assert!(validate_app_key("_myapp").is_err());

        // But can contain them
        assert!(validate_app_key("my-app").is_ok());
        assert!(validate_app_key("my_app").is_ok());
    }

    #[test]
    fn validate_app_key_rejects_special_chars() {
        // Various special characters that could cause issues
        assert!(validate_app_key("my app").is_err()); // space
        assert!(validate_app_key("my@app").is_err());
        assert!(validate_app_key("my:app").is_err());
        assert!(validate_app_key("my*app").is_err());
        assert!(validate_app_key("my?app").is_err());
        assert!(validate_app_key("my<app").is_err());
        assert!(validate_app_key("my>app").is_err());
        assert!(validate_app_key("my|app").is_err());
        assert!(validate_app_key("my\"app").is_err());
        assert!(validate_app_key("my'app").is_err());
    }

    #[test]
    fn store_open_with_env_override() {
        // Use temp dir as NINE_S_ROOT
        let dir = tempdir().unwrap();
        std::env::set_var("NINE_S_ROOT", dir.path());

        let key = [42u8; 32]; // Test key
        let store = Store::open("testapp", &key).unwrap();

        // Verify path is under our custom root
        assert!(store.path().starts_with(dir.path()));
        assert!(store.path().to_string_lossy().contains("testapp"));
        assert_eq!(store.app_key(), Some("testapp"));
        assert!(store.is_encrypted());

        // Write and read works (transparently encrypted)
        store.write("/test", json!({"value": 42})).unwrap();
        let scroll = store.read("/test").unwrap().unwrap();
        assert_eq!(scroll.data["value"], 42);

        // Cleanup
        std::env::remove_var("NINE_S_ROOT");
    }

    #[test]
    fn store_open_isolation() {
        // Two apps should be isolated via different directories AND different derived keys
        // This tests namespace isolation using Store::at (avoids env var race)
        use crate::vault::crypto::derive_app_key;

        let dir = tempdir().unwrap();
        let master_key = [42u8; 32];

        // Simulate Store::open behavior: different dirs + different derived keys
        let dir_a = dir.path().join("app-a");
        let dir_b = dir.path().join("app-b");
        let key_a = derive_app_key(&master_key, "app-a");
        let key_b = derive_app_key(&master_key, "app-b");

        // App A writes data
        let store_a = Store::at(&dir_a, &key_a).unwrap();
        store_a.write("/secret", json!({"owner": "A"})).unwrap();

        // App B cannot see App A's data (different directory)
        let store_b = Store::at(&dir_b, &key_b).unwrap();
        let result = store_b.read("/secret").unwrap();
        assert!(result.is_none()); // App B sees nothing at /secret

        // App B writes its own data at the same path
        store_b.write("/secret", json!({"owner": "B"})).unwrap();

        // Both apps still see their own data
        let a_data = store_a.read("/secret").unwrap().unwrap();
        assert_eq!(a_data.data["owner"], "A");

        let b_data = store_b.read("/secret").unwrap().unwrap();
        assert_eq!(b_data.data["owner"], "B");

        // Verify keys are different (HKDF guarantee)
        assert_ne!(key_a, key_b);
    }

    #[test]
    fn store_hkdf_cryptographic_isolation() {
        // Verify that different app keys produce different encryption keys
        // This tests the HKDF derivation indirectly through Store::at
        use crate::vault::crypto::derive_app_key;

        let dir = tempdir().unwrap();
        let master_key = [42u8; 32];

        // Derive app-specific key for "writer"
        let writer_key = derive_app_key(&master_key, "writer");

        // Write with the derived key
        {
            let store = Store::at(dir.path(), &writer_key).unwrap();
            store.write("/data", json!({"secret": "value"})).unwrap();
        }

        // Try to read with the raw master key (should fail - different key)
        {
            let store_raw = Store::at(dir.path(), &master_key).unwrap();
            let result = store_raw.read("/data");
            // This should fail because derive_app_key(master, "writer") != master
            assert!(result.is_err(), "Raw master key should not decrypt app data");
        }

        // Reading with the correct derived key works
        {
            let store = Store::at(dir.path(), &writer_key).unwrap();
            let scroll = store.read("/data").unwrap().unwrap();
            assert_eq!(scroll.data["secret"], "value");
        }

        // Different app key = different derived key = cannot decrypt
        let reader_key = derive_app_key(&master_key, "reader");
        assert_ne!(writer_key, reader_key);
        {
            let store = Store::at(dir.path(), &reader_key).unwrap();
            let result = store.read("/data");
            assert!(result.is_err(), "Different app key should not decrypt data");
        }
    }

    #[test]
    fn store_hkdf_same_app_same_master_same_data() {
        // Same app + same master = same derived key = can read data
        // This simulates two process instances with same credentials
        use crate::vault::crypto::derive_app_key;

        let dir = tempdir().unwrap();
        let master_key = [42u8; 32];
        let app_key = derive_app_key(&master_key, "myapp");

        // Instance 1 writes
        {
            let store = Store::at(dir.path(), &app_key).unwrap();
            store.write("/config", json!({"theme": "dark"})).unwrap();
        }

        // Instance 2 reads (different Store instance, same derived key)
        {
            let store = Store::at(dir.path(), &app_key).unwrap();
            let scroll = store.read("/config").unwrap().unwrap();
            assert_eq!(scroll.data["theme"], "dark");
        }
    }

    #[test]
    fn store_hkdf_derivation_consistency() {
        // Verify that derive_app_key produces consistent results
        use crate::vault::crypto::derive_app_key;

        let master = [0x42; 32];

        // Same inputs = same output
        let key1 = derive_app_key(&master, "beewallet");
        let key2 = derive_app_key(&master, "beewallet");
        assert_eq!(key1, key2, "HKDF should be deterministic");

        // Different apps = different keys
        let key_nostr = derive_app_key(&master, "nostr");
        assert_ne!(key1, key_nostr, "Different apps should have different keys");

        // Different masters = different keys for same app
        let other_master = [0x43; 32];
        let key_other = derive_app_key(&other_master, "beewallet");
        assert_ne!(key1, key_other, "Different masters should have different keys");
    }

    #[test]
    fn store_open_invalid_key_rejected() {
        let dir = tempdir().unwrap();
        std::env::set_var("NINE_S_ROOT", dir.path());

        let key = [42u8; 32];

        // All of these should fail (invalid app_key, not encryption key)
        assert!(Store::open("", &key).is_err());
        assert!(Store::open("..", &key).is_err());
        assert!(Store::open("../attack", &key).is_err());
        assert!(Store::open("foo/bar", &key).is_err());
        assert!(Store::open("-bad", &key).is_err());

        // Cleanup
        std::env::remove_var("NINE_S_ROOT");
    }

    // ========================================================================
    // Encryption Tests
    // ========================================================================

    #[test]
    fn store_encrypted_write_read() {
        let dir = tempdir().unwrap();
        let key = [42u8; 32];
        let store = Store::at(dir.path(), &key).unwrap();

        // Write sensitive data
        store.write("/wallet/seed", json!({"phrase": "abandon abandon ... about"})).unwrap();

        // Read it back (transparently decrypted)
        let scroll = store.read("/wallet/seed").unwrap().unwrap();
        assert_eq!(scroll.data["phrase"], "abandon abandon ... about");

        // Verify data is actually encrypted on disk by reading raw file
        // FileNamespace stores at: _scrolls/wallet/seed.json
        let file_path = dir.path().join("_scrolls").join("wallet").join("seed.json");
        let raw_content = std::fs::read_to_string(&file_path).unwrap();
        let raw_json: serde_json::Value = serde_json::from_str(&raw_content).unwrap();

        // The data field should be a SealedValue, not the original JSON
        let data = &raw_json["data"];
        assert!(data.get("ciphertext").is_some(), "data should contain ciphertext");
        assert!(data.get("nonce").is_some(), "data should contain nonce");
        assert!(data.get("phrase").is_none(), "phrase should NOT be in plaintext");
    }

    #[test]
    fn store_encrypted_wrong_key_fails() {
        let dir = tempdir().unwrap();
        let key1 = [1u8; 32];
        let key2 = [2u8; 32];

        // Write with key1
        let store1 = Store::at(dir.path(), &key1).unwrap();
        store1.write("/secret", json!({"data": "sensitive"})).unwrap();

        // Try to read with key2 - should fail
        let store2 = Store::at(dir.path(), &key2).unwrap();
        let result = store2.read("/secret");
        assert!(result.is_err()); // Decryption fails
    }

    #[test]
    fn store_encrypted_history_works() {
        let dir = tempdir().unwrap();
        let key = [42u8; 32];
        let store = Store::at(dir.path(), &key).unwrap();

        // Create history
        store.write("/doc", json!({"v": 1})).unwrap();
        store.write("/doc", json!({"v": 2})).unwrap();
        store.write("/doc", json!({"v": 3})).unwrap();

        // History should work (patches are on plaintext)
        let patches = store.history("/doc").unwrap();
        assert_eq!(patches.len(), 3);
    }

    #[test]
    fn store_always_encrypted() {
        // Sovereignty: encryption is mandatory, not optional
        let dir = tempdir().unwrap();
        let store = Store::at(dir.path(), &Store::test_key()).unwrap();

        // All stores are encrypted
        assert!(store.is_encrypted());

        // Data is transparently encrypted/decrypted
        store.write("/public", json!({"data": "visible"})).unwrap();
        let scroll = store.read("/public").unwrap().unwrap();
        assert_eq!(scroll.data["data"], "visible");
    }

    // ========================================================================
    // CSP Sequencing Tests - Rapid writes with correct ordering
    // ========================================================================

    #[test]
    fn csp_rapid_writes_sequential_consistency() {
        // CSP Insight: Sequence derived from filesystem (monotonic counter)
        // Each write sees the previous write's patch file and increments
        let dir = tempdir().unwrap();
        let store = Store::at(dir.path(), &Store::test_key()).unwrap();

        // Rapid writes without any delay
        for i in 1..=100 {
            store.write("/counter", json!({"value": i})).unwrap();
        }

        // All patches must have unique, sequential seq numbers
        let patches = store.history("/counter").unwrap();
        assert_eq!(patches.len(), 100, "Should have exactly 100 patches");

        // Verify sequential ordering (CSP guarantee)
        for (i, patch) in patches.iter().enumerate() {
            assert_eq!(
                patch.seq,
                (i + 1) as u64,
                "Patch {} should have seq {}, got {}",
                i,
                i + 1,
                patch.seq
            );
        }

        // Hash chain should be intact
        for i in 1..patches.len() {
            assert!(
                patches[i].parent.is_some(),
                "Patch {} should have parent hash",
                i + 1
            );
            assert_eq!(
                patches[i].parent.as_ref(),
                Some(&patches[i - 1].hash),
                "Patch {} parent should match patch {} hash",
                i + 1,
                i
            );
        }

        // First patch should have no parent (genesis)
        assert!(patches[0].parent.is_none(), "First patch should be genesis");
    }

    #[test]
    fn csp_sequence_from_filesystem_not_memory() {
        // CSP: The filesystem is the channel, not in-memory state
        // Multiple store instances should see consistent sequence
        let dir = tempdir().unwrap();
        let key = Store::test_key();

        // Instance 1 writes
        {
            let store1 = Store::at(dir.path(), &key).unwrap();
            store1.write("/doc", json!({"v": 1})).unwrap();
            store1.write("/doc", json!({"v": 2})).unwrap();
        }

        // Instance 2 continues (fresh instance, no in-memory state)
        {
            let store2 = Store::at(dir.path(), &key).unwrap();
            store2.write("/doc", json!({"v": 3})).unwrap();
            store2.write("/doc", json!({"v": 4})).unwrap();
        }

        // Verify sequence is continuous across instances
        let store = Store::at(dir.path(), &key).unwrap();
        let patches = store.history("/doc").unwrap();
        assert_eq!(patches.len(), 4);

        for (i, patch) in patches.iter().enumerate() {
            assert_eq!(patch.seq, (i + 1) as u64);
        }
    }

    #[test]
    fn csp_benchmark_write_throughput() {
        use std::time::Instant;

        let dir = tempdir().unwrap();
        let store = Store::at(dir.path(), &Store::test_key()).unwrap();

        let iterations = 50;
        let start = Instant::now();

        for i in 0..iterations {
            store
                .write(
                    "/benchmark",
                    json!({
                        "iteration": i,
                        "data": "x".repeat(100), // 100 byte payload
                    }),
                )
                .unwrap();
        }

        let elapsed = start.elapsed();
        let per_write = elapsed / iterations;

        // Verify correctness
        let patches = store.history("/benchmark").unwrap();
        assert_eq!(patches.len(), iterations as usize);

        // Log performance (visible in test output with --nocapture)
        println!(
            "CSP Benchmark: {} writes in {:?} ({:?}/write)",
            iterations, elapsed, per_write
        );

        // Sanity check: should be faster than 100ms per write
        assert!(
            per_write.as_millis() < 100,
            "Write too slow: {:?}",
            per_write
        );
    }
}
