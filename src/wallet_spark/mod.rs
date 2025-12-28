//! Wallet implementation using Breez SDK Spark
//!
//! This module provides a WalletManager that wraps Breez SDK Spark,
//! offering the same API as beewallet-core-breez for interoperability.
//!
//! ## Architecture: Wallet IS a Namespace
//!
//! Following the 9S Protocol dialectics, WalletManager implements Namespace
//! directly. This means:
//!
//! 1. **Single interface**: Only Namespace trait matters
//! 2. **No duplication**: WalletBackend trait is secondary (compat only)
//! 3. **Composability**: Any Namespace can behave like a wallet
//! 4. **Testing**: Mock the namespace, not 14 individual methods
//!
//! ## Path Ontology
//!
//! | Operation | 9S Path | Data |
//! |-----------|---------|------|
//! | Get balance | `read("/balance")` | - |
//! | Get address | `read("/address")` | - |
//! | List transactions | `read("/transactions")` | - |
//! | Get single tx | `read("/tx/{txid}")` | - |
//! | Send payment | `write("/send", ...)` | `{to, amount, feeRate?}` |
//! | Create invoice | `write("/invoice", ...)` | `{amount, description?}` |
//! | Watch payments | `watch("/tx/**")` | - |
//! | Get pubkey | `read("/pubkey")` | - |
//!
//! ## Status: SCAFFOLD
//!
//! This is a placeholder implementation. Methods return NotImplemented.
//! Real implementation requires Breez SDK Spark integration.

pub mod signing;
pub mod events;
pub mod config;
pub mod namespace;
pub mod reactor;
pub mod sdk;

// Persistence requires encrypted Store (crypto feature)
#[cfg(feature = "crypto")]
pub mod persistence;

use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use serde_json::{json, Value};
use tokio::runtime::Runtime;

use crate::nine_s::{self, Namespace, Scroll};

// Re-export submodule types
pub use config::WalletConfig;
pub use events::{
    EventListener, SdkEvent, Payment, PaymentType, PaymentState, PaymentDetails,
    ClaimedDeposit, UnclaimedDeposit,
};
pub use namespace::WalletNamespace;
pub use reactor::{WalletReactor, ReactorEventAdapter};
pub use sdk::{SparkSdkWrapper, PaymentInfo, ReceiveInfo};

#[cfg(feature = "crypto")]
pub use persistence::WalletPersistence;

// Re-export common types
pub use crate::wallet_trait::{SignedMessage, TransactionDetails, WalletBalance};

#[derive(Error, Debug)]
pub enum WalletError {
    #[error("Not connected")]
    NotConnected,
    #[error("Already connected")]
    AlreadyConnected,
    #[error("Invalid mnemonic: {0}")]
    InvalidMnemonic(String),
    #[error("SDK error: {0}")]
    Sdk(String),
    #[error("Not implemented yet")]
    NotImplemented,
    #[error("IO error: {0}")]
    Io(String),
    #[error("Invalid data: {0}")]
    InvalidData(String),
    #[error("Path not found: {0}")]
    NotFound(String),
}

impl From<nine_s::Error> for WalletError {
    fn from(e: nine_s::Error) -> Self {
        match e {
            nine_s::Error::NotFound(p) => WalletError::NotFound(p),
            nine_s::Error::InvalidData(d) => WalletError::InvalidData(d),
            nine_s::Error::Unavailable(u) => WalletError::Sdk(u),
            _ => WalletError::Sdk(e.to_string()),
        }
    }
}

impl From<WalletError> for nine_s::Error {
    fn from(e: WalletError) -> Self {
        match e {
            WalletError::NotConnected => nine_s::Error::Unavailable("Not connected".into()),
            WalletError::NotImplemented => nine_s::Error::Unavailable("Not implemented".into()),
            WalletError::NotFound(p) => nine_s::Error::NotFound(p),
            WalletError::InvalidData(d) => nine_s::Error::InvalidData(d),
            _ => nine_s::Error::Internal(e.to_string()),
        }
    }
}

/// Network selection for Spark
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SparkNetwork {
    /// Bitcoin mainnet
    Mainnet,
    /// Bitcoin testnet
    #[default]
    Testnet,
    /// Bitcoin regtest (local)
    Regtest,
}

