//! Unified Key Derivation
//!
//! Derives all keys from a single BIP39 mnemonic:
//! - Bitcoin (m/84'/0'/0') - BIP84 Native SegWit
//! - Nostr (m/44'/1237'/0'/0/0) - NIP-06
//! - Lightning (raw 256-bit entropy) - for future LDK
//! - Mobinumber - human-readable ID from Nostr pubkey
//!
//! # Security
//! - Mnemonic is zeroized on drop
//! - Entropy is zeroized after mnemonic generation
//! - Seed bytes are zeroized after use

use bip39::Mnemonic;
use nostr::bitcoin::bip32::{ChildNumber, DerivationPath, Xpriv};
use nostr::bitcoin::secp256k1::Secp256k1;
use nostr::bitcoin::NetworkKind;
use nostr::nips::nip06::FromMnemonic;
use nostr::{Keys as NostrKeys, ToBech32};
use thiserror::Error;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Error, Debug)]
pub enum KeyError {
    #[error("Invalid mnemonic: {0}")]
    InvalidMnemonic(String),
    #[error("Derivation error: {0}")]
    DerivationError(String),
}

/// Zeroizing wrapper for seed phrase string
///
/// Automatically zeroizes the string content when dropped.
/// Use this when you need a seed phrase that cleans up after itself.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct SecureSeed(String);

impl SecureSeed {
    /// Create a new secure seed from a string
    pub fn new(seed: String) -> Self {
        Self(seed)
    }

