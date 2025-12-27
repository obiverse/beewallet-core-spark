//! WalletNamespace - Thin adapter for BeeShell orchestration
//!
//! This module provides a `WalletNamespace` that wraps a shared `WalletManager`.
//! Since WalletManager now implements `Namespace` directly (the 9S Way),
//! WalletNamespace is primarily an adapter for the orchestrated mode pattern
//! where BeeShell manages wallet lifecycle.
//!
//! # Modes
//!
//! 1. **Orchestrated** (with_wallet): BeeShell manages connection
//! 2. **Standalone** (new): Namespace handles lazy connection
//!
//! # Why This Exists
//!
//! BeeShell shares wallet state across multiple namespaces. The
//! `Arc<RwLock<Option<WalletManager>>>` pattern allows:
//!
//! - BeeShell to create/destroy wallet on unlock/lock
//! - WalletNamespace to route operations to shared wallet
//! - Multiple namespaces to share same wallet instance
//!
//! # Architecture (The 9S Way)
//!
//! ```text
//! BeeShell
//!     │
//!     ├── wallet: Arc<RwLock<Option<WalletManager>>>
//!     │
//!     └── mount("/wallet", WalletNamespace::with_wallet(wallet.clone()))
//!
//! WalletNamespace::read("/balance")
//!     └── wallet.read().unwrap().read("/balance")
//!         └── WalletManager::read("/balance")  // The real implementation
//! ```
//!
//! # Paths
//!
//! All paths delegate to WalletManager's Namespace implementation:
//!
//! | Path | Method | Description |
//! |------|--------|-------------|
//! | `/status` | read | Connection status (always works) |
//! | `/balance` | read | Get balance |
//! | `/address` | read | Get receive address |
//! | `/network` | read | Get network |
//! | `/transactions` | read | List recent transactions |
//! | `/send` | write | Send to destination |
//! | `/invoice` | write | Create Lightning invoice |

use crate::nine_s::{Error, Namespace, Receiver, Result, Scroll};
use super::{SparkNetwork, WalletConfig, WalletManager};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tokio::runtime::Handle;

/// WalletNamespace - Thin adapter for BeeShell orchestration
///
/// This wraps a shared WalletManager reference, delegating all Namespace
/// operations to the underlying WalletManager's Namespace implementation.
///
/// ## Design (The 9S Way)
///
/// WalletManager implements Namespace directly. WalletNamespace exists
/// solely to manage the shared `Arc<RwLock<Option<WalletManager>>>` pattern
/// used by BeeShell for wallet lifecycle orchestration.
pub struct WalletNamespace {
    /// Connected wallet manager (shared or owned)
    wallet: Arc<RwLock<Option<WalletManager>>>,
    /// Mnemonic for connection (only used in standalone mode)
    mnemonic: Arc<RwLock<Option<String>>>,
    /// Working directory for wallet data
    data_dir: PathBuf,
    /// Network configuration
    network: SparkNetwork,
    /// API key for Breez SDK
    api_key: Option<String>,
    /// Whether this namespace manages its own connection
    self_managed: bool,
    /// Runtime handle for async operations
    runtime_handle: Option<Handle>,
}

impl WalletNamespace {
    /// Create a new standalone WalletNamespace
    ///
    /// Wallet connects lazily on first operation that needs it.
    pub fn new(
        mnemonic: Arc<RwLock<Option<String>>>,
        data_dir: PathBuf,
        network: SparkNetwork,
        api_key: Option<String>,
    ) -> Self {
        Self {
            wallet: Arc::new(RwLock::new(None)),
            mnemonic,
            data_dir,
            network,
            api_key,
            self_managed: true,
            runtime_handle: None,
        }
    }

    /// Create with external wallet reference (for BeeShell orchestration)
    ///
    /// BeeShell manages the wallet connection lifecycle. This namespace
    /// just routes operations to the shared wallet.
    pub fn with_wallet(
        wallet: Arc<RwLock<Option<WalletManager>>>,
        mnemonic: Arc<RwLock<Option<String>>>,
        data_dir: PathBuf,
        network: SparkNetwork,
        api_key: Option<String>,
        runtime_handle: Handle,
    ) -> Self {
        Self {
            wallet,
            mnemonic,
            data_dir,
            network,
            api_key,
            self_managed: false,
            runtime_handle: Some(runtime_handle),
        }
    }