impl std::fmt::Display for SparkNetwork {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SparkNetwork::Mainnet => write!(f, "mainnet"),
            SparkNetwork::Testnet => write!(f, "testnet"),
            SparkNetwork::Regtest => write!(f, "regtest"),
        }
    }
}

/// Spark Wallet Manager
///
/// Main interface to Breez SDK Spark.
///
/// ## Design: Wallet IS a Namespace
///
/// WalletManager implements `Namespace` directly. All operations
/// are expressible through the 5 frozen ops:
///
/// - `read("/balance")` → Get balance
/// - `read("/address")` → Get receive address
/// - `write("/send", {to, amount})` → Send payment
/// - `write("/invoice", {amount})` → Create invoice
/// - `watch("/tx/**")` → Watch for payments
///
/// Convenience methods (`balance()`, `send()`) are thin wrappers
/// that delegate to `read()`/`write()`.
pub struct WalletManager {
    /// Async runtime for SDK operations
    runtime: Arc<Runtime>,
    /// SDK wrapper for Spark operations
    sdk: Arc<sdk::SparkSdkWrapper>,
    network: SparkNetwork,
    #[allow(dead_code)]
    api_key: Option<String>,
    working_dir: Option<PathBuf>,
    /// Reactive event core - handles SDK events → Scroll streams
    reactor: Arc<reactor::WalletReactor>,
    /// Persistence layer - bridges reactor with encrypted store
    #[cfg(feature = "crypto")]
    persistence: Option<Arc<persistence::WalletPersistence>>,
}

impl WalletManager {
    /// Create new wallet manager
    pub fn new(network: SparkNetwork, api_key: Option<String>) -> Self {
        let runtime = Arc::new(
            Runtime::new().expect("Failed to create tokio runtime")
        );
        let reactor = Arc::new(reactor::WalletReactor::new());
        let sdk = Arc::new(sdk::SparkSdkWrapper::new(
            reactor.clone(),
            runtime.handle().clone(),
        ));

        Self {
            runtime,
            sdk,
            network,
            api_key,
            working_dir: None,
            reactor,
            #[cfg(feature = "crypto")]
            persistence: None,
        }
    }

    /// Get reactor for event handling
    ///
    /// Use this to:
    /// - Register the ReactorEventAdapter with the SDK
    /// - Manually emit events for testing
    /// - Access cached state
    pub fn reactor(&self) -> Arc<reactor::WalletReactor> {
        self.reactor.clone()
    }

    /// Get persistence layer (if initialized)
    ///
    /// Persistence is initialized when `with_store()` is called.
    #[cfg(feature = "crypto")]
    pub fn persistence(&self) -> Option<Arc<persistence::WalletPersistence>> {
        self.persistence.clone()
    }

    /// Set working directory for wallet data
    pub fn with_working_dir(mut self, dir: PathBuf) -> Self {
        self.working_dir = Some(dir);
        self
    }

    /// Initialize persistence with an encrypted Store
    ///
    /// This bridges the reactive event core (Reactor) with encrypted
    /// cold storage (Store). Call this before `connect()` to enable
    /// state persistence across restarts.
    ///
    /// ## Architecture
    ///
    /// ```text
    /// SDK Events → Reactor (hot cache) → Store (cold storage)
    ///                  ↑                      |
    ///                  └── load_into_cache() ←┘
    /// ```
    #[cfg(feature = "crypto")]
    pub fn with_store(mut self, store: crate::nine_s::Store) -> Self {
        let persistence = persistence::WalletPersistence::new(store, self.reactor.clone());
        self.persistence = Some(Arc::new(persistence));
        self
    }

    /// Get current network
    pub fn network(&self) -> SparkNetwork {
        self.network
    }

    // =========================================================================
    // Convenience Methods (thin wrappers over Namespace)
    // =========================================================================

    /// Connect with mnemonic
    ///
    /// This is a lifecycle operation, not expressible as read/write.
    pub fn connect(&self, mnemonic: &str, passphrase: Option<&str>) -> Result<(), WalletError> {
        let working_dir = self.working_dir
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "/tmp/beewallet-spark".to_string());

