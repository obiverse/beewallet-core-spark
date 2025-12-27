//! Wallet implementation using Breez SDK Spark
//!
//! This module provides a WalletManager that wraps Breez SDK Spark,
//! offering the same API as beewallet-core-breez for interoperability.
//!
//! ## Status: SCAFFOLD
//!
//! This is a placeholder implementation. Methods return NotImplemented.
//! Real implementation requires Breez SDK Spark integration.

pub mod signing;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use thiserror::Error;

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
/// API mirrors beewallet-core-breez::WalletManager.
pub struct WalletManager {
    // SDK instance would go here
    // sdk: Arc<Mutex<Option<Arc<SparkSdk>>>>,
    // runtime: Arc<Runtime>,
    network: SparkNetwork,
    api_key: Option<String>,
    working_dir: Option<PathBuf>,
    connected: Arc<Mutex<bool>>,
}

impl WalletManager {
    /// Create new wallet manager
    pub fn new(network: SparkNetwork, api_key: Option<String>) -> Self {
        Self {
            network,
            api_key,
            working_dir: None,
            connected: Arc::new(Mutex::new(false)),
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

    /// Connect with mnemonic
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

    /// Get balance
    pub fn balance(&self) -> Result<WalletBalance, WalletError> {
        if !self.is_connected() {
            return Err(WalletError::NotConnected);
        }
        // TODO: sdk.get_info().balance
        Err(WalletError::NotImplemented)
    }

    /// Sync wallet
    pub fn sync(&self) -> Result<(), WalletError> {
        if !self.is_connected() {
            return Err(WalletError::NotConnected);
        }
        // TODO: sdk.sync()
        Err(WalletError::NotImplemented)
    }

    /// Get receive address
    pub fn new_address(&self) -> Result<String, WalletError> {
        if !self.is_connected() {
            return Err(WalletError::NotConnected);
        }
        // TODO: sdk.receive_onchain().address
        Err(WalletError::NotImplemented)
    }

    /// Send to destination (BTC address or Lightning invoice)
    pub fn send(
        &self,
        _destination: &str,
        _amount_sats: u64,
        _fee_rate: Option<f64>,
    ) -> Result<String, WalletError> {
        if !self.is_connected() {
            return Err(WalletError::NotConnected);
        }
        // TODO: sdk.send_payment() or sdk.send_onchain()
        Err(WalletError::NotImplemented)
    }

    /// Estimate fee for send
    pub fn estimate_fee(&self, _destination: &str, _amount_sats: u64) -> Result<u64, WalletError> {
        if !self.is_connected() {
            return Err(WalletError::NotConnected);
        }
        // TODO: sdk.prepare_send_payment()
        Err(WalletError::NotImplemented)
    }

    /// Create Lightning invoice
    pub fn create_invoice(
        &self,
        _amount_sats: u64,
        _description: Option<&str>,
    ) -> Result<String, WalletError> {
        if !self.is_connected() {
            return Err(WalletError::NotConnected);
        }
        // TODO: sdk.receive_payment()
        Err(WalletError::NotImplemented)
    }

    /// List transactions
    pub fn transactions(&self, _limit: usize) -> Result<Vec<TransactionDetails>, WalletError> {
        if !self.is_connected() {
            return Err(WalletError::NotConnected);
        }
        // TODO: sdk.list_payments()
        Err(WalletError::NotImplemented)
    }

    /// Get node pubkey
    pub fn pubkey(&self) -> Result<String, WalletError> {
        if !self.is_connected() {
            return Err(WalletError::NotConnected);
        }
        // TODO: sdk.get_info().pubkey
        Err(WalletError::NotImplemented)
    }

    /// Sign message using node key
    pub fn sign_message(&self, _message: &str) -> Result<SignedMessage, WalletError> {
        if !self.is_connected() {
            return Err(WalletError::NotConnected);
        }
        // TODO: sdk.sign_message()
        Err(WalletError::NotImplemented)
    }

    /// Verify message signature
    pub fn verify_message(
        &self,
        _message: &str,
        _signature: &str,
        _pubkey: &str,
    ) -> Result<bool, WalletError> {
        if !self.is_connected() {
            return Err(WalletError::NotConnected);
        }
        // TODO: sdk.check_message()
        Err(WalletError::NotImplemented)
    }

    /// Get backend name
    pub fn backend_name(&self) -> &'static str {
        "spark"
    }

    /// Get backend version
    pub fn backend_version(&self) -> &'static str {
        "0.1.0-experimental"
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
}
