//! # BeeWallet Core (Spark Edition)
//!
//! Experimental Bitcoin + Lightning wallet using Breez SDK Spark.
//!
//! ## Why Spark?
//!
//! Spark uses native Bitcoin via statechain model:
//! - No Liquid sidechain wrapping
//! - No minimum amounts (good for zaps)
//! - Direct Bitcoin, more "sovereign"
//! - Experimental but promising
//!
//! ## How It Works
//!
//! Breez SDK Spark uses statechain transfers:
//! - Receive BTC → Direct to statechain address
//! - Send BTC → Statechain transfer or on-chain sweep
//! - Lightning → Native Lightning via Spark protocol
//!
//! Users see BTC amounts; the SDK handles statechain operations.
//!
//! ## SQLite Conflict Note
//!
//! This crate exists separately from beewallet-core-breez because:
//! - Liquid SDK uses rusqlite ~0.37 (custom fork)
//! - Spark SDK uses rusqlite 0.31 (crates.io)
//! - Both bundle SQLite → symbol conflicts
//!
//! Apps must choose ONE backend at compile time.
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use beewallet_core_spark::*;
//!
//! // Create wallet manager
//! let manager = WalletManager::new(SparkNetwork::Testnet);
//!
//! // Connect with mnemonic
//! manager.connect("your twelve word mnemonic...", None)?;
//!
//! // Get balance
//! let balance = manager.balance()?;
//!
//! // Get receive address
//! let address = manager.new_address()?;
//!
//! // Send to any destination (BTC address or Lightning invoice)
//! manager.send("bc1q...", 50000, None)?;
//!
//! // Create Lightning invoice
//! let invoice = manager.create_invoice(10000, Some("Coffee"))?;
//! ```
//!
//! ## Features
//!
//! - `wallet` - Full wallet with Breez SDK Spark
//! - `crypto` - Crypto primitives for encrypted storage
//! - `keys` - BIP39 + Nostr identity
//! - `std-channel` - Std threading for watch channels

// 9S Protocol - always available
pub mod nine_s;

// Mobi21 Protocol - always available
pub mod mobi;

// Crypto module - encryption primitives
#[cfg(feature = "crypto")]
pub mod vault;

// Keys module - BIP39 + Nostr identity
#[cfg(feature = "keys")]
pub mod keys;

// Nostr module - NIP-06 signing + NIP-44 encryption
#[cfg(feature = "keys")]
pub mod nostr;

// Wallet trait - unified interface for all backends
#[cfg(feature = "wallet")]
pub mod wallet_trait;

// Wallet module - Breez SDK Spark (experimental)
#[cfg(feature = "wallet")]
pub mod wallet_spark;

// ============================================================================
// Re-exports: The Public API
// ============================================================================

// 9S Protocol (always available)
pub use nine_s::{
    current_iso_time, current_time_millis, kingdoms, types as scroll_types, verbs,
    Error as NineSError, FileNamespace, Kernel, MemoryNamespace, Metadata, Namespace,
    Scroll, Store, Tense,
};
// Git-like primitives
pub use nine_s::{Anchor, Patch, PatchError, PatchOp};
// Sealed scrolls (requires crypto)
#[cfg(feature = "crypto")]
pub use nine_s::{SealedScroll, MAX_SEALED_SIZE};

// Mobi21 Protocol (always available)
pub use mobi::{
    derive_from_bytes as mobi_derive_from_bytes,
    derive_from_hex as mobi_derive_from_hex,
    derive_mobinumber as mobi_derive_mobinumber,
    derive_mobinumber_canonical as mobi_derive_mobinumber_canonical,
    display_matches as mobi_display_matches,
    normalize as mobi_normalize,
    validate as mobi_validate,
    Error as MobiError, Mobi,
};

// Vault crypto (crypto feature)
#[cfg(feature = "crypto")]
pub use vault::{
    derive_app_key, derive_key, generate_salt, seal, unseal, CryptoError, RateLimiter,
    SealedValue, SecureSeed, SessionManager,
};

// VaultStore (wallet feature - needs vault initialized)
#[cfg(feature = "wallet")]
pub use vault::VaultStore;

// Keys (keys feature)
#[cfg(feature = "keys")]
pub use keys::{derive_mobinumber, KeyError, MasterKey};

// Nostr (keys feature - signing & encryption)
#[cfg(feature = "keys")]
pub use nostr::{parse_public_key, NostrError, NostrSigner};

// Wallet trait (any wallet backend)
#[cfg(feature = "wallet")]
pub use wallet_trait::{SignedMessage, TransactionDetails, WalletBackend, WalletBalance, WalletError};

// Wallet Spark backend (experimental)
#[cfg(feature = "wallet")]
pub use wallet_spark::{
    SparkNetwork, WalletConfig, WalletManager as SparkWalletManager, WalletNamespace,
    // Event types for payment streaming
    EventListener, SdkEvent, Payment, PaymentState, PaymentType, PaymentDetails,
};

// Default WalletManager alias
#[cfg(feature = "wallet")]
pub use wallet_spark::WalletManager;

/// Type alias for compatibility with beewallet-core-breez
///
/// In megab, code uses `LiquidNetwork::Testnet`. This alias allows
/// the same code to work with Spark backend.
#[cfg(feature = "wallet")]
pub type LiquidNetwork = SparkNetwork;

/// Network enum - compatibility with bitcoin::Network style API
#[cfg(feature = "wallet")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Network {
    /// Bitcoin mainnet
    Bitcoin,
    /// Bitcoin testnet
    Testnet,
    /// Bitcoin testnet4
    Testnet4,
    /// Bitcoin signet
    Signet,
    /// Bitcoin regtest
    Regtest,
}

#[cfg(feature = "wallet")]
impl Network {
    /// Convert to SparkNetwork
    pub fn to_spark_network(self) -> SparkNetwork {
        match self {
            Network::Bitcoin => SparkNetwork::Mainnet,
            Network::Regtest => SparkNetwork::Regtest,
            _ => SparkNetwork::Testnet,
        }
    }
}

#[cfg(feature = "wallet")]
impl Default for Network {
    fn default() -> Self {
        Network::Testnet
    }
}

#[cfg(feature = "wallet")]
impl From<Network> for SparkNetwork {
    fn from(n: Network) -> Self {
        n.to_spark_network()
    }
}

#[cfg(feature = "wallet")]
impl From<SparkNetwork> for Network {
    fn from(n: SparkNetwork) -> Self {
        match n {
            SparkNetwork::Mainnet => Network::Bitcoin,
            SparkNetwork::Testnet => Network::Testnet,
            SparkNetwork::Regtest => Network::Regtest,
        }
    }
}

// Wallet module re-export for compatibility with existing code
#[cfg(feature = "wallet")]
pub mod wallet {
    //! Wallet module - compatibility layer for beewallet-core API

    pub use crate::wallet_spark::{
        SparkNetwork, TransactionDetails, WalletBalance, WalletError, WalletManager,
    };

    /// Message signing module
    pub mod signing {
        pub use crate::wallet_spark::signing::*;
    }
}

/// Sync backend enum - compatibility stub
///
/// Spark SDK handles sync internally, so this is only for API compatibility.
#[cfg(feature = "wallet")]
#[derive(Debug, Clone)]
pub enum SyncBackend {
    /// Electrum backend (URL optional)
    Electrum { url: Option<String> },
    /// Esplora REST API
    Esplora { url: String },
}

#[cfg(feature = "wallet")]
impl Default for SyncBackend {
    fn default() -> Self {
        SyncBackend::Electrum { url: None }
    }
}