        self.runtime.block_on(async {
            self.sdk.connect(
                mnemonic,
                passphrase,
                self.network,
                &working_dir,
            ).await
        }).map_err(|e| WalletError::Sdk(e.to_string()))
    }

    /// Initialize from mnemonic (same as connect for Spark)
    pub fn init_from_mnemonic(
        &self,
        mnemonic: &str,
        passphrase: Option<&str>,
    ) -> Result<(), WalletError> {
        self.connect(mnemonic, passphrase)
    }

    /// Load existing or initialize new wallet
    pub fn load_or_init(
        &self,
        mnemonic: &str,
        passphrase: Option<&str>,
    ) -> Result<(), WalletError> {
        self.connect(mnemonic, passphrase)
    }

    /// Disconnect
    pub fn disconnect(&self) -> Result<(), WalletError> {
        self.runtime.block_on(async {
            self.sdk.disconnect().await
        }).map_err(|e| WalletError::Sdk(e.to_string()))
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.runtime.block_on(async {
            self.sdk.is_connected().await
        })
    }

    /// Get balance (delegates to read("/balance"))
    pub fn balance(&self) -> Result<WalletBalance, WalletError> {
        let scroll = self.read("/balance")
            .map_err(WalletError::from)?
            .ok_or(WalletError::NotConnected)?;

        Ok(WalletBalance {
            confirmed: scroll.data["confirmed"].as_u64().unwrap_or(0),
            trusted_pending: scroll.data["trusted_pending"].as_u64().unwrap_or(0),
            untrusted_pending: scroll.data["untrusted_pending"].as_u64().unwrap_or(0),
            immature: scroll.data["immature"].as_u64().unwrap_or(0),
        })
    }

    /// Sync wallet (delegates to write("/sync", {}))
    pub fn sync(&self) -> Result<(), WalletError> {
        self.write("/sync", json!({})).map_err(WalletError::from)?;
        Ok(())
    }

    /// Get receive address (delegates to read("/address"))
    pub fn new_address(&self) -> Result<String, WalletError> {
        let scroll = self.read("/address")
            .map_err(WalletError::from)?
            .ok_or(WalletError::NotConnected)?;

        scroll.data["address"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or(WalletError::InvalidData("no address in response".into()))
    }

    /// Send to destination (delegates to write("/send", {to, amount, feeRate}))
    pub fn send(
        &self,
        destination: &str,
        amount_sats: u64,
        fee_rate: Option<f64>,
    ) -> Result<String, WalletError> {
        let mut data = json!({
            "to": destination,
            "amount": amount_sats,
        });
        if let Some(rate) = fee_rate {
            data["feeRate"] = json!(rate);
        }

        let scroll = self.write("/send", data).map_err(WalletError::from)?;

        scroll.data["txid"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or(WalletError::InvalidData("no txid in response".into()))
    }

    /// Estimate fee (delegates to write("/fee-estimate", {to, amount}))
    pub fn estimate_fee(&self, destination: &str, amount_sats: u64) -> Result<u64, WalletError> {
        let scroll = self.write("/fee-estimate", json!({
            "to": destination,
            "amount": amount_sats,
        })).map_err(WalletError::from)?;

        scroll.data["fee"]
            .as_u64()
            .ok_or(WalletError::InvalidData("no fee in response".into()))
    }

    /// Create Lightning invoice (delegates to write("/invoice", {amount, description}))
    pub fn create_invoice(
        &self,
        amount_sats: u64,
        description: Option<&str>,
    ) -> Result<String, WalletError> {
        let mut data = json!({"amount": amount_sats});
        if let Some(desc) = description {
            data["description"] = json!(desc);
        }

        let scroll = self.write("/invoice", data).map_err(WalletError::from)?;

        scroll.data["invoice"]
            .as_str()
            .or_else(|| scroll.data["destination"].as_str())
            .map(|s| s.to_string())
            .ok_or(WalletError::InvalidData("no invoice in response".into()))
    }

    /// List transactions (delegates to read("/transactions"))
    pub fn transactions(&self, limit: usize) -> Result<Vec<TransactionDetails>, WalletError> {
        let scroll = self.read(&format!("/transactions?limit={}", limit))
            .map_err(WalletError::from)?
            .ok_or(WalletError::NotConnected)?;

        let txs = scroll.data["transactions"]
            .as_array()
            .map(|arr| {
                arr.iter().filter_map(|v| {
                    Some(TransactionDetails {
                        txid: v["txid"].as_str()?.to_string(),
                        received: v["received"].as_u64().unwrap_or(0),
                        sent: v["sent"].as_u64().unwrap_or(0),
                        fee: v["fee"].as_u64(),
                        confirmation_time: v["confirmation_time"].as_u64(),
                        is_confirmed: v["is_confirmed"].as_bool().unwrap_or(false),
                        vsize: v["vsize"].as_u64(),
                        timestamp: v["timestamp"].as_u64(),
                    })
                }).collect()
            })
            .unwrap_or_default();

        Ok(txs)
    }

    /// Get node pubkey (delegates to read("/pubkey"))
    pub fn pubkey(&self) -> Result<String, WalletError> {
        let scroll = self.read("/pubkey")
            .map_err(WalletError::from)?
            .ok_or(WalletError::NotConnected)?;

        scroll.data["pubkey"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or(WalletError::InvalidData("no pubkey in response".into()))
    }

    /// Sign message (delegates to write("/sign", {message}))
    pub fn sign_message(&self, message: &str) -> Result<SignedMessage, WalletError> {
        let scroll = self.write("/sign", json!({"message": message}))
            .map_err(WalletError::from)?;

        Ok(SignedMessage {
            address: scroll.data["address"].as_str().unwrap_or("").to_string(),
            message: message.to_string(),
            signature: scroll.data["signature"].as_str().unwrap_or("").to_string(),
        })
    }

    /// Verify message signature
    pub fn verify_message(
        &self,
        message: &str,
        signature: &str,
        pubkey: &str,
    ) -> Result<bool, WalletError> {
        let scroll = self.write("/verify", json!({
            "message": message,
            "signature": signature,
            "pubkey": pubkey,
        })).map_err(WalletError::from)?;

        Ok(scroll.data["valid"].as_bool().unwrap_or(false))
    }

    /// Get backend name
    pub fn backend_name(&self) -> &'static str {
        "spark"
    }

    /// Get backend version
    pub fn backend_version(&self) -> &'static str {
        "0.1.0-experimental"
    }

    /// Convert network to string for scroll responses
    fn network_str(&self) -> &'static str {
        match self.network {
            SparkNetwork::Mainnet => "bitcoin",
            SparkNetwork::Testnet => "testnet",
            SparkNetwork::Regtest => "regtest",
        }
    }
}

