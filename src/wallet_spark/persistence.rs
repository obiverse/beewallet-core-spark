//! Wallet Persistence - Store as the HDD
//!
//! The Store is the encrypted, persistent filesystem for wallet data.
//! The Reactor is the hot memory cache for fast access and reactivity.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────┐
//! │              WalletReactor (RAM)                 │
//! │   State Cache ◄──────────────► Event Bus        │
//! │        │                                         │
//! │        │ persist()  ▲ load()                    │
//! │        ▼            │                            │
//! ├─────────────────────────────────────────────────┤
//! │                Store (HDD)                       │
//! │   ~/.nine_s/beewallet-spark/_scrolls/           │
//! │     /wallet/balance.json.enc                    │
//! │     /wallet/tx/{txid}.json.enc                  │
//! │     /wallet/config.json.enc                     │
//! └─────────────────────────────────────────────────┘
//! ```
//!
//! ## Persistence Strategy
//!
//! - **On Event**: Write-through to Store after updating Reactor cache
//! - **On Connect**: Load from Store into Reactor cache
//! - **On Watch**: Serve from Reactor cache (instant)
//!
//! ## Data Categories
//!
//! | Path | Persistence | Description |
//! |------|-------------|-------------|
//! | `/wallet/balance` | Write-through | Always persist |
//! | `/wallet/tx/{id}` | Write-through | Transaction history |
//! | `/wallet/config` | Write-through | Wallet configuration |
//! | `/wallet/synced` | Ephemeral | Not persisted |

use crate::nine_s::{Namespace, Scroll, Store};
use super::reactor::WalletReactor;
use std::sync::Arc;

/// Paths that should be persisted to Store
const PERSISTENT_PATHS: &[&str] = &[
    "/wallet/balance",
    "/wallet/tx/",
    "/wallet/config",
    "/wallet/address",
    "/wallet/pubkey",
];

/// Paths that are ephemeral (not persisted)
const EPHEMERAL_PATHS: &[&str] = &[
    "/wallet/synced",
    "/wallet/status",
];

/// Check if a path should be persisted
fn should_persist(path: &str) -> bool {
    // Check ephemeral first
    for prefix in EPHEMERAL_PATHS {
        if path.starts_with(prefix) {
            return false;
        }
    }

    // Check persistent patterns
    for prefix in PERSISTENT_PATHS {
        if path.starts_with(prefix) {
            return true;
        }
    }

    // Default: persist wallet data
    path.starts_with("/wallet/")
}

/// Wallet persistence layer
///
/// Bridges the Reactor (hot cache) with the Store (cold storage).
pub struct WalletPersistence {
    /// The encrypted store
    store: Store,
    /// Reference to reactor for cache updates
    reactor: Arc<WalletReactor>,
}

impl WalletPersistence {
    /// Create new persistence layer
    pub fn new(store: Store, reactor: Arc<WalletReactor>) -> Self {
        Self { store, reactor }
    }

    /// Load persisted data into reactor cache
    ///
    /// Call this on wallet connect to restore state.
    pub fn load_into_cache(&self) -> Result<usize, crate::nine_s::Error> {
        let mut loaded = 0;

        // Load all wallet scrolls from store
        let paths = self.store.list("/wallet")?;

        for path in paths {
            if let Ok(Some(scroll)) = self.store.read(&path) {
                self.reactor.set_state(scroll);
                loaded += 1;
            }
        }

        Ok(loaded)
    }

    /// Persist a scroll to store and update reactor cache
    ///
    /// This is write-through: both cache and store are updated.
    pub fn persist(&self, scroll: Scroll) -> Result<(), crate::nine_s::Error> {
        // Update reactor cache first (for immediate reactivity)
        self.reactor.emit(scroll.clone());

        // Persist to store if not ephemeral
        if should_persist(&scroll.key) {
            self.store.write_scroll(scroll)?;
        }

        Ok(())
    }

    /// Persist balance update
    pub fn persist_balance(&self, confirmed: u64, pending: u64) -> Result<(), crate::nine_s::Error> {
        let scroll = Scroll::typed(
            "/wallet/balance",
            serde_json::json!({
                "confirmed": confirmed,
                "trusted_pending": pending,
                "untrusted_pending": 0,
                "immature": 0,
                "total": confirmed + pending,
                "spendable": confirmed,
            }),
            "wallet/balance@v1",
        );

        self.persist(scroll)
    }

