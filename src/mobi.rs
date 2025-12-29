//! Mobi21 Protocol - Rust FFI bindings
//!
//! Safe Rust wrapper around the mobi C library.
//! Derives a 21-digit identifier from a secp256k1 public key.
//!
//! Display 12 digits to users. Store full 21. Resolve collisions progressively.
//!
//! # Example
//! ```
//! use beewallet_core_spark::mobi::{Mobi, derive_from_hex};
//!
//! let mobi = derive_from_hex("17162c921dc4d2518f9a101db33695df1afb56ab82f5ff3e5da6eec3ca5cd917").unwrap();
//! println!("Display: {}", mobi.display_formatted());  // "879-044-656-584"
//! println!("Full: {}", mobi.full);                    // "879044656584686196443"
//! ```
//!
//! Copyright (c) 2024-2025 OBIVERSE LLC
//! Licensed under MIT OR Apache-2.0

use std::ffi::{c_char, c_int, c_uchar, CStr};
use std::mem::MaybeUninit;
use thiserror::Error;

// FFI declarations matching mobi.h
#[repr(C)]
struct MobiT {
    full: [c_char; 22],     // 21 digits + null
    display: [c_char; 13],  // 12 digits + null
    extended: [c_char; 16], // 15 digits + null
    lng: [c_char; 19],      // 18 digits + null
}

#[allow(dead_code)]
extern "C" {
    fn mobi_derive(pubkey_hex: *const c_char, out: *mut MobiT) -> c_int;
    fn mobi_derive_bytes(pubkey: *const c_uchar, out: *mut MobiT) -> c_int;
    fn mobi_format_display(mobi: *const MobiT, out: *mut c_char) -> c_int;
    fn mobi_format_extended(mobi: *const MobiT, out: *mut c_char) -> c_int;
    fn mobi_format_full(mobi: *const MobiT, out: *mut c_char) -> c_int;
    fn mobi_normalize(input: *const c_char, out: *mut c_char, out_len: usize) -> c_int;
    fn mobi_validate(mobi: *const c_char) -> c_int;
    fn mobi_display_matches(a: *const c_char, b: *const c_char) -> c_int;
}

/// Mobi21 error types
#[derive(Debug, Error)]
pub enum Error {
    #[error("Invalid hex: contains non-hexadecimal characters")]
    InvalidHex,

    #[error("Invalid length: expected 64 hex characters (32 bytes)")]
    InvalidLength,

    #[error("Invalid character in mobi string")]
    InvalidChar,

    #[error("Null pointer in FFI call")]
    NullPointer,

    #[error("Unknown error from C library: {0}")]
    Unknown(i32),
}

pub type Result<T> = std::result::Result<T, Error>;

fn error_from_code(code: c_int) -> Error {
    match code {
        -1 => Error::NullPointer,
        -2 => Error::InvalidHex,
        -3 => Error::InvalidLength,
        -4 => Error::InvalidChar,
        _ => Error::Unknown(code),
    }
}

/// Mobi21 - Complete 21-digit identifier with all display forms
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mobi {
    /// Full 21 digits - canonical form (always unique)
    pub full: String,
    /// First 12 digits - display form
    pub display: String,
    /// First 15 digits - extended form (collision resolution L1)
    pub extended: String,
    /// First 18 digits - long form (collision resolution L2)
    pub lng: String,
}

impl Mobi {
    /// Format display form with hyphens: XXX-XXX-XXX-XXX
    pub fn display_formatted(&self) -> String {
        format_with_hyphens(&self.display)
    }

    /// Format extended form with hyphens: XXX-XXX-XXX-XXX-XXX
    pub fn extended_formatted(&self) -> String {
        format_with_hyphens(&self.extended)
    }

    /// Format long form with hyphens: XXX-XXX-XXX-XXX-XXX-XXX
    pub fn lng_formatted(&self) -> String {
        format_with_hyphens(&self.lng)
    }

    /// Format full form with hyphens: XXX-XXX-XXX-XXX-XXX-XXX-XXX
    pub fn full_formatted(&self) -> String {
        format_with_hyphens(&self.full)
    }

    /// Check if display forms match (first 12 digits)
    pub fn display_matches(&self, other: &Mobi) -> bool {
        self.display == other.display
    }

    /// Check if full forms match (all 21 digits)
    pub fn full_matches(&self, other: &Mobi) -> bool {
        self.full == other.full
    }
}

/// Format digits with hyphens every 3 digits
fn format_with_hyphens(digits: &str) -> String {
    digits
        .chars()
        .collect::<Vec<_>>()
        .chunks(3)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("-")
}