// =============================================================================
// Namespace Implementation: The 9S Way
// =============================================================================

impl Namespace for WalletManager {
    /// Read wallet data by path
    ///
    /// Paths:
    /// - `/status` - Connection status (always works)
    /// - `/balance` - Wallet balance
    /// - `/address` - Receive address
    /// - `/pubkey` - Node public key
    /// - `/network` - Current network
    /// - `/transactions` - Recent transactions
    /// - `/tx/{txid}` - Single transaction
    fn read(&self, path: &str) -> nine_s::Result<Option<Scroll>> {
        // Status check doesn't require connection
        if path == "/status" || path.is_empty() || path == "/" {
            let connected = self.is_connected();
            return Ok(Some(Scroll::typed(
                "/wallet/status",
                json!({
                    "connected": connected,
                    "network": self.network_str(),
                    "backend": "spark",
                    "version": self.backend_version(),
                }),
                "wallet/status@v1",
            )));
        }

        // Everything else requires connection
        if !self.is_connected() {
            return Err(nine_s::Error::Unavailable("Wallet not connected".into()));
        }

        match path {
            "/balance" => {
                let balance = self.runtime.block_on(async {
                    self.sdk.get_balance().await
                }).map_err(|e| nine_s::Error::Internal(e.to_string()))?;

                // Cache in reactor
                self.reactor.emit_balance(balance, 0);

                Ok(Some(Scroll::typed(
                    "/wallet/balance",
                    json!({
                        "confirmed": balance,
                        "trusted_pending": 0,
                        "untrusted_pending": 0,
                        "immature": 0,
                    }),
                    "wallet/balance@v1",
                )))
            }
            "/address" => {
                let address = self.runtime.block_on(async {
                    self.sdk.get_spark_address().await
                }).map_err(|e| nine_s::Error::Internal(e.to_string()))?;

                Ok(Some(Scroll::typed(
                    "/wallet/address",
                    json!({"address": address}),
                    "wallet/address@v1",
                )))
            }
            "/pubkey" => {
                // Spark address includes the identity pubkey
                let address = self.runtime.block_on(async {
                    self.sdk.get_spark_address().await
                }).map_err(|e| nine_s::Error::Internal(e.to_string()))?;

                // Extract pubkey from spark address (it's part of the address format)
                Ok(Some(Scroll::typed(
                    "/wallet/pubkey",
                    json!({"pubkey": address}),
                    "wallet/pubkey@v1",
                )))
            }
            "/network" => {
                Ok(Some(Scroll::typed(
                    "/wallet/network",
                    json!({"network": self.network_str()}),
                    "wallet/network@v1",
                )))
            }
            p if p == "/transactions" || p.starts_with("/transactions?") => {
                // Parse limit from query string
                let limit = p.split("limit=")
                    .nth(1)
                    .and_then(|s| s.split('&').next())
                    .and_then(|s| s.parse::<u32>().ok());

                let payments = self.runtime.block_on(async {
                    self.sdk.list_payments(limit).await
                }).map_err(|e| nine_s::Error::Internal(e.to_string()))?;

                let txs: Vec<Value> = payments.iter().map(|p| json!({
                    "txid": p.id,
                    "received": if matches!(p.payment_type, PaymentType::Receive) { p.amount_sat } else { 0 },
                    "sent": if matches!(p.payment_type, PaymentType::Send) { p.amount_sat } else { 0 },
                    "fee": p.fee_sat,
                    "timestamp": p.timestamp,
                    "is_confirmed": matches!(p.status, PaymentState::Complete),
                })).collect();

                Ok(Some(Scroll::typed(
                    "/wallet/transactions",
                    json!({"transactions": txs}),
                    "wallet/transactions@v1",
                )))
            }
            p if p.starts_with("/tx/") => {
                let txid = &p[4..];
                // Try to find in cached payments
                let payments = self.runtime.block_on(async {
                    self.sdk.list_payments(Some(100)).await
                }).map_err(|e| nine_s::Error::Internal(e.to_string()))?;

                if let Some(payment) = payments.iter().find(|p| p.id == txid) {
                    Ok(Some(Scroll::typed(
                        &format!("/wallet/tx/{}", txid),
                        json!({
                            "txid": payment.id,
                            "type": if matches!(payment.payment_type, PaymentType::Receive) { "receive" } else { "send" },
                            "amount_sat": payment.amount_sat,
                            "fee_sat": payment.fee_sat,
                            "timestamp": payment.timestamp,
                            "state": format!("{:?}", payment.status),
                        }),
                        "wallet/payment@v1",
                    )))
                } else {
                    Err(nine_s::Error::NotFound(format!("Transaction {} not found", txid)))
                }
            }
            _ => Ok(None),
        }
    }