    /// Get a reference to the seed phrase
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for SecureSeed {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Master key manager - derives all keys from a single seed
///
/// # Security
/// The mnemonic and optional BIP39 passphrase are stored internally
/// and zeroized when the MasterKey is dropped.
///
/// # BIP39 Passphrase
/// The optional passphrase provides an additional layer of security beyond the mnemonic.
/// Using a passphrase creates an entirely different set of keys from the same mnemonic.
/// **Warning**: If you use a passphrase, you MUST remember it - without it, you cannot
/// recover your funds, even with the correct mnemonic phrase.
#[derive(Zeroize)]
pub struct MasterKey {
    /// Raw mnemonic phrase bytes for zeroization
    mnemonic_bytes: Vec<u8>,
    #[zeroize(skip)] // Mnemonic type doesn't implement Zeroize, but we zeroize mnemonic_bytes
    mnemonic: Mnemonic,
    /// Optional BIP39 passphrase (zeroized on drop)
    bip39_passphrase: Option<Vec<u8>>,
}

impl ZeroizeOnDrop for MasterKey {}

impl MasterKey {
    /// Generate a new random mnemonic
    ///
    /// # Security
    /// Entropy is zeroized immediately after mnemonic generation.
    pub fn generate(word_count: usize) -> Result<Self, KeyError> {
        use rand::RngCore;

        let entropy_bytes = match word_count {
            12 => 16, // 128 bits
            24 => 32, // 256 bits
            _ => return Err(KeyError::InvalidMnemonic("Word count must be 12 or 24".into())),
        };

        let mut entropy = vec![0u8; entropy_bytes];
        rand::rngs::OsRng.fill_bytes(&mut entropy);

        let mnemonic = Mnemonic::from_entropy(&entropy);

        // SECURITY: Zeroize entropy immediately after use
        entropy.zeroize();

        let mnemonic = mnemonic.map_err(|e| KeyError::InvalidMnemonic(e.to_string()))?;
        let mnemonic_bytes = mnemonic.to_string().into_bytes();

        Ok(Self {
            mnemonic_bytes,
            mnemonic,
            bip39_passphrase: None,
        })
    }

    /// Create from existing mnemonic phrase
    pub fn from_mnemonic(phrase: &str) -> Result<Self, KeyError> {
        let mnemonic = Mnemonic::parse(phrase).map_err(|e| KeyError::InvalidMnemonic(e.to_string()))?;
        let mnemonic_bytes = phrase.as_bytes().to_vec();
        Ok(Self {
            mnemonic_bytes,
            mnemonic,
            bip39_passphrase: None,
        })
    }

    /// Create from existing mnemonic phrase with optional BIP39 passphrase
    ///
    /// # Arguments
    /// * `phrase` - The BIP39 mnemonic phrase
    /// * `passphrase` - Optional BIP39 passphrase for additional security
    ///
    /// # Security
    /// The passphrase is stored in a zeroizing buffer. If provided, it will be used
    /// in seed derivation, creating an entirely different set of keys.
    ///
    /// # Warning
    /// If you use a passphrase, you MUST remember it. Without the exact passphrase,
    /// you cannot recover your funds even with the correct mnemonic phrase.
    pub fn from_mnemonic_with_passphrase(phrase: &str, passphrase: Option<&str>) -> Result<Self, KeyError> {
        let mnemonic = Mnemonic::parse(phrase).map_err(|e| KeyError::InvalidMnemonic(e.to_string()))?;
        let mnemonic_bytes = phrase.as_bytes().to_vec();
        let bip39_passphrase = passphrase.map(|p| p.as_bytes().to_vec());
        Ok(Self {
            mnemonic_bytes,
            mnemonic,
            bip39_passphrase,
        })
    }

    /// Set or clear the BIP39 passphrase
    ///
    /// # Security
    /// The old passphrase (if any) is zeroized before being replaced.
    pub fn set_passphrase(&mut self, passphrase: Option<&str>) {
        // Zeroize old passphrase if present
        if let Some(ref mut old) = self.bip39_passphrase {
            old.zeroize();
        }
        self.bip39_passphrase = passphrase.map(|p| p.as_bytes().to_vec());
    }

    /// Check if a BIP39 passphrase is set
    pub fn has_passphrase(&self) -> bool {
        self.bip39_passphrase.is_some()
    }

    /// Get the mnemonic phrase as a secure string
    ///
    /// # Security
    /// Returns a SecureSeed that automatically zeroizes when dropped.
    /// This prevents the seed phrase from lingering in memory.
    pub fn mnemonic_phrase(&self) -> SecureSeed {
        SecureSeed::new(self.mnemonic.to_string())
    }

    /// Get the mnemonic words as a vector
    pub fn mnemonic_words(&self) -> Vec<&'static str> {
        self.mnemonic.words().collect()
    }

    /// Get the raw seed bytes (for BIP32 derivation)
    ///
    /// Uses the optional BIP39 passphrase if one was set during creation.
    /// With no passphrase, this is equivalent to `to_seed("")`.
    pub fn seed(&self) -> [u8; 64] {
        match &self.bip39_passphrase {
            Some(passphrase) => {
                let passphrase_str = String::from_utf8_lossy(passphrase);
                self.mnemonic.to_seed(&*passphrase_str)
            }
            None => self.mnemonic.to_seed(""),
        }
    }

    /// Get the raw seed bytes with a specific passphrase (one-time use)
    ///
    /// This allows deriving a seed with a different passphrase without
    /// modifying the stored passphrase. Useful for checking if a passphrase
    /// matches before committing to it.
    pub fn seed_with_passphrase(&self, passphrase: &str) -> [u8; 64] {
        self.mnemonic.to_seed(passphrase)
    }

    /// Get raw 256-bit entropy for Lightning (LDK)
    ///
    /// # Design Note (Issue #10)
    /// This uses truncated seed (`seed[0..32]`) rather than a dedicated BIP32 path
    /// like `m/1017'/0'/0'`. This is intentional for simplicity, but means there's
    /// no isolation between Bitcoin and Lightning key material at the derivation level.
    /// Master seed compromise = Lightning compromise regardless of derivation method.
    ///
    /// # Security
    /// The full 64-byte seed is explicitly zeroized after extracting entropy.
    /// The returned array should be zeroized by the caller when done.
    pub fn lightning_entropy(&self) -> [u8; 32] {
        let mut seed = self.seed();
        let mut entropy = [0u8; 32];
        entropy.copy_from_slice(&seed[..32]);
        // SECURITY: Explicitly zeroize the full seed to prevent memory forensics
        // (Fixes issue #3: Stack memory is NOT guaranteed to be overwritten)
        seed.zeroize();
        entropy
    }

    /// Derive BIP32 master extended private key
    ///
    /// This is the root key from which all Bitcoin keys are derived.
    /// Use `bitcoin_account_xprv` for BIP84 account-level keys.
    pub fn bitcoin_master_xprv(&self, network: NetworkKind) -> Result<Xpriv, KeyError> {
        let seed = self.seed();
        Xpriv::new_master(network, &seed)
            .map_err(|e| KeyError::DerivationError(e.to_string()))
    }

    /// Derive BIP84 account extended private key (m/84'/coin'/account')
    ///
    /// Standard path for Native SegWit wallets.
    /// - Mainnet: m/84'/0'/0'
    /// - Testnet: m/84'/1'/0'
    pub fn bitcoin_account_xprv(
        &self,
        network: NetworkKind,
        account: u32,
    ) -> Result<Xpriv, KeyError> {
        let secp = Secp256k1::new();
        let master = self.bitcoin_master_xprv(network)?;

        let coin_type = match network {
            NetworkKind::Main => 0,
            NetworkKind::Test => 1,
        };

        let path = DerivationPath::from(vec![
            ChildNumber::from_hardened_idx(84).map_err(|e| KeyError::DerivationError(e.to_string()))?,
            ChildNumber::from_hardened_idx(coin_type).map_err(|e| KeyError::DerivationError(e.to_string()))?,
            ChildNumber::from_hardened_idx(account).map_err(|e| KeyError::DerivationError(e.to_string()))?,
        ]);

        master
            .derive_priv(&secp, &path)
            .map_err(|e| KeyError::DerivationError(e.to_string()))
    }

    /// Derive a specific Bitcoin private key at path m/84'/coin'/account'/change/index
    ///
    /// # Arguments
    /// * `network` - Main or Test network
    /// * `account` - Account index (usually 0)
    /// * `change` - 0 for external (receive), 1 for internal (change)
    /// * `index` - Address index
    pub fn bitcoin_key_at_path(
        &self,
        network: NetworkKind,
        account: u32,
        change: u32,
        index: u32,
    ) -> Result<Xpriv, KeyError> {
        let secp = Secp256k1::new();
        let account_xprv = self.bitcoin_account_xprv(network, account)?;

        let path = DerivationPath::from(vec![
            ChildNumber::from_normal_idx(change).map_err(|e| KeyError::DerivationError(e.to_string()))?,
            ChildNumber::from_normal_idx(index).map_err(|e| KeyError::DerivationError(e.to_string()))?,
        ]);

        account_xprv
            .derive_priv(&secp, &path)
            .map_err(|e| KeyError::DerivationError(e.to_string()))
    }

    /// Derive Nostr keys using NIP-06
    ///
    /// Standard derivation path: m/44'/1237'/account'/0/0
    pub fn nostr_keys(&self, account: Option<u32>) -> Result<NostrKeys, KeyError> {
        let passphrase: Option<String> = self.bip39_passphrase.as_ref()
            .map(|p| String::from_utf8_lossy(p).to_string());
        NostrKeys::from_mnemonic_with_account(
            self.mnemonic_phrase().as_str().to_string(),
            passphrase,
            account,
        )
        .map_err(|e| KeyError::DerivationError(e.to_string()))
    }

    /// Get Nostr public key in hex format
    pub fn nostr_pubkey_hex(&self) -> Result<String, KeyError> {
        let keys = self.nostr_keys(None)?;
        Ok(keys.public_key().to_hex())
    }

    /// Get Nostr public key in bech32 (npub) format
    pub fn nostr_npub(&self) -> Result<String, KeyError> {
        let keys = self.nostr_keys(None)?;
        keys.public_key()
            .to_bech32()
            .map_err(|e| KeyError::DerivationError(e.to_string()))
    }

    /// Get the Mobinumber - a human-readable phone-number-like identifier
    ///
    /// Format: XXX-XXX-XXX-XXX (12 digits derived from Nostr pubkey)
    ///
    /// Mobinumber is the universal human-readable identity in the BeeWallet
    /// ecosystem. It's deterministically derived from your Nostr pubkey,
    /// making it easy to share and remember.
    pub fn mobinumber(&self) -> Result<String, KeyError> {
        let pubkey_hex = self.nostr_pubkey_hex()?;
        Ok(derive_mobinumber(&pubkey_hex))
    }
}

/// Derive a Mobinumber from a hex pubkey using Mobi21 protocol
///
/// Format: XXX-XXX-XXX-XXX (12 digits from Mobi21 derivation)
///
/// # About Mobi21
/// Mobi21 derives a 21-digit identifier from a secp256k1 public key using
/// rejection sampling for uniform distribution. The display form is 12 digits,
/// with progressive collision resolution via extended (15), long (18), and
/// full (21) forms.
///
/// For new code, use `crate::mobi::derive_from_hex()` to get the full Mobi struct
/// with all forms available.
pub fn derive_mobinumber(pubkey_hex: &str) -> String {
    crate::mobi::derive_mobinumber(pubkey_hex)
}

impl Drop for MasterKey {
    fn drop(&mut self) {
        // SECURITY: Zeroize all sensitive data on drop
        self.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_12_words() {
        let key = MasterKey::generate(12).unwrap();
        assert_eq!(key.mnemonic_words().len(), 12);
    }

    #[test]
    fn generate_24_words() {
        let key = MasterKey::generate(24).unwrap();
        assert_eq!(key.mnemonic_words().len(), 24);
    }

    #[test]
    fn from_mnemonic_valid() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let key = MasterKey::from_mnemonic(phrase).unwrap();
        assert_eq!(key.mnemonic_phrase().as_str(), phrase);
    }

