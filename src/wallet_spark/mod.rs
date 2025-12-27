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

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use serde_json::{json, Value};

use crate::nine_s::{self, Namespace, Scroll};

// Re-export submodule types
pub use config::WalletConfig;
pub use events::{
    EventListener, SdkEvent, Payment, PaymentType, PaymentState, PaymentDetails,
    ClaimedDeposit, UnclaimedDeposit,
};
pub use namespace::WalletNamespace;

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
    // SDK instance would go here
    // sdk: Arc<Mutex<Option<Arc<SparkSdk>>>>,
    // runtime: Arc<Runtime>,
    network: SparkNetwork,
    api_key: Option<String>,
    working_dir: Option<PathBuf>,
    connected: Arc<Mutex<bool>>,
    // Event broadcast channel for watch()
    #[cfg(feature = "std-channel")]
    event_sender: Arc<Mutex<Option<std::sync::mpsc::Sender<Scroll>>>>,
}

impl WalletManager {
    /// Create new wallet manager
    pub fn new(network: SparkNetwork, api_key: Option<String>) -> Self {
        Self {
            network,
            api_key,
            working_dir: None,
            connected: Arc::new(Mutex::new(false)),
            #[cfg(feature = "std-channel")]
            event_sender: Arc::new(Mutex::new(None)),
        }
    }

    /// Set working directory for wallet data
    pub fn with_working_dir(mut self, dir: PathBuf) -> Self {
        self.working_dir = Some(dir);
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
    pub fn connect(&self, _mnemonic: &str, _passphrase: Option<&str>) -> Result<(), WalletError> {
        // TODO: Implement with Breez SDK Spark
        // let config = SparkSdkConfig::new(self.network, self.api_key.clone(), self.working_dir.clone());
        // let sdk = self.runtime.block_on(async { SparkSdk::connect(config, mnemonic).await })?;
        // *self.sdk.lock().unwrap() = Some(Arc::new(sdk));
        *self.connected.lock().unwrap() = true;
        Err(WalletError::NotImplemented)
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
        *self.connected.lock().unwrap() = false;
        Ok(())
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        *self.connected.lock().unwrap()
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
                // TODO: sdk.get_info().balance
                Err(nine_s::Error::Unavailable("Not implemented".into()))
            }
            "/address" => {
                // TODO: sdk.receive_onchain().address
                Err(nine_s::Error::Unavailable("Not implemented".into()))
            }
            "/pubkey" => {
                // TODO: sdk.get_info().pubkey
                Err(nine_s::Error::Unavailable("Not implemented".into()))
            }
            "/network" => {
                Ok(Some(Scroll::typed(
                    "/wallet/network",
                    json!({"network": self.network_str()}),
                    "wallet/network@v1",
                )))
            }
            p if p == "/transactions" || p.starts_with("/transactions?") => {
                // TODO: sdk.list_payments()
                Err(nine_s::Error::Unavailable("Not implemented".into()))
            }
            p if p.starts_with("/tx/") => {
                // TODO: lookup single transaction
                let _txid = &p[4..];
                Err(nine_s::Error::Unavailable("Not implemented".into()))
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
                let _destination = data["to"].as_str()
                    .or_else(|| data["destination"].as_str())
                    .ok_or_else(|| nine_s::Error::InvalidData("Missing 'to' field".into()))?;
                let _amount = data["amount"].as_u64()
                    .or_else(|| data["amount_sat"].as_u64())
                    .ok_or_else(|| nine_s::Error::InvalidData("Missing 'amount' field".into()))?;
                let _fee_rate = data["feeRate"].as_f64()
                    .or_else(|| data["fee_rate"].as_f64());

                // TODO: sdk.send_payment()
                Err(nine_s::Error::Unavailable("Not implemented".into()))
            }
            "/invoice" | "/receive" => {
                let _amount = data["amount"].as_u64()
                    .or_else(|| data["amountSat"].as_u64())
                    .or_else(|| data["amount_sat"].as_u64())
                    .ok_or_else(|| nine_s::Error::InvalidData("Missing 'amount' field".into()))?;
                let _description = data["description"].as_str();

                // TODO: sdk.receive_payment()
                Err(nine_s::Error::Unavailable("Not implemented".into()))
            }
            "/sync" => {
                // TODO: sdk.sync()
                Err(nine_s::Error::Unavailable("Not implemented".into()))
            }
            "/sign" => {
                let _message = data["message"].as_str()
                    .ok_or_else(|| nine_s::Error::InvalidData("Missing 'message' field".into()))?;

                // TODO: sdk.sign_message()
                Err(nine_s::Error::Unavailable("Not implemented".into()))
            }
            "/verify" => {
                let _message = data["message"].as_str()
                    .ok_or_else(|| nine_s::Error::InvalidData("Missing 'message' field".into()))?;
                let _signature = data["signature"].as_str()
                    .ok_or_else(|| nine_s::Error::InvalidData("Missing 'signature' field".into()))?;
                let _pubkey = data["pubkey"].as_str()
                    .ok_or_else(|| nine_s::Error::InvalidData("Missing 'pubkey' field".into()))?;

                // TODO: sdk.check_message()
                Err(nine_s::Error::Unavailable("Not implemented".into()))
            }
            "/fee-estimate" => {
                let _destination = data["to"].as_str()
                    .or_else(|| data["destination"].as_str())
                    .ok_or_else(|| nine_s::Error::InvalidData("Missing 'to' field".into()))?;
                let _amount = data["amount"].as_u64().unwrap_or(0);

                // TODO: sdk.prepare_send_payment()
                Err(nine_s::Error::Unavailable("Not implemented".into()))
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
    fn watch(&self, pattern: &str) -> nine_s::Result<nine_s::Receiver<Scroll>> {
        if !self.is_connected() {
            return Err(nine_s::Error::Unavailable("Wallet not connected".into()));
        }

        // TODO: Wire up SDK event listener to channel
        // For now, return error
        Err(nine_s::Error::Unavailable(format!(
            "Watch not yet implemented for pattern: {}",
            pattern
        )))
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
}