    /// Persist a transaction
    pub fn persist_transaction(&self, txid: &str, data: serde_json::Value) -> Result<(), crate::nine_s::Error> {
        let scroll = Scroll::typed(
            &format!("/wallet/tx/{}", txid),
            data,
            "wallet/tx@v1",
        );

        self.persist(scroll)
    }

    /// Get scroll from cache or load from store
    pub fn get(&self, path: &str) -> Result<Option<Scroll>, crate::nine_s::Error> {
        // Try cache first
        if let Some(scroll) = self.reactor.get_cached(path) {
            return Ok(Some(scroll));
        }

        // Fall back to store
        if let Ok(Some(scroll)) = self.store.read(path) {
            // Populate cache
            self.reactor.set_state(scroll.clone());
            return Ok(Some(scroll));
        }

        Ok(None)
    }

    /// List paths from store
    pub fn list(&self, prefix: &str) -> Result<Vec<String>, crate::nine_s::Error> {
        self.store.list(prefix)
    }

    /// Get the underlying store for advanced operations
    pub fn store(&self) -> &Store {
        &self.store
    }

    /// Create an anchor (checkpoint) for important state
    pub fn anchor(&self, path: &str, label: Option<&str>) -> Result<crate::nine_s::Anchor, crate::nine_s::Error> {
        self.store.anchor(path, label)
    }

    /// Clear all persisted data (use with caution!)
    pub fn clear(&self) -> Result<(), crate::nine_s::Error> {
        // Clear reactor cache
        self.reactor.clear_state();

        // Delete all wallet scrolls from store
        let paths = self.store.list("/wallet")?;
        for path in paths {
            self.store.delete(&path)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_persist() {
        assert!(should_persist("/wallet/balance"));
        assert!(should_persist("/wallet/tx/abc123"));
        assert!(should_persist("/wallet/config"));
        assert!(!should_persist("/wallet/synced"));
        assert!(!should_persist("/wallet/status"));
    }

    // Tests requiring Store need the crypto feature
    #[cfg(feature = "crypto")]
    mod crypto_tests {
        use super::*;
        use tempfile::tempdir;

        #[test]
        fn test_persist_and_load() {
            let dir = tempdir().unwrap();
            let key = Store::test_key();

            // First session: persist balance
            {
                let store = Store::at(dir.path(), &key).unwrap();
                let reactor = Arc::new(WalletReactor::new());
                let persistence = WalletPersistence::new(store, reactor.clone());

                // Persist balance
                persistence.persist_balance(50000, 1000).unwrap();

                // Should be in cache
                let cached = reactor.get_cached("/wallet/balance").unwrap();
                assert_eq!(cached.data["confirmed"], 50000);
            }

            // Second session: simulate restart, load from store
            {
                let store = Store::at(dir.path(), &key).unwrap();
                let new_reactor = Arc::new(WalletReactor::new());
                let new_persistence = WalletPersistence::new(store, new_reactor.clone());

                // Load into new cache
                let loaded = new_persistence.load_into_cache().unwrap();
                assert!(loaded > 0);

                // Should be in new cache
                let cached = new_reactor.get_cached("/wallet/balance").unwrap();
                assert_eq!(cached.data["confirmed"], 50000);
            }
        }

        #[test]
        fn test_transaction_persistence() {
            let dir = tempdir().unwrap();
            let key = Store::test_key();
            let store = Store::at(dir.path(), &key).unwrap();
            let reactor = Arc::new(WalletReactor::new());
            let persistence = WalletPersistence::new(store, reactor.clone());

            // Persist transaction
            persistence.persist_transaction(
                "tx123",
                serde_json::json!({
                    "amount_sat": 21000,
                    "status": "confirmed",
                }),
            ).unwrap();

            // Should be retrievable
            let tx = persistence.get("/wallet/tx/tx123").unwrap().unwrap();
            assert_eq!(tx.data["amount_sat"], 21000);
        }

        #[test]
        fn test_ephemeral_not_persisted() {
            let dir = tempdir().unwrap();
            let key = Store::test_key();
            let store = Store::at(dir.path(), &key).unwrap();
            let reactor = Arc::new(WalletReactor::new());
            let persistence = WalletPersistence::new(store, reactor.clone());

            // Emit synced event (ephemeral)
            let scroll = Scroll::typed(
                "/wallet/synced",
                serde_json::json!({"synced": true}),
                "wallet/synced@v1",
            );
            persistence.persist(scroll).unwrap();

            // Should be in cache
            assert!(reactor.get_cached("/wallet/synced").is_some());

            // But NOT in store
            let stored = persistence.store.read("/wallet/synced").unwrap();
            assert!(stored.is_none());
        }
    }
}
