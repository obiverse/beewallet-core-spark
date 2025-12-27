//! Wallet configuration
//!
//! Builder pattern for WalletConfig, matching beewallet-core-breez API.

use super::SparkNetwork;
use std::path::PathBuf;

/// Configuration for wallet connection
///
/// Use the builder pattern:
/// ```rust,ignore
/// let config = WalletConfig::new(SparkNetwork::Testnet)
///     .with_api_key("your-api-key".to_string())
///     .with_working_dir("/path/to/data");
/// ```
#[derive(Debug, Clone)]
pub struct WalletConfig {
    /// Network to connect to
    pub network: SparkNetwork,
    /// Breez API key (required for mainnet, optional for regtest)
    pub api_key: Option<String>,
    /// Working directory for wallet data
    pub working_dir: Option<PathBuf>,
}

impl WalletConfig {
    /// Create new config for the given network
    pub fn new(network: SparkNetwork) -> Self {
        Self {
            network,
            api_key: None,
            working_dir: None,
        }
    }

    /// Set API key (required for mainnet)
    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.api_key = Some(api_key);
        self
    }

    /// Set working directory for wallet data
    pub fn with_working_dir<P: Into<PathBuf>>(mut self, dir: P) -> Self {
        self.working_dir = Some(dir.into());
        self
    }
}

impl Default for WalletConfig {
    fn default() -> Self {
        Self::new(SparkNetwork::Testnet)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let config = WalletConfig::new(SparkNetwork::Mainnet)
            .with_api_key("test-key".to_string())
            .with_working_dir("/tmp/wallet");

        assert_eq!(config.network, SparkNetwork::Mainnet);
        assert_eq!(config.api_key, Some("test-key".to_string()));
        assert_eq!(config.working_dir, Some(PathBuf::from("/tmp/wallet")));
    }

    #[test]
    fn test_config_default() {
        let config = WalletConfig::default();
        assert_eq!(config.network, SparkNetwork::Testnet);
        assert!(config.api_key.is_none());
        assert!(config.working_dir.is_none());
    }
}