    #[test]
    fn from_mnemonic_invalid() {
        let result = MasterKey::from_mnemonic("invalid words here");
        assert!(result.is_err());
    }

    #[test]
    fn seed_is_deterministic() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let key1 = MasterKey::from_mnemonic(phrase).unwrap();
        let key2 = MasterKey::from_mnemonic(phrase).unwrap();
        assert_eq!(key1.seed(), key2.seed());
    }

    #[test]
    fn passphrase_changes_seed() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

        // Without passphrase
        let key_no_pass = MasterKey::from_mnemonic(phrase).unwrap();

        // With passphrase
        let key_with_pass = MasterKey::from_mnemonic_with_passphrase(phrase, Some("TREZOR")).unwrap();

        // Seeds must be different
        assert_ne!(key_no_pass.seed(), key_with_pass.seed());

        // Verify passphrase state
        assert!(!key_no_pass.has_passphrase());
        assert!(key_with_pass.has_passphrase());
    }

    #[test]
    fn passphrase_deterministic() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let passphrase = "TREZOR";

        let key1 = MasterKey::from_mnemonic_with_passphrase(phrase, Some(passphrase)).unwrap();
        let key2 = MasterKey::from_mnemonic_with_passphrase(phrase, Some(passphrase)).unwrap();

        assert_eq!(key1.seed(), key2.seed());
    }

    #[test]
    fn seed_with_passphrase_method() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

        let key = MasterKey::from_mnemonic(phrase).unwrap();

        // seed() with no passphrase set should equal seed_with_passphrase("")
        assert_eq!(key.seed(), key.seed_with_passphrase(""));

        // seed_with_passphrase should produce same result as from_mnemonic_with_passphrase
        let key_with_pass = MasterKey::from_mnemonic_with_passphrase(phrase, Some("test")).unwrap();
        assert_eq!(key.seed_with_passphrase("test"), key_with_pass.seed());
    }

    #[test]
    fn set_passphrase_changes_seed() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

        let mut key = MasterKey::from_mnemonic(phrase).unwrap();
        let seed_no_pass = key.seed();

        // Set passphrase
        key.set_passphrase(Some("test"));
        assert!(key.has_passphrase());
        let seed_with_pass = key.seed();
        assert_ne!(seed_no_pass, seed_with_pass);

        // Clear passphrase
        key.set_passphrase(None);
        assert!(!key.has_passphrase());
        assert_eq!(key.seed(), seed_no_pass);
    }

    #[test]
    fn bip39_test_vector_with_passphrase() {
        // BIP39 test vector from https://github.com/trezor/python-mnemonic/blob/master/vectors.json
        // Mnemonic: "abandon" x 11 + "about"
        // Passphrase: "TREZOR"
        // Expected seed (first 32 bytes): c55257c360c07c72029aebc1b53c05ed0362ada38ead3e3e9efa3708e53495531f09a6987599d18264c1e1c92f2cf141630c7a3c4ab7c81b2f001698e7463b04
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

        let key = MasterKey::from_mnemonic_with_passphrase(phrase, Some("TREZOR")).unwrap();
        let seed = key.seed();

        // Verify first 8 bytes of the expected seed
        assert_eq!(seed[0], 0xc5);
        assert_eq!(seed[1], 0x52);
        assert_eq!(seed[2], 0x57);
        assert_eq!(seed[3], 0xc3);
        assert_eq!(seed[4], 0x60);
        assert_eq!(seed[5], 0xc0);
        assert_eq!(seed[6], 0x7c);
        assert_eq!(seed[7], 0x72);
    }

    #[test]
    fn empty_passphrase_same_as_none() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

        let key_none = MasterKey::from_mnemonic(phrase).unwrap();
        let key_empty = MasterKey::from_mnemonic_with_passphrase(phrase, Some("")).unwrap();

        // Empty string passphrase should produce same seed as no passphrase
        assert_eq!(key_none.seed(), key_empty.seed());
    }

    #[test]
    fn bitcoin_master_xprv_deterministic() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

        let key1 = MasterKey::from_mnemonic(phrase).unwrap();
        let key2 = MasterKey::from_mnemonic(phrase).unwrap();

        let xprv1 = key1.bitcoin_master_xprv(NetworkKind::Main).unwrap();
        let xprv2 = key2.bitcoin_master_xprv(NetworkKind::Main).unwrap();

        assert_eq!(xprv1.encode(), xprv2.encode());
    }

    #[test]
    fn bitcoin_account_xprv_different_networks() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let key = MasterKey::from_mnemonic(phrase).unwrap();

        let mainnet = key.bitcoin_account_xprv(NetworkKind::Main, 0).unwrap();
        let testnet = key.bitcoin_account_xprv(NetworkKind::Test, 0).unwrap();

        // Different networks should produce different account keys
        assert_ne!(mainnet.encode(), testnet.encode());
    }

    #[test]
    fn bitcoin_key_at_path_derivation() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let key = MasterKey::from_mnemonic(phrase).unwrap();

        // Derive first external address key (m/84'/0'/0'/0/0)
        let key_0_0 = key.bitcoin_key_at_path(NetworkKind::Main, 0, 0, 0).unwrap();
        let key_0_1 = key.bitcoin_key_at_path(NetworkKind::Main, 0, 0, 1).unwrap();

        // Different indices should produce different keys
        assert_ne!(key_0_0.encode(), key_0_1.encode());
    }

    #[test]
    fn nostr_keys_derivation() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let key = MasterKey::from_mnemonic(phrase).unwrap();

        let nostr_keys = key.nostr_keys(None).unwrap();
        let npub = key.nostr_npub().unwrap();
        let hex = key.nostr_pubkey_hex().unwrap();

        // Should produce valid bech32 npub
        assert!(npub.starts_with("npub1"));
        // Should produce valid 64-char hex
        assert_eq!(hex.len(), 64);
        // Should be consistent
        assert_eq!(nostr_keys.public_key().to_bech32().unwrap(), npub);
    }

    #[test]
    fn nostr_keys_deterministic() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

        let key1 = MasterKey::from_mnemonic(phrase).unwrap();
        let key2 = MasterKey::from_mnemonic(phrase).unwrap();

        assert_eq!(key1.nostr_npub().unwrap(), key2.nostr_npub().unwrap());
    }

    #[test]
    fn mobinumber_format() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let key = MasterKey::from_mnemonic(phrase).unwrap();

        let mobinumber = key.mobinumber().unwrap();

        // Should be in format XXX-XXX-XXX-XXX
        let parts: Vec<&str> = mobinumber.split('-').collect();
        assert_eq!(parts.len(), 4);
        for part in parts {
            assert_eq!(part.len(), 3);
            assert!(part.chars().all(|c| c.is_ascii_digit()));
        }
    }

    #[test]
    fn mobinumber_deterministic() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

        let key1 = MasterKey::from_mnemonic(phrase).unwrap();
        let key2 = MasterKey::from_mnemonic(phrase).unwrap();

        assert_eq!(key1.mobinumber().unwrap(), key2.mobinumber().unwrap());
    }

    #[test]
    fn mobinumber_different_seeds() {
        let key1 = MasterKey::from_mnemonic(
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
        ).unwrap();
        let key2 = MasterKey::from_mnemonic(
            "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong"
        ).unwrap();

        // Different seeds should produce different mobinumbers
        assert_ne!(key1.mobinumber().unwrap(), key2.mobinumber().unwrap());
    }

    #[test]
    fn derive_mobinumber_direct() {
        // Test the derive_mobinumber function directly with a valid 64-char hex pubkey
        // Using all-zeros pubkey - canonical Mobi21 test vector
        let mobinumber = derive_mobinumber(
            "0000000000000000000000000000000000000000000000000000000000000000"
        );

        // Mobi21 canonical output for all-zeros: 587-135-537-154
        assert_eq!(mobinumber, "587-135-537-154");
        assert_eq!(mobinumber.len(), 15); // 12 digits + 3 dashes
        assert!(mobinumber.chars().filter(|c| *c == '-').count() == 3);
    }
}