    /// Write wallet operations
    ///
    /// Paths:
    /// - `/send` - Send payment (requires: to, amount; optional: feeRate)
    /// - `/invoice` - Create invoice (requires: amount; optional: description)
    /// - `/sync` - Trigger wallet sync
    /// - `/sign` - Sign message (requires: message)
    /// - `/verify` - Verify signature (requires: message, signature, pubkey)
    /// - `/fee-estimate` - Estimate fee (requires: to, amount)
    fn write(&self, path: &str, data: Value) -> nine_s::Result<Scroll> {
        if !self.is_connected() {
            return Err(nine_s::Error::Unavailable("Wallet not connected".into()));
        }

        match path {
            "/send" => {
                let destination = data["to"].as_str()
                    .or_else(|| data["destination"].as_str())
                    .ok_or_else(|| nine_s::Error::InvalidData("Missing 'to' field".into()))?;
                let amount = data["amount"].as_u64()
                    .or_else(|| data["amount_sat"].as_u64());

                let payment = self.runtime.block_on(async {
                    self.sdk.send(destination, amount).await
                }).map_err(|e| nine_s::Error::Internal(e.to_string()))?;

                Ok(Scroll::typed(
                    &format!("/wallet/tx/{}", payment.id),
                    json!({
                        "txid": payment.id,
                        "type": "send",
                        "amount_sat": payment.amount_sat,
                        "fee_sat": payment.fee_sat,
                        "timestamp": payment.timestamp,
                        "state": format!("{:?}", payment.status),
                    }),
                    "wallet/payment@v1",
                ))
            }
            "/invoice" | "/receive" => {
                let amount = data["amount"].as_u64()
                    .or_else(|| data["amountSat"].as_u64())
                    .or_else(|| data["amount_sat"].as_u64())
                    .ok_or_else(|| nine_s::Error::InvalidData("Missing 'amount' field".into()))?;
                let description = data["description"].as_str().map(|s| s.to_string());

                let receive_info = self.runtime.block_on(async {
                    self.sdk.create_invoice(amount, description).await
                }).map_err(|e| nine_s::Error::Internal(e.to_string()))?;

                Ok(Scroll::typed(
                    "/wallet/invoice",
                    json!({
                        "invoice": receive_info.destination,
                        "destination": receive_info.destination,
                        "fee_sat": receive_info.fee_sat,
                    }),
                    "wallet/invoice@v1",
                ))
            }
            "/sync" => {
                self.runtime.block_on(async {
                    self.sdk.sync().await
                }).map_err(|e| nine_s::Error::Internal(e.to_string()))?;

                Ok(Scroll::typed(
                    "/wallet/sync",
                    json!({"synced": true}),
                    "wallet/sync@v1",
                ))
            }
            "/sign" => {
                let _message = data["message"].as_str()
                    .ok_or_else(|| nine_s::Error::InvalidData("Missing 'message' field".into()))?;

                // Signing not directly exposed in SDK wrapper yet
                Err(nine_s::Error::Unavailable("Signing not implemented".into()))
            }
            "/verify" => {
                let _message = data["message"].as_str()
                    .ok_or_else(|| nine_s::Error::InvalidData("Missing 'message' field".into()))?;
                let _signature = data["signature"].as_str()
                    .ok_or_else(|| nine_s::Error::InvalidData("Missing 'signature' field".into()))?;
                let _pubkey = data["pubkey"].as_str()
                    .ok_or_else(|| nine_s::Error::InvalidData("Missing 'pubkey' field".into()))?;

                // Verification not directly exposed in SDK wrapper yet
                Err(nine_s::Error::Unavailable("Verification not implemented".into()))
            }
            "/fee-estimate" => {
                // Spark has no fees for Spark-to-Spark transfers
                // Fee estimation would be needed for Lightning/on-chain
                Ok(Scroll::typed(
                    "/wallet/fee-estimate",
                    json!({"fee": 0}),
                    "wallet/fee-estimate@v1",
                ))
            }
            _ => Err(nine_s::Error::NotFound(path.into())),
        }
    }

