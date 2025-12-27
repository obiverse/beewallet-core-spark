//! Message signing for Breez SDK Spark wallet
//!
//! Wraps Breez SDK Spark's `sign_message` and `check_message` methods.
//!
//! Note: Unlike BDK-style signing which takes mnemonic directly,
//! Breez SDK signing requires a connected wallet instance.
//! Use WalletManager::sign_message() and WalletManager::verify_message()
//! for actual signing operations.
//!
//! The standalone functions here are stubs for API compatibility.

use bitcoin::NetworkKind;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SigningError {
    #[error("SDK error: {0}")]
    SdkError(String),
    #[error("Wallet not connected")]
    NotConnected,
    #[error("Standalone signing not supported - use WalletManager methods")]
    RequiresWalletInstance,
}

/// Signed message result
#[derive(Debug, Clone)]
pub struct SignedMessage {
    /// The pubkey/address that signed
    pub address: String,
    /// The original message
    pub message: String,
    /// The zbase encoded signature
    pub signature: String,
}

/// Sign a message - REQUIRES WALLET INSTANCE
///
/// Standalone signing is not supported in Breez SDK.
/// Use `WalletManager::sign_message()` instead.
pub fn sign_message(
    _mnemonic: &str,
    _passphrase: Option<&str>,
    _address: &str,
    _message: &str,
    _network: NetworkKind,
) -> Result<SignedMessage, SigningError> {
    Err(SigningError::RequiresWalletInstance)
}

/// Verify a signed message - REQUIRES WALLET INSTANCE
///
/// Standalone verification is not supported in Breez SDK.
/// Use `WalletManager::verify_message()` instead.
pub fn verify_message(
    _address: &str,
    _message: &str,
    _signature: &str,
) -> Result<bool, SigningError> {
    Err(SigningError::RequiresWalletInstance)
}
