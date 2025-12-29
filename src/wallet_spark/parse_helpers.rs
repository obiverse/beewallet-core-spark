//! Input parsing helpers for WalletNamespace
//!
//! Pure functions for detecting and parsing payment input types.
//! Used to pre-detect LNURL variants before SDK parsing.

use bech32::FromBase32;

/// LNURL-Auth detection result
#[derive(Debug, Clone)]
pub struct LnUrlAuthDetection {
    pub domain: String,
    pub url: String,
    pub k1: String,
    pub action: Option<String>,
}

/// Try to decode LNURL bech32 and detect LNURL-Auth (tag=login)
///
/// Returns Some(LnUrlAuthDetection) if this is an LNURL-Auth,
/// None otherwise (should fall through to SDK parse).
///
/// This pre-detection is needed because some LNURL-Auth QR codes get
/// misdetected as LNURL-Pay by the SDK.
///
/// # Arguments
/// * `input` - Raw input string (LNURL bech32 encoded)
///
/// # Returns
/// * `Some(LnUrlAuthDetection)` - If input is a valid LNURL-Auth
/// * `None` - If not LNURL-Auth (let SDK handle it)
pub fn try_detect_lnurl_auth(input: &str) -> Option<LnUrlAuthDetection> {
    let input_lower = input.to_lowercase();

    // Only try to decode bech32 LNURL (starts with "lnurl")
    // keyauth:// is handled by SDK directly
    if !input_lower.starts_with("lnurl1") {
        return None;
    }

    // Try bech32 decode
    let (_hrp, data, _variant) = bech32::decode(input).ok()?;
    let decoded_bytes = Vec::<u8>::from_base32(&data).ok()?;
    let decoded_url = String::from_utf8(decoded_bytes).ok()?;

    // Check for tag=login in the decoded URL
    if !decoded_url.contains("tag=login") {
        return None;
    }

    // Extract domain
    let url = url::Url::parse(&decoded_url).ok()?;
    let domain = url.domain()?.to_string();

    // Extract k1 (required for LNURL-Auth)
    let k1 = url
        .query_pairs()
        .find(|(key, _)| key == "k1")
        .map(|(_, v)| v.to_string())?;

    // Validate k1 is 32 bytes hex
    let k1_bytes = hex::decode(&k1).ok()?;
    if k1_bytes.len() != 32 {
        return None;
    }

    // Extract action (optional: register, login, link, auth)
    let action = url
        .query_pairs()
        .find(|(key, _)| key == "action")
        .map(|(_, v)| v.to_string())
        .filter(|a| ["register", "login", "link", "auth"].contains(&a.as_str()));

    Some(LnUrlAuthDetection {
        domain,
        url: decoded_url,
        k1,
        action,
    })
}

/// Check if input looks like a Lightning invoice (BOLT11)
pub fn is_bolt11(input: &str) -> bool {
    let lower = input.to_lowercase();
    lower.starts_with("lnbc") || lower.starts_with("lntb") || lower.starts_with("lnbcrt")
}

/// Check if input looks like a Bitcoin address
pub fn is_bitcoin_address(input: &str) -> bool {
    // Basic prefix checks
    input.starts_with("bc1")
        || input.starts_with("tb1")
        || input.starts_with("bcrt1")
        || input.starts_with('1')
        || input.starts_with('3')
        || input.starts_with('m')
        || input.starts_with('n')
        || input.starts_with('2')
}

/// Check if input looks like an LNURL
pub fn is_lnurl(input: &str) -> bool {
    let lower = input.to_lowercase();
    lower.starts_with("lnurl") || lower.contains('@') // Lightning Address
}

/// Detect input type for routing
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputTypeHint {
    Bolt11,
    BitcoinAddress,
    LnUrl,
    LnUrlAuth,
    Unknown,
}

/// Quick detection of input type (for routing before full parsing)
pub fn detect_input_type(input: &str) -> InputTypeHint {
    // Check for LNURL-Auth first (most specific)
    if try_detect_lnurl_auth(input).is_some() {
        return InputTypeHint::LnUrlAuth;
    }

    if is_bolt11(input) {
        return InputTypeHint::Bolt11;
    }

    if is_bitcoin_address(input) {
        return InputTypeHint::BitcoinAddress;
    }

    if is_lnurl(input) {
        return InputTypeHint::LnUrl;
    }

    InputTypeHint::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_non_lnurl_returns_none() {
        assert!(try_detect_lnurl_auth("bitcoin:bc1q...").is_none());
        assert!(try_detect_lnurl_auth("lnbc1...").is_none());
        assert!(try_detect_lnurl_auth("random string").is_none());
    }

    #[test]
    fn test_is_bolt11() {
        assert!(is_bolt11("lnbc1234..."));
        assert!(is_bolt11("LNBC1234...")); // Case insensitive
        assert!(is_bolt11("lntb1234...")); // Testnet
        assert!(is_bolt11("lnbcrt1234...")); // Regtest
        assert!(!is_bolt11("bc1q..."));
        assert!(!is_bolt11("lnurl1..."));
    }

    #[test]
    fn test_is_bitcoin_address() {
        // Mainnet
        assert!(is_bitcoin_address("bc1qtest..."));
        assert!(is_bitcoin_address("1Bitcoin..."));
        assert!(is_bitcoin_address("3Multi..."));

        // Testnet
        assert!(is_bitcoin_address("tb1qtest..."));
        assert!(is_bitcoin_address("mTest..."));
        assert!(is_bitcoin_address("nTest..."));
        assert!(is_bitcoin_address("2Multi..."));

        // Regtest
        assert!(is_bitcoin_address("bcrt1..."));

        // Not addresses
        assert!(!is_bitcoin_address("lnbc1..."));
        assert!(!is_bitcoin_address("lnurl1..."));
    }

    #[test]
    fn test_is_lnurl() {
        assert!(is_lnurl("lnurl1..."));
        assert!(is_lnurl("LNURL1...")); // Case insensitive
        assert!(is_lnurl("user@example.com")); // Lightning Address
        assert!(!is_lnurl("bc1q..."));
        assert!(!is_lnurl("lnbc1..."));
    }

    #[test]
    fn test_detect_input_type() {
        assert_eq!(detect_input_type("lnbc1..."), InputTypeHint::Bolt11);
        assert_eq!(detect_input_type("bc1qtest..."), InputTypeHint::BitcoinAddress);
        assert_eq!(detect_input_type("random"), InputTypeHint::Unknown);
    }
}