/// Derive Mobi from hex-encoded public key (64 hex characters)
pub fn derive_from_hex(pubkey_hex: &str) -> Result<Mobi> {
    if pubkey_hex.len() != 64 {
        return Err(Error::InvalidLength);
    }

    // Create null-terminated C string
    let mut hex_buf = [0u8; 65];
    hex_buf[..64].copy_from_slice(pubkey_hex.as_bytes());

    let mut mobi = MaybeUninit::<MobiT>::uninit();

    let result = unsafe { mobi_derive(hex_buf.as_ptr() as *const c_char, mobi.as_mut_ptr()) };

    if result != 0 {
        return Err(error_from_code(result));
    }

    let mobi = unsafe { mobi.assume_init() };
    Ok(mobi_t_to_mobi(&mobi))
}

/// Derive Mobi from raw 32-byte public key
pub fn derive_from_bytes(pubkey: &[u8; 32]) -> Result<Mobi> {
    let mut mobi = MaybeUninit::<MobiT>::uninit();

    let result = unsafe { mobi_derive_bytes(pubkey.as_ptr(), mobi.as_mut_ptr()) };

    if result != 0 {
        return Err(error_from_code(result));
    }

    let mobi = unsafe { mobi.assume_init() };
    Ok(mobi_t_to_mobi(&mobi))
}

/// Convert C MobiT to Rust Mobi
fn mobi_t_to_mobi(mobi: &MobiT) -> Mobi {
    Mobi {
        full: c_char_array_to_string(&mobi.full),
        display: c_char_array_to_string(&mobi.display),
        extended: c_char_array_to_string(&mobi.extended),
        lng: c_char_array_to_string(&mobi.lng),
    }
}

/// Convert C char array to Rust String
fn c_char_array_to_string<const N: usize>(arr: &[c_char; N]) -> String {
    unsafe { CStr::from_ptr(arr.as_ptr()) }
        .to_string_lossy()
        .into_owned()
}

/// Normalize mobi string - strip formatting, return digits only
///
/// Accepts: "879-044-656-584" or "879044656584" or "879 044 656 584"
/// Returns: "879044656584" (digits only)
pub fn normalize(input: &str) -> Result<String> {
    let mut input_buf = vec![0u8; input.len() + 1];
    input_buf[..input.len()].copy_from_slice(input.as_bytes());

    let mut out_buf = [0u8; 32];

    let result = unsafe {
        mobi_normalize(
            input_buf.as_ptr() as *const c_char,
            out_buf.as_mut_ptr() as *mut c_char,
            out_buf.len(),
        )
    };

    if result < 0 {
        return Err(error_from_code(result));
    }

    Ok(unsafe { CStr::from_ptr(out_buf.as_ptr() as *const c_char) }
        .to_string_lossy()
        .into_owned())
}

/// Validate mobi string (12, 15, 18, or 21 digits)
pub fn validate(mobi: &str) -> bool {
    let mut buf = vec![0u8; mobi.len() + 1];
    buf[..mobi.len()].copy_from_slice(mobi.as_bytes());

    unsafe { mobi_validate(buf.as_ptr() as *const c_char) == 1 }
}

/// Check if two mobi strings match on display form (first 12 digits)
pub fn display_matches(a: &str, b: &str) -> bool {
    let mut a_buf = vec![0u8; a.len() + 1];
    a_buf[..a.len()].copy_from_slice(a.as_bytes());

    let mut b_buf = vec![0u8; b.len() + 1];
    b_buf[..b.len()].copy_from_slice(b.as_bytes());

    unsafe {
        mobi_display_matches(
            a_buf.as_ptr() as *const c_char,
            b_buf.as_ptr() as *const c_char,
        ) == 1
    }
}

// ============================================================================
// Legacy API (for backward compatibility)
// ============================================================================

/// Derive 12-digit mobinumber from hex pubkey (legacy API)
///
/// Returns formatted display form: "XXX-XXX-XXX-XXX"
///
/// For new code, use `derive_from_hex()` to get the full Mobi struct.
pub fn derive_mobinumber(pubkey_hex: &str) -> String {
    match derive_from_hex(pubkey_hex) {
        Ok(mobi) => mobi.display_formatted(),
        Err(_) => "000-000-000-000".to_string(),
    }
}

