//! BeeWallet Tauri - Spark Wallet Desktop App
//!
//! ## Architecture: 9S Bus
//!
//! Everything flows through the 9S protocol:
//!
//! ```text
//! UI (TypeScript)
//!     │
//!     ├── invoke('nine_s', { op: 'read', path: '/wallet/balance' })
//!     ├── invoke('nine_s', { op: 'write', path: '/wallet/send', data: {...} })
//!     └── listen('nine_s://wallet/**') → events
//!     │
//!     ▼
//! Tauri (Rust)
//!     │
//!     └── nine_s command → dispatches to namespaces
//!         │
//!         ├── /system/* → System namespace (lifecycle, info)
//!         ├── /wallet/* → Wallet namespace (balance, send, receive)
//!         ├── /identity/* → Identity namespace (keys, mobinumber)
//!         └── /vault/* → Vault namespace (encrypted storage)
//! ```
//!
//! ## Path Ontology
//!
//! | Path | Op | Description |
//! |------|-----|-------------|
//! | `/system/info` | read | App name, version, network |
//! | `/system/status` | read | Connection status, wallet exists |
//! | `/system/connect` | write | Connect with mnemonic |
//! | `/system/disconnect` | write | Disconnect wallet |
//! | `/identity/mnemonic` | write | Generate new mnemonic |
//! | `/identity/validate` | write | Validate mnemonic phrase |
//! | `/identity/mobinumber` | write | Derive mobinumber from phrase |
//! | `/wallet/balance` | read | Get balance in sats |
//! | `/wallet/address` | read | Get Spark address |
//! | `/wallet/bitcoin-address` | read | Get Bitcoin address (for faucet) |
//! | `/wallet/send` | write | Send payment |
//! | `/wallet/invoice` | write | Create invoice |
//! | `/wallet/payments` | read | List payments |
//! | `/wallet/sync` | write | Sync with network |
//! | `/vault/status` | read | Vault initialization status |
//! | `/vault/init` | write | Initialize vault with PIN + mnemonic |
//! | `/vault/unlock` | write | Unlock vault with PIN |
//! | `/vault/lock` | write | Lock the vault |
//! | `/vault/reset` | write | Reset vault (DANGER) |

use beewallet_core_spark::{
    nine_s::{Namespace, Scroll},
    wallet_spark::{WalletConfig, WalletManager, SparkNetwork},
    vault::VaultStore,
    keys::MasterKey,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager, State};

// ============================================================================
// STATE
// ============================================================================

/// Application state - the root namespace
pub struct AppState {
    /// Wallet manager (None until connected)
    wallet: Mutex<Option<Arc<WalletManager>>>,
    /// Vault store for encrypted mnemonic
    vault: Mutex<Option<VaultStore>>,
    /// Vault key (32 bytes, None until unlocked)
    vault_key: Mutex<Option<[u8; 32]>>,
    /// Working directory for wallet data
    working_dir: String,
    /// Cached mnemonic for identity operations (cleared on disconnect)
    mnemonic: Mutex<Option<String>>,
}

impl AppState {
    fn new(working_dir: String) -> Self {
        // Open vault at working_dir/vault
        let vault_path = std::path::Path::new(&working_dir).join("vault");
        let vault = VaultStore::open(&vault_path).ok();

        Self {
            wallet: Mutex::new(None),
            vault: Mutex::new(vault),
            vault_key: Mutex::new(None),
            working_dir,
            mnemonic: Mutex::new(None),
        }
    }

    fn is_vault_initialized(&self) -> bool {
        self.vault.lock().unwrap()
            .as_ref()
            .and_then(|v| v.is_initialized().ok())
            .unwrap_or(false)
    }

    fn is_vault_unlocked(&self) -> bool {
        self.vault_key.lock().unwrap().is_some()
    }

    fn is_connected(&self) -> bool {
        self.wallet.lock().unwrap().is_some()
    }