    /// List available paths
    fn list(&self, prefix: &str) -> nine_s::Result<Vec<String>> {
        match prefix {
            "/" | "" => Ok(vec![
                "/status".to_string(),
                "/balance".to_string(),
                "/address".to_string(),
                "/pubkey".to_string(),
                "/network".to_string(),
                "/transactions".to_string(),
            ]),
            "/tx" => {
                // TODO: list transaction IDs
                Ok(vec![])
            }
            _ => Ok(vec![]),
        }
    }

    /// Watch for changes (payment events)
    ///
    /// The reactor handles pattern matching and dispatches Scrolls
    /// to watchers when SDK events occur.
    ///
    /// ## Patterns
    ///
    /// - `/wallet/balance` - Balance updates
    /// - `/wallet/tx/*` - Specific transaction updates
    /// - `/wallet/tx/**` - All transaction events
    /// - `/wallet/**` - All wallet events
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let mut rx = wallet.watch("/wallet/tx/**")?;
    ///
    /// // In UI thread or async task:
    /// while let Some(scroll) = rx.recv() {
    ///     match scroll.type_.as_str() {
    ///         "wallet/payment@v1" => update_payment(scroll),
    ///         "wallet/balance@v1" => update_balance(scroll),
    ///         _ => {}
    ///     }
    /// }
    /// ```
    fn watch(&self, pattern: &str) -> nine_s::Result<nine_s::Receiver<Scroll>> {
        // Watch works even when disconnected (will receive events when connected)
        // This allows UI to set up watchers before connection is established
        Ok(self.reactor.watch(pattern))
    }