/// Derive 12-digit mobinumber, unformatted (legacy API)
///
/// Returns canonical display form: "879044656584"
pub fn derive_mobinumber_canonical(pubkey_hex: &str) -> String {
    match derive_from_hex(pubkey_hex) {
        Ok(mobi) => mobi.display,
        Err(_) => "000000000000".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Canonical test vectors from mobi C reference implementation
    const ALL_ZERO_PUBKEY: &str =
        "0000000000000000000000000000000000000000000000000000000000000000";
    const ABANDON_PUBKEY: &str =
        "17162c921dc4d2518f9a101db33695df1afb56ab82f5ff3e5da6eec3ca5cd917";

    #[test]
    fn test_derive_all_zeros() {
        let mobi = derive_from_hex(ALL_ZERO_PUBKEY).unwrap();

        // Canonical test vector - MUST match C implementation
        assert_eq!(mobi.full, "587135537154686717107");
        assert_eq!(mobi.display, "587135537154");
        assert_eq!(mobi.extended, "587135537154686");
        assert_eq!(mobi.lng, "587135537154686717");

        assert_eq!(mobi.display_formatted(), "587-135-537-154");
        assert_eq!(mobi.full_formatted(), "587-135-537-154-686-717-107");
    }

    #[test]
    fn test_derive_abandon() {
        let mobi = derive_from_hex(ABANDON_PUBKEY).unwrap();

        // Canonical test vector - MUST match C implementation
        assert_eq!(mobi.full, "879044656584686196443");
        assert_eq!(mobi.display, "879044656584");

        assert_eq!(mobi.display_formatted(), "879-044-656-584");
    }

    #[test]
    fn test_derive_bytes() {
        let pubkey = [0u8; 32];
        let mobi = derive_from_bytes(&pubkey).unwrap();

        assert_eq!(mobi.full, "587135537154686717107");
    }

    #[test]
    fn test_deterministic() {
        let mobi1 = derive_from_hex(ALL_ZERO_PUBKEY).unwrap();
        let mobi2 = derive_from_hex(ALL_ZERO_PUBKEY).unwrap();

        assert_eq!(mobi1.full, mobi2.full);
        assert_eq!(mobi1.display, mobi2.display);
    }

    #[test]
    fn test_different_pubkeys() {
        let mobi1 = derive_from_hex(ALL_ZERO_PUBKEY).unwrap();
        let mobi2 = derive_from_hex(ABANDON_PUBKEY).unwrap();

        assert_ne!(mobi1.full, mobi2.full);
    }

    #[test]
    fn test_invalid_hex() {
        let result = derive_from_hex(
            "zzzz000000000000000000000000000000000000000000000000000000000000",
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_length() {
        let result = derive_from_hex("00000000");
        assert!(result.is_err());
    }

    #[test]
    fn test_normalize() {
        assert_eq!(normalize("879-044-656-584").unwrap(), "879044656584");
        assert_eq!(normalize("879 044 656 584").unwrap(), "879044656584");
        assert_eq!(normalize("(879) 044-656-584").unwrap(), "879044656584");
        assert_eq!(normalize("879044656584").unwrap(), "879044656584");
    }

    #[test]
    fn test_validate() {
        assert!(validate("879044656584")); // 12 digits
        assert!(validate("879044656584686")); // 15 digits
        assert!(validate("879044656584686196")); // 18 digits
        assert!(validate("879044656584686196443")); // 21 digits

        assert!(!validate("87904465658")); // 11 digits
        assert!(!validate("879-044-656-584")); // formatted
        assert!(!validate("87904465658a")); // with letter
    }

    #[test]
    fn test_display_matches_fn() {
        assert!(display_matches("879044656584", "879044656584"));
        assert!(display_matches("879044656584686", "879044656584000"));
        assert!(display_matches(
            "879044656584686196443",
            "879044656584999999999"
        ));
        assert!(!display_matches("879044656584", "879044656585"));
    }

    #[test]
    fn test_legacy_api() {
        let mobi = derive_mobinumber(ABANDON_PUBKEY);
        assert_eq!(mobi, "879-044-656-584");

        let canonical = derive_mobinumber_canonical(ABANDON_PUBKEY);
        assert_eq!(canonical, "879044656584");
    }

    #[test]
    fn test_prefix_consistency() {
        let mobi = derive_from_hex(ALL_ZERO_PUBKEY).unwrap();

        // Display is prefix of extended, extended is prefix of lng, lng is prefix of full
        assert!(mobi.extended.starts_with(&mobi.display));
        assert!(mobi.lng.starts_with(&mobi.extended));
        assert!(mobi.full.starts_with(&mobi.lng));
    }

    #[test]
    fn test_lengths() {
        let mobi = derive_from_hex(ALL_ZERO_PUBKEY).unwrap();

        assert_eq!(mobi.full.len(), 21);
        assert_eq!(mobi.display.len(), 12);
        assert_eq!(mobi.extended.len(), 15);
        assert_eq!(mobi.lng.len(), 18);
    }
}