    fn wallet_exists(&self) -> bool {
        let wallet_dir = std::path::Path::new(&self.working_dir);
        if !wallet_dir.exists() {
            return false;
        }
        // Spark SDK stores data in network subdirectories (regtest/, testnet/, mainnet/)
        // Check if any of them contain wallet data (storage.sql)
        for network in ["regtest", "testnet", "mainnet"] {
            let network_dir = wallet_dir.join(network);
            if network_dir.exists() && network_dir.is_dir() {
                // Check if there's any subdirectory with storage.sql
                if let Ok(entries) = std::fs::read_dir(&network_dir) {
                    for entry in entries.flatten() {
                        if entry.path().join("storage.sql").exists() {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}

// ============================================================================
// 9S BUS - SINGLE ENTRY POINT
// ============================================================================

/// 9S operation type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NineSOperation {
    Read,
    Write,
    List,
}

/// 9S request from UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NineSRequest {
    pub op: NineSOperation,
    pub path: String,
    #[serde(default)]
    pub data: Value,
}

/// 9S response to UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NineSResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scroll: Option<ScrollData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paths: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrollData {
    pub key: String,
    pub data: Value,
    #[serde(rename = "type")]
    pub type_: String,
}

impl From<Scroll> for ScrollData {
    fn from(s: Scroll) -> Self {
        Self {
            key: s.key,
            data: s.data,
            type_: s.type_,
        }
    }
}

impl NineSResponse {
    fn ok_scroll(scroll: Scroll) -> Self {
        Self {
            ok: true,
            scroll: Some(scroll.into()),
            paths: None,
            error: None,
        }
    }

    fn ok_paths(paths: Vec<String>) -> Self {
        Self {
            ok: true,
            scroll: None,
            paths: Some(paths),
            error: None,
        }
    }

    fn ok_none() -> Self {
        Self {
            ok: true,
            scroll: None,
            paths: None,
            error: None,
        }
    }

    fn err(msg: impl Into<String>) -> Self {
        Self {
            ok: false,
            scroll: None,
            paths: None,
            error: Some(msg.into()),
        }
    }
}

/// The single 9S bus command
///
/// All UI ↔ Backend communication goes through this.
#[tauri::command]
fn nine_s(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    request: NineSRequest,
) -> NineSResponse {
    let path = request.path.as_str();

    // Route by namespace prefix
    match request.op {
        NineSOperation::Read => dispatch_read(&state, path),
        NineSOperation::Write => dispatch_write(&state, &app, path, request.data),
        NineSOperation::List => dispatch_list(&state, path),
    }
}

// ============================================================================
// DISPATCH - Route to namespaces
// ============================================================================

fn dispatch_read(state: &State<'_, AppState>, path: &str) -> NineSResponse {
    // System namespace - no wallet required
    if path.starts_with("/system/") {
        return handle_system_read(state, path);
    }

    // Vault namespace - no wallet required
    if path.starts_with("/vault/") {
        return handle_vault_read(state, path);
    }

    // Wallet namespace - requires connection
    if path.starts_with("/wallet/") {
        return handle_wallet_read(state, path);
    }

    NineSResponse::err(format!("Unknown read path: {}", path))
}

fn dispatch_write(
    state: &State<'_, AppState>,
    app: &tauri::AppHandle,
    path: &str,
    data: Value,
) -> NineSResponse {
    // System namespace
    if path.starts_with("/system/") {
        return handle_system_write(state, app, path, data);
    }

    // Vault namespace
    if path.starts_with("/vault/") {
        return handle_vault_write(state, app, path, data);
    }

    // Identity namespace - no wallet required
    if path.starts_with("/identity/") {
        return handle_identity_write(state, path, data);
    }

    // Wallet namespace
    if path.starts_with("/wallet/") {
        return handle_wallet_write(state, path, data);
    }

    NineSResponse::err(format!("Unknown write path: {}", path))
}

fn dispatch_list(state: &State<'_, AppState>, prefix: &str) -> NineSResponse {
    let mut paths = vec![];

    // Always include system paths
    if prefix.is_empty() || prefix == "/" || prefix.starts_with("/system") {
        paths.extend([
            "/system/info".to_string(),
            "/system/status".to_string(),
        ]);
    }

    // Always include vault paths
    if prefix.is_empty() || prefix == "/" || prefix.starts_with("/vault") {
        paths.extend([
            "/vault/status".to_string(),
            "/vault/init".to_string(),
            "/vault/unlock".to_string(),
            "/vault/lock".to_string(),
        ]);
    }

    // Always include identity paths
    if prefix.is_empty() || prefix == "/" || prefix.starts_with("/identity") {
        paths.extend([
            "/identity/mnemonic".to_string(),
            "/identity/validate".to_string(),
            "/identity/mobinumber".to_string(),
        ]);
    }

    // Wallet paths only when connected
    if state.is_connected() {
        if prefix.is_empty() || prefix == "/" || prefix.starts_with("/wallet") {
            paths.extend([
                "/wallet/balance".to_string(),
                "/wallet/address".to_string(),
                "/wallet/bitcoin-address".to_string(),
                "/wallet/payments".to_string(),
            ]);
        }
    }

    NineSResponse::ok_paths(paths)
}

// ============================================================================
// SYSTEM NAMESPACE
// ============================================================================

fn handle_system_read(state: &State<'_, AppState>, path: &str) -> NineSResponse {
    match path {
        "/system/info" => {
            NineSResponse::ok_scroll(Scroll::typed(
                "/system/info",
                json!({
                    "name": "BeeWallet Spark",
                    "version": "0.1.0",
                    "backend": "beewallet-core-spark",
                    "network": "Spark (Regtest)",
                    "faucet": "https://app.lightspark.com/regtest-faucet"
                }),
                "system/info@v1",
            ))
        }
        "/system/status" => {
            NineSResponse::ok_scroll(Scroll::typed(
                "/system/status",
                json!({
                    "connected": state.is_connected(),
                    "wallet_exists": state.wallet_exists(),
                    "working_dir": state.working_dir,
                }),
                "system/status@v1",
            ))
        }
        _ => NineSResponse::err(format!("Unknown system path: {}", path)),
    }
}

fn handle_system_write(
    state: &State<'_, AppState>,
    app: &tauri::AppHandle,
    path: &str,
    data: Value,
) -> NineSResponse {
    match path {
        "/system/connect" => {
            let mnemonic = data.get("mnemonic")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let passphrase = data.get("passphrase")
                .and_then(|v| v.as_str());
            let network = data.get("network")
                .and_then(|v| v.as_str())
                .unwrap_or("regtest");

            // Parse network
            let spark_network = match network {
                "mainnet" => SparkNetwork::Mainnet,
                "testnet" => SparkNetwork::Testnet,
                _ => SparkNetwork::Regtest,
            };

            // Create wallet config
            let config = WalletConfig::new(spark_network)
                .with_working_dir(&state.working_dir);

            let wallet = WalletManager::new(spark_network, config.api_key.clone())
                .with_working_dir(config.working_dir.unwrap_or_default());

            // Connect synchronously (the connect method already handles blocking internally)
            let mnemonic_str = mnemonic.to_string();
            let passphrase_opt = passphrase.map(|s| s.to_string());

            match wallet.connect(&mnemonic_str, passphrase_opt.as_deref()) {
                Ok(()) => {
                    // Store wallet and mnemonic
                    *state.wallet.lock().unwrap() = Some(Arc::new(wallet));
                    *state.mnemonic.lock().unwrap() = Some(mnemonic.to_string());

                    // Emit connection event
                    let _ = app.emit("nine_s://system/connected", json!({
                        "network": format!("{:?}", spark_network)
                    }));

                    NineSResponse::ok_scroll(Scroll::typed(
                        "/system/connect",
                        json!({
                            "status": "connected",
                            "network": format!("{:?}", spark_network)
                        }),
                        "system/connect@v1",
                    ))
                }
                Err(e) => NineSResponse::err(e.to_string()),
            }
        }
        "/system/disconnect" => {
            let mut guard = state.wallet.lock().unwrap();
            if let Some(wallet) = guard.take() {
                if let Err(e) = wallet.disconnect() {
                    return NineSResponse::err(e.to_string());
                }
            }
            // Clear mnemonic
            *state.mnemonic.lock().unwrap() = None;

            // Emit disconnection event
            let _ = app.emit("nine_s://system/disconnected", json!({}));

            NineSResponse::ok_scroll(Scroll::typed(
                "/system/disconnect",
                json!({ "status": "disconnected" }),
                "system/disconnect@v1",
            ))
        }
        _ => NineSResponse::err(format!("Unknown system write path: {}", path)),
    }
}

// ============================================================================
// IDENTITY NAMESPACE
// ============================================================================

#[allow(unused_variables)]
fn handle_identity_write(state: &State<'_, AppState>, path: &str, data: Value) -> NineSResponse {
    match path {
        "/identity/mnemonic" => {
            let word_count = data.get("word_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(12) as usize;

            match MasterKey::generate(word_count) {
                Ok(key) => NineSResponse::ok_scroll(Scroll::typed(
                    "/identity/mnemonic",
                    json!({
                        "phrase": key.mnemonic_phrase().as_str(),
                        "word_count": word_count,
                    }),
                    "identity/mnemonic@v1",
                )),
                Err(e) => NineSResponse::err(e.to_string()),
            }
        }
        "/identity/validate" => {
            let phrase = data.get("phrase")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let valid = MasterKey::from_mnemonic(phrase).is_ok();

            NineSResponse::ok_scroll(Scroll::typed(
                "/identity/validate",
                json!({ "valid": valid }),
                "identity/validate@v1",
            ))
        }
        "/identity/mobinumber" => {
            let phrase = data.get("phrase")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            match MasterKey::from_mnemonic(phrase) {
                Ok(key) => match key.mobinumber() {
                    Ok(mobi) => NineSResponse::ok_scroll(Scroll::typed(
                        "/identity/mobinumber",
                        json!({ "mobinumber": mobi }),
                        "identity/mobinumber@v1",
                    )),
                    Err(e) => NineSResponse::err(e.to_string()),
                },
                Err(e) => NineSResponse::err(e.to_string()),
            }
        }
        _ => NineSResponse::err(format!("Unknown identity path: {}", path)),
    }
}

// ============================================================================
// VAULT NAMESPACE
// ============================================================================

fn handle_vault_read(state: &State<'_, AppState>, path: &str) -> NineSResponse {
    match path {
        "/vault/status" => {
            let initialized = state.is_vault_initialized();
            let unlocked = state.is_vault_unlocked();
            let lockout_remaining = state.vault.lock().unwrap()
                .as_ref()
                .map(|v| v.lockout_remaining())
                .unwrap_or(0);

            NineSResponse::ok_scroll(Scroll::typed(
                "/vault/status",
                json!({
                    "initialized": initialized,
                    "unlocked": unlocked,
                    "lockout_remaining": lockout_remaining,
                }),
                "vault/status@v1",
            ))
        }
        _ => NineSResponse::err(format!("Unknown vault read path: {}", path)),
    }
}

fn handle_vault_write(
    state: &State<'_, AppState>,
    app: &tauri::AppHandle,
    path: &str,
    data: Value,
) -> NineSResponse {
    match path {
        "/vault/init" => {
            // Initialize vault with PIN + mnemonic
            let pin = data.get("pin")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let mnemonic = data.get("mnemonic")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if pin.is_empty() || mnemonic.is_empty() {
                return NineSResponse::err("PIN and mnemonic are required");
            }

            let vault_guard = state.vault.lock().unwrap();
            let vault = match vault_guard.as_ref() {
                Some(v) => v,
                None => return NineSResponse::err("Vault not available"),
            };

            match vault.initialize(pin, mnemonic) {
                Ok(vault_key) => {
                    // Store the vault key
                    *state.vault_key.lock().unwrap() = Some(vault_key);
                    // Store mnemonic for wallet connection
                    *state.mnemonic.lock().unwrap() = Some(mnemonic.to_string());

                    let _ = app.emit("nine_s://vault/initialized", json!({}));

                    NineSResponse::ok_scroll(Scroll::typed(
                        "/vault/init",
                        json!({ "status": "initialized" }),
                        "vault/init@v1",
                    ))
                }
                Err(e) => NineSResponse::err(e.to_string()),
            }
        }
        "/vault/unlock" => {
            // Unlock vault with PIN
            let pin = data.get("pin")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if pin.is_empty() {
                return NineSResponse::err("PIN is required");
            }

            let vault_guard = state.vault.lock().unwrap();
            let vault = match vault_guard.as_ref() {
                Some(v) => v,
                None => return NineSResponse::err("Vault not available"),
            };

            match vault.unlock(pin) {
                Ok(vault_key) => {
                    // Get the seed phrase
                    match vault.get_seed(&vault_key) {
                        Ok(seed) => {
                            // Store vault key and mnemonic
                            *state.vault_key.lock().unwrap() = Some(vault_key);
                            *state.mnemonic.lock().unwrap() = Some(seed.as_str().to_string());

                            let _ = app.emit("nine_s://vault/unlocked", json!({}));

                            NineSResponse::ok_scroll(Scroll::typed(
                                "/vault/unlock",
                                json!({ "status": "unlocked" }),
                                "vault/unlock@v1",
                            ))
                        }
                        Err(e) => NineSResponse::err(e.to_string()),
                    }
                }
                Err(e) => NineSResponse::err(e.to_string()),
            }
        }
        "/vault/lock" => {
            // Clear vault key and mnemonic
            *state.vault_key.lock().unwrap() = None;
            *state.mnemonic.lock().unwrap() = None;

            // Disconnect wallet
            if let Some(wallet) = state.wallet.lock().unwrap().take() {
                let _ = wallet.disconnect();
            }

            let _ = app.emit("nine_s://vault/locked", json!({}));

            NineSResponse::ok_scroll(Scroll::typed(
                "/vault/lock",
                json!({ "status": "locked" }),
                "vault/lock@v1",
            ))
        }
        "/vault/reset" => {
            // DANGER: Reset vault (destroys encrypted seed)
            let vault_guard = state.vault.lock().unwrap();
            if let Some(vault) = vault_guard.as_ref() {
                if let Err(e) = vault.reset() {
                    return NineSResponse::err(e.to_string());
                }
            }

            // Clear all state
            drop(vault_guard);
            *state.vault_key.lock().unwrap() = None;
            *state.mnemonic.lock().unwrap() = None;
            if let Some(wallet) = state.wallet.lock().unwrap().take() {
                let _ = wallet.disconnect();
            }

            let _ = app.emit("nine_s://vault/reset", json!({}));

            NineSResponse::ok_scroll(Scroll::typed(
                "/vault/reset",
                json!({ "status": "reset" }),
                "vault/reset@v1",
            ))
        }
        "/vault/auto-connect" => {
            // Auto-connect wallet if vault is unlocked and we have mnemonic
            let mnemonic = state.mnemonic.lock().unwrap().clone();
            let mnemonic = match mnemonic {
                Some(m) => m,
                None => return NineSResponse::err("Vault not unlocked"),
            };

            let network = data.get("network")
                .and_then(|v| v.as_str())
                .unwrap_or("regtest");

            let spark_network = match network {
                "mainnet" => SparkNetwork::Mainnet,
                "testnet" => SparkNetwork::Testnet,
                _ => SparkNetwork::Regtest,
            };

            let config = WalletConfig::new(spark_network)
                .with_working_dir(&state.working_dir);

            let wallet = WalletManager::new(spark_network, config.api_key.clone())
                .with_working_dir(config.working_dir.unwrap_or_default());

            match wallet.connect(&mnemonic, None) {
                Ok(()) => {
                    *state.wallet.lock().unwrap() = Some(Arc::new(wallet));

                    let _ = app.emit("nine_s://system/connected", json!({
                        "network": format!("{:?}", spark_network)
                    }));

                    NineSResponse::ok_scroll(Scroll::typed(
                        "/vault/auto-connect",
                        json!({
                            "status": "connected",
                            "network": format!("{:?}", spark_network)
                        }),
                        "vault/auto-connect@v1",
                    ))
                }
                Err(e) => NineSResponse::err(e.to_string()),
            }
        }
        _ => NineSResponse::err(format!("Unknown vault write path: {}", path)),
    }
}

// ============================================================================
// WALLET NAMESPACE
// ============================================================================

fn require_wallet(state: &State<'_, AppState>) -> Result<Arc<WalletManager>, NineSResponse> {
    state.wallet.lock().unwrap()
        .clone()
        .ok_or_else(|| NineSResponse::err("Wallet not connected"))
}

fn handle_wallet_read(state: &State<'_, AppState>, path: &str) -> NineSResponse {
    let wallet = match require_wallet(state) {
        Ok(w) => w,
        Err(e) => return e,
    };

    // Strip /wallet prefix for WalletManager
    let wallet_path = path.strip_prefix("/wallet").unwrap_or(path);

    match wallet.read(wallet_path) {
        Ok(Some(scroll)) => NineSResponse::ok_scroll(scroll),
        Ok(None) => NineSResponse::ok_none(),
        Err(e) => NineSResponse::err(e.to_string()),
    }
}

fn handle_wallet_write(state: &State<'_, AppState>, path: &str, data: Value) -> NineSResponse {
    let wallet = match require_wallet(state) {
        Ok(w) => w,
        Err(e) => return e,
    };

    // Strip /wallet prefix for WalletManager
    let wallet_path = path.strip_prefix("/wallet").unwrap_or(path);

    match wallet.write(wallet_path, data) {
        Ok(scroll) => NineSResponse::ok_scroll(scroll),
        Err(e) => NineSResponse::err(e.to_string()),
    }
}

// ============================================================================
// APP ENTRY
// ============================================================================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Get app data directory for wallet storage
            let app_data_dir = app.path().app_data_dir()
                .expect("Failed to get app data directory");

            let working_dir = app_data_dir.join("spark-wallet");
            std::fs::create_dir_all(&working_dir).ok();

            let working_dir_str = working_dir.to_string_lossy().to_string();
            app.manage(AppState::new(working_dir_str));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![nine_s])
        .run(tauri::generate_context!())
        .expect("error while running BeeWallet Tauri");
}
