//! Unified Wallet Backend Trait
//!
//! This module defines the common interface for all wallet backends.
//! Both Liquid and Spark implementations conform to this trait.
//!
//! # Dialectics: WalletBackend vs Namespace
//!
//! **Thesis**: Named methods (`balance()`, `send()`) are ergonomic.
//! **Antithesis**: 9S Protocol uses 5 frozen ops (read/write/list/watch/close).
//! **Synthesis**: WalletManager implements Namespace directly; WalletBackend
//! is kept for backwards compatibility but convenience methods delegate to
//! Namespace operations internally.
//!
//! ## Preferred Pattern (9S Way)
//!
//! ```rust,ignore
//! use beewallet_core_spark::{WalletManager, Namespace};
//!
//! let wallet = WalletManager::new(SparkNetwork::Regtest, None);
//! wallet.connect(mnemonic, None)?;
//!
//! // Use namespace operations directly
//! let balance_scroll = wallet.read("/balance")?;
//! let send_scroll = wallet.write("/send", json!({"to": addr, "amount": 1000}))?;
//!
//! // Or use convenience methods (which delegate to read/write)
//! let balance = wallet.balance()?;
//! let txid = wallet.send(addr, 1000, None)?;
//! ```
//!
//! # Backend Selection
//!
//! Due to SQLite conflicts, backends are in separate crates:
//! - `beewallet-core-breez` - Breez SDK Liquid (production, L-BTC wrapping)
//! - `beewallet-core-spark` - Breez SDK Spark (experimental, native BTC)

use std::fmt;
use thiserror::Error;

/// Wallet errors (unified across backends)
#[derive(Error, Debug)]
pub enum WalletError {
    #[error("Invalid mnemonic: {0}")]
    InvalidMnemonic(String),
    #[error("SDK error: {0}")]
    Sdk(String),
    #[error("Wallet not initialized")]
    NotInitialized,
    #[error("Invalid address: {0}")]
    InvalidAddress(String),
    #[error("Insufficient funds")]
    InsufficientFunds,
    #[error("Transaction error: {0}")]
    Transaction(String),
    #[error("Invalid fee rate: {0}")]
    InvalidFeeRate(String),
    #[error("Not connected")]
    NotConnected,
    #[error("Internal error: {0}")]
    Internal(String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("LNURL error: {0}")]
    Lnurl(String),
}

/// Balance breakdown (unified across backends)
#[derive(Debug, Clone, Default)]
pub struct WalletBalance {
    /// Confirmed, spendable balance (in satoshis)
    pub confirmed: u64,
    /// Immature coinbase (always 0 for SDK wallets)
    pub immature: u64,
    /// Pending incoming (swaps/claims in progress)
    pub trusted_pending: u64,
    /// Untrusted pending (always 0 for SDK wallets)
    pub untrusted_pending: u64,
}

impl WalletBalance {
    /// Total balance (confirmed + pending)
    pub fn total(&self) -> u64 {
        self.confirmed + self.trusted_pending
    }

    /// Spendable balance (confirmed only)
    pub fn spendable(&self) -> u64 {
        self.confirmed
    }
}

/// Transaction details (unified across backends)
#[derive(Debug, Clone)]
pub struct TransactionDetails {
    pub txid: String,
    pub received: u64,
    pub sent: u64,
    pub fee: Option<u64>,
    pub confirmation_time: Option<u64>,
    pub is_confirmed: bool,
    pub vsize: Option<u64>,
    pub timestamp: Option<u64>,
}

/// Signed message result
#[derive(Debug, Clone)]
pub struct SignedMessage {
    pub address: String,
    pub message: String,
    pub signature: String,
}

impl fmt::Display for SignedMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "-----BEGIN BITCOIN SIGNED MESSAGE-----\n{}\n-----BEGIN SIGNATURE-----\n{}\n{}\n-----END BITCOIN SIGNED MESSAGE-----",
            self.message, self.address, self.signature
        )
    }
}