    /// Close wallet connection
    fn close(&self) -> nine_s::Result<()> {
        let _ = self.disconnect();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_manager_creation() {
        let manager = WalletManager::new(SparkNetwork::Testnet, None);
        assert!(!manager.is_connected());
        assert_eq!(manager.backend_name(), "spark");
        assert_eq!(manager.network(), SparkNetwork::Testnet);
    }

    #[test]
    fn test_network_display() {
        assert_eq!(SparkNetwork::Mainnet.to_string(), "mainnet");
        assert_eq!(SparkNetwork::Testnet.to_string(), "testnet");
        assert_eq!(SparkNetwork::Regtest.to_string(), "regtest");
    }

    #[test]
    fn test_namespace_status_always_works() {
        let manager = WalletManager::new(SparkNetwork::Regtest, None);

        // Status should work even when not connected
        let scroll = manager.read("/status").unwrap().unwrap();
        assert_eq!(scroll.data["connected"], false);
        assert_eq!(scroll.data["network"], "regtest");
        assert_eq!(scroll.data["backend"], "spark");
    }

    #[test]
    fn test_namespace_list() {
        let manager = WalletManager::new(SparkNetwork::Testnet, None);

        let paths = manager.list("/").unwrap();
        assert!(paths.contains(&"/status".to_string()));
        assert!(paths.contains(&"/balance".to_string()));
        assert!(paths.contains(&"/address".to_string()));
    }

    #[test]
    fn test_namespace_requires_connection() {
        let manager = WalletManager::new(SparkNetwork::Testnet, None);

        // Balance should fail when not connected
        let result = manager.read("/balance");
        assert!(result.is_err());
    }

    #[test]
    fn test_error_conversion() {
        let wallet_err = WalletError::NotConnected;
        let nine_s_err: nine_s::Error = wallet_err.into();
        assert!(matches!(nine_s_err, nine_s::Error::Unavailable(_)));

        let nine_s_err = nine_s::Error::NotFound("/test".into());
        let wallet_err: WalletError = nine_s_err.into();
        assert!(matches!(wallet_err, WalletError::NotFound(_)));
    }

    #[test]
    fn test_watch_reactive() {
        let manager = WalletManager::new(SparkNetwork::Regtest, None);

        // Set up watcher before connection
        let mut rx = manager.watch("/wallet/balance").unwrap();

        // Emit balance via reactor
        manager.reactor().emit_balance(50000, 1000);

        // Should receive the balance scroll
        let scroll = rx.try_recv().expect("Should receive balance scroll");
        assert_eq!(scroll.key, "/wallet/balance");
        assert_eq!(scroll.data["confirmed"], 50000);
        assert_eq!(scroll.data["trusted_pending"], 1000);
    }

    #[test]
    fn test_watch_payment_events() {
        let manager = WalletManager::new(SparkNetwork::Regtest, None);

        // Watch for all tx events
        let mut rx = manager.watch("/wallet/tx/**").unwrap();

        // Simulate a payment event from SDK
        let payment = Payment {
            id: "pay123".to_string(),
            payment_type: PaymentType::Receive,
            state: PaymentState::Complete,
            amount_sat: 21000,
            fee_sat: Some(100),
            timestamp: Some(1234567890),
            description: Some("Test sats".to_string()),
            details: PaymentDetails::default(),
        };

        manager.reactor().ingest(SdkEvent::PaymentSucceeded { payment });

        // Should receive payment scroll
        let scroll = rx.try_recv().expect("Should receive payment scroll");
        assert!(scroll.key.starts_with("/wallet/tx/"));
        assert_eq!(scroll.data["amount_sat"], 21000);
        assert_eq!(scroll.data["type"], "receive");
    }

    #[cfg(feature = "crypto")]
    mod crypto_tests {
        use super::*;
        use tempfile::tempdir;
        use crate::nine_s::Store;

        #[test]
        fn test_with_store_initializes_persistence() {
            let dir = tempdir().unwrap();
            let key = Store::test_key();
            let store = Store::at(dir.path(), &key).unwrap();

            let manager = WalletManager::new(SparkNetwork::Regtest, None)
                .with_store(store);

            // Persistence should be initialized
            assert!(manager.persistence().is_some());
        }

        #[test]
        fn test_wallet_manager_without_store_no_persistence() {
            let manager = WalletManager::new(SparkNetwork::Regtest, None);

            // Persistence should not be initialized
            assert!(manager.persistence().is_none());
        }

        #[test]
        fn test_persistence_through_wallet_manager() {
            let dir = tempdir().unwrap();
            let key = Store::test_key();
            let store = Store::at(dir.path(), &key).unwrap();

            let manager = WalletManager::new(SparkNetwork::Regtest, None)
                .with_store(store);

            let persistence = manager.persistence().expect("Persistence should be initialized");

            // Persist balance
            persistence.persist_balance(100000, 5000).unwrap();

            // Should be in reactor cache
            let cached = manager.reactor().get_cached("/wallet/balance").unwrap();
            assert_eq!(cached.data["confirmed"], 100000);
            assert_eq!(cached.data["trusted_pending"], 5000);
        }
    }
}