    /// Ensure wallet is connected
    fn ensure_connected(&self) -> Result<()> {
        {
            let guard = self.wallet.read().map_err(|e| Error::Internal(e.to_string()))?;
            if guard.is_some() {
                return Ok(());
            }
        }

        if !self.self_managed {
            return Err(Error::Unavailable(
                "Wallet not connected. BeeShell should connect on unlock.".into()
            ));
        }

        let _mnemonic = {
            let guard = self.mnemonic.read().map_err(|e| Error::Internal(e.to_string()))?;
            guard.clone().ok_or_else(|| Error::Unavailable("Wallet not unlocked".into()))?
        };

        let _config = WalletConfig::new(self.network)
            .with_working_dir(self.data_dir.join("wallet"));

        // TODO: Implement actual connection when Spark SDK is wired
        // For now, return error indicating scaffold
        Err(Error::Unavailable("Spark SDK connection not yet implemented".into()))
    }

    /// Get wallet reference, ensuring connected
    fn require_wallet(&self) -> Result<std::sync::RwLockReadGuard<'_, Option<WalletManager>>> {
        self.ensure_connected()?;
        self.wallet.read().map_err(|e| Error::Internal(e.to_string()))
    }

    /// Convert network to string
    fn network_str(&self) -> &'static str {
        match self.network {
            SparkNetwork::Mainnet => "bitcoin",
            SparkNetwork::Testnet => "testnet",
            SparkNetwork::Regtest => "regtest",
        }
    }
}

impl Namespace for WalletNamespace {
    fn read(&self, path: &str) -> Result<Option<Scroll>> {
        // Status check doesn't require connection
        if path == "/status" || path.is_empty() {
            let connected = self.wallet.read()
                .map(|w| w.as_ref().map(|m| m.is_connected()).unwrap_or(false))
                .unwrap_or(false);
            return Ok(Some(Scroll::typed(
                "/wallet/status",
                json!({
                    "connected": connected,
                    "network": self.network_str(),
                    "backend": "spark",
                }),
                "wallet/status@v1",
            )));
        }

        // Everything else delegates to WalletManager's Namespace impl
        let guard = self.require_wallet()?;
        let wallet = guard.as_ref().ok_or_else(|| Error::Unavailable("Wallet not available".into()))?;

        // Delegate to WalletManager's Namespace implementation
        wallet.read(path)
    }

    fn write(&self, path: &str, data: Value) -> Result<Scroll> {
        let guard = self.require_wallet()?;
        let wallet = guard.as_ref().ok_or_else(|| Error::Unavailable("Wallet not available".into()))?;

        // Delegate to WalletManager's Namespace implementation
        wallet.write(path, data)
    }

    fn list(&self, prefix: &str) -> Result<Vec<String>> {
        // List works even without connection
        match prefix {
            "/" | "" => Ok(vec![
                "/wallet/status".to_string(),
                "/wallet/balance".to_string(),
                "/wallet/address".to_string(),
                "/wallet/network".to_string(),
                "/wallet/transactions".to_string(),
            ]),
            _ => {
                // If connected, delegate to WalletManager
                if let Ok(guard) = self.wallet.read() {
                    if let Some(wallet) = guard.as_ref() {
                        return wallet.list(prefix);
                    }
                }
                Ok(vec![])
            }
        }
    }

    fn watch(&self, pattern: &str) -> Result<Receiver<Scroll>> {
        let guard = self.require_wallet()?;
        let wallet = guard.as_ref().ok_or_else(|| Error::Unavailable("Wallet not available".into()))?;

        // Delegate to WalletManager's Namespace implementation
        wallet.watch(pattern)
    }

    fn close(&self) -> Result<()> {
        if let Ok(mut guard) = self.wallet.write() {
            if let Some(wallet) = guard.take() {
                let _ = wallet.disconnect();
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_namespace_status_without_connection() {
        let mnemonic = Arc::new(RwLock::new(None));
        let ns = WalletNamespace::new(
            mnemonic,
            PathBuf::from("/tmp/test"),
            SparkNetwork::Testnet,
            None,
        );

        let scroll = ns.read("/status").unwrap().unwrap();
        assert_eq!(scroll.data["connected"], false);
        assert_eq!(scroll.data["network"], "testnet");
        assert_eq!(scroll.data["backend"], "spark");
    }

    #[test]
    fn test_namespace_list() {
        let mnemonic = Arc::new(RwLock::new(None));
        let ns = WalletNamespace::new(
            mnemonic,
            PathBuf::from("/tmp/test"),
            SparkNetwork::Testnet,
            None,
        );

        let paths = ns.list("/").unwrap();
        assert!(paths.contains(&"/wallet/status".to_string()));
        assert!(paths.contains(&"/wallet/balance".to_string()));
    }

    #[test]
    fn test_namespace_requires_wallet_for_balance() {
        let mnemonic = Arc::new(RwLock::new(None));
        let ns = WalletNamespace::new(
            mnemonic,
            PathBuf::from("/tmp/test"),
            SparkNetwork::Testnet,
            None,
        );

        // Balance should fail when not connected
        let result = ns.read("/balance");
        assert!(result.is_err());
    }
}