/// Unified wallet backend trait
///
/// Both Liquid and Spark implementations conform to this trait,
/// allowing code to be written against the trait without caring
/// which backend is in use.
///
/// # Deprecation Note
///
/// This trait is preserved for backwards compatibility with existing code.
/// The preferred pattern is to use `Namespace` directly:
///
/// ```rust,ignore
/// // Preferred: Use Namespace trait
/// wallet.read("/balance")?;
/// wallet.write("/send", json!({...}))?;
///
/// // Also available: Convenience methods
/// wallet.balance()?;
/// wallet.send(dest, amount, None)?;
/// ```
///
/// WalletManager implements both `Namespace` and `WalletBackend`.
/// New code should prefer `Namespace` operations.
pub trait WalletBackend: Send + Sync {
    // =========================================================================
    // Lifecycle
    // =========================================================================

    /// Connect to the wallet with mnemonic
    fn connect(&self, mnemonic: &str, passphrase: Option<&str>) -> Result<(), WalletError>;

    /// Disconnect from the wallet
    fn disconnect(&self) -> Result<(), WalletError>;

    /// Check if connected
    fn is_connected(&self) -> bool;

    // =========================================================================
    // Core Wallet Operations
    // =========================================================================

    /// Get wallet balance
    fn balance(&self) -> Result<WalletBalance, WalletError>;

    /// Sync wallet state
    fn sync(&self) -> Result<(), WalletError>;

    /// Get a new receive address
    fn new_address(&self) -> Result<String, WalletError>;

    /// Send payment to destination (BTC address or Lightning invoice)
    fn send(
        &self,
        destination: &str,
        amount_sats: u64,
        fee_rate: Option<f64>,
    ) -> Result<String, WalletError>;

    /// Estimate fee for a transaction
    fn estimate_fee(&self, destination: &str, amount_sats: u64) -> Result<u64, WalletError>;

    /// List recent transactions
    fn transactions(&self, limit: usize) -> Result<Vec<TransactionDetails>, WalletError>;

    // =========================================================================
    // Lightning
    // =========================================================================

    /// Create a Lightning invoice
    fn create_invoice(
        &self,
        amount_sats: u64,
        description: Option<&str>,
    ) -> Result<String, WalletError>;

    /// Get wallet pubkey (for signing identity)
    fn pubkey(&self) -> Result<String, WalletError>;

    // =========================================================================
    // Message Signing
    // =========================================================================

    /// Sign a message with the wallet's private key
    fn sign_message(&self, message: &str) -> Result<SignedMessage, WalletError>;

    /// Verify a message signature
    fn verify_message(
        &self,
        message: &str,
        signature: &str,
        pubkey: &str,
    ) -> Result<bool, WalletError>;

    // =========================================================================
    // Backend Info
    // =========================================================================

    /// Get backend name ("liquid" or "spark")
    fn backend_name(&self) -> &'static str;

    /// Get backend version
    fn backend_version(&self) -> &'static str;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_balance_default() {
        let balance = WalletBalance::default();
        assert_eq!(balance.total(), 0);
        assert_eq!(balance.spendable(), 0);
    }

    #[test]
    fn test_wallet_balance_calculation() {
        let balance = WalletBalance {
            confirmed: 1000,
            immature: 0,
            trusted_pending: 500,
            untrusted_pending: 0,
        };
        assert_eq!(balance.total(), 1500);
        assert_eq!(balance.spendable(), 1000);
    }

    #[test]
    fn test_signed_message_display() {
        let signed = SignedMessage {
            address: "bc1qtest".to_string(),
            message: "Hello".to_string(),
            signature: "sig123".to_string(),
        };
        let display = format!("{}", signed);
        assert!(display.contains("Hello"));
        assert!(display.contains("bc1qtest"));
        assert!(display.contains("sig123"));
    }
}
