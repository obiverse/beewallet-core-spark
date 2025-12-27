//! WalletNamespace - Bitcoin + Lightning as a 9S Namespace
//!
//! Provides the same API as beewallet-core-breez WalletNamespace
//! for drop-in replacement compatibility.
//!
//! # Modes
//!
//! 1. **Orchestrated** (with_wallet): BeeShell manages connection
//! 2. **Standalone** (new): Namespace handles lazy connection
//!
//! # Paths
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
use super::{SparkNetwork, WalletConfig, WalletError, WalletManager};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tokio::runtime::Handle;

/// WalletNamespace - Bitcoin + Lightning wallet as 9S Namespace
///
/// Two modes:
/// 1. **Orchestrated** (with_wallet): BeeShell manages connection
/// 2. **Standalone** (new): Namespace handles lazy connection
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

    /// Block on async operation using runtime handle
    fn block_on<F, T>(&self, future: F) -> std::result::Result<T, WalletError>
    where
        F: std::future::Future<Output = std::result::Result<T, WalletError>>,
    {
        if let Some(ref handle) = self.runtime_handle {
            return handle.block_on(future);
        }

        Handle::try_current()
            .map_err(|_| WalletError::Sdk(
                "No runtime available. Use with_wallet() with a runtime handle.".into()
            ))?
            .block_on(future)
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

        let mnemonic = {
            let guard = self.mnemonic.read().map_err(|e| Error::Internal(e.to_string()))?;
            guard.clone().ok_or_else(|| Error::Unavailable("Wallet not unlocked".into()))?
        };

        let config = WalletConfig::new(self.network)
            .with_working_dir(self.data_dir.join("wallet"));

        let config = if let Some(ref key) = self.api_key {
            config.with_api_key(key.clone())
        } else {
            config
        };

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
                .map(|w| w.is_some())
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

        // Everything else needs wallet
        let guard = self.require_wallet()?;
        let wallet = guard.as_ref().ok_or_else(|| Error::Unavailable("Wallet not available".into()))?;

        match path {
            "/balance" => {
                let balance = wallet.balance()
                    .map_err(|e| Error::Internal(e.to_string()))?;
                Ok(Some(Scroll::typed(
                    "/wallet/balance",
                    json!({
                        "confirmed": balance.confirmed,
                        "trusted_pending": balance.trusted_pending,
                        "untrusted_pending": balance.untrusted_pending,
                        "immature": balance.immature,
                        "total": balance.total(),
                        "spendable": balance.spendable(),
                    }),
                    "wallet/balance@v1",
                )))
            }
            "/address" => {
                let address = wallet.new_address()
                    .map_err(|e| Error::Internal(e.to_string()))?;
                Ok(Some(Scroll::typed(
                    "/wallet/address",
                    json!({"address": address}),
                    "wallet/address@v1",
                )))
            }
            "/network" => {
                Ok(Some(Scroll::typed(
                    "/wallet/network",
                    json!({"network": self.network_str()}),
                    "wallet/network@v1",
                )))
            }
            "/pubkey" => {
                let pubkey = wallet.pubkey()
                    .map_err(|e| Error::Internal(e.to_string()))?;
                Ok(Some(Scroll::typed(
                    "/wallet/pubkey",
                    json!({"pubkey": pubkey}),
                    "wallet/pubkey@v1",
                )))
            }
            "/transactions" => {
                let txs = wallet.transactions(50)
                    .map_err(|e| Error::Internal(e.to_string()))?;

                let tx_list: Vec<Value> = txs.iter().map(|tx| json!({
                    "txid": tx.txid,
                    "received": tx.received,
                    "sent": tx.sent,
                    "fee": tx.fee,
                    "is_confirmed": tx.is_confirmed,
                    "timestamp": tx.timestamp,
                })).collect();

                Ok(Some(Scroll::typed(
                    "/wallet/transactions",
                    json!({"transactions": tx_list}),
                    "wallet/transactions@v1",
                )))
            }
            _ => Ok(None),
        }
    }

    fn write(&self, path: &str, data: Value) -> Result<Scroll> {
        let guard = self.require_wallet()?;
        let wallet = guard.as_ref().ok_or_else(|| Error::Unavailable("Wallet not available".into()))?;

        match path {
            "/send" => {
                let destination = data["destination"].as_str()
                    .or_else(|| data["to"].as_str())
                    .ok_or_else(|| Error::InvalidData("Missing destination".into()))?;
                let amount = data["amount"].as_u64()
                    .or_else(|| data["amount_sat"].as_u64())
                    .ok_or_else(|| Error::InvalidData("Missing amount".into()))?;
                let fee_rate = data["fee_rate"].as_f64();

                let tx_id = wallet.send(destination, amount, fee_rate)
                    .map_err(|e| Error::Internal(e.to_string()))?;

                Ok(Scroll::typed(
                    &format!("/wallet/tx/{}", tx_id),
                    json!({
                        "txid": tx_id,
                        "amount": -(amount as i64),
                        "status": "pending",
                    }),
                    "wallet/tx@v1",
                ))
            }
            "/invoice" | "/receive" => {
                let amount_sat = data["amountSat"].as_u64()
                    .or_else(|| data["amount_sat"].as_u64())
                    .ok_or_else(|| Error::InvalidData("Missing amountSat".into()))?;
                let description = data["description"].as_str();

                let invoice = wallet.create_invoice(amount_sat, description)
                    .map_err(|e| Error::Internal(e.to_string()))?;

                Ok(Scroll::typed(
                    "/wallet/invoice",
                    json!({
                        "destination": invoice,
                        "amountSat": amount_sat,
                    }),
                    "wallet/invoice@v1",
                ))
            }
            "/fee-estimate" => {
                let destination = data["destination"].as_str()
                    .or_else(|| data["to"].as_str())
                    .ok_or_else(|| Error::InvalidData("Missing destination".into()))?;
                let amount = data["amount"].as_u64()
                    .or_else(|| data["amount_sat"].as_u64())
                    .unwrap_or(0);

                let fee = wallet.estimate_fee(destination, amount)
                    .map_err(|e| Error::Internal(e.to_string()))?;

                Ok(Scroll::typed(
                    "/wallet/fee-estimate",
                    json!({
                        "fee": fee,
                        "destination": destination,
                        "amount": amount,
                    }),
                    "wallet/fee-estimate@v1",
                ))
            }
            _ => Err(Error::NotFound(path.into())),
        }
    }

    fn list(&self, prefix: &str) -> Result<Vec<String>> {
        match prefix {
            "/" | "" => Ok(vec![
                "/wallet/status".to_string(),
                "/wallet/balance".to_string(),
                "/wallet/address".to_string(),
                "/wallet/network".to_string(),
                "/wallet/transactions".to_string(),
            ]),
            _ => Ok(vec![]),
        }
    }

    fn watch(&self, _pattern: &str) -> Result<Receiver<Scroll>> {
        Err(Error::Unavailable("Wallet watch not yet implemented".into()))
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
}
