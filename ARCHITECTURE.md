# Architecture

## Core Design Principles

### 1. Five Frozen Operations (9S Protocol)

```rust
pub trait Namespace: Send + Sync {
    fn read(&self, path: &str) -> Result<Option<Scroll>>;
    fn write(&self, path: &str, data: Value) -> Result<Scroll>;
    fn list(&self, prefix: &str) -> Result<Vec<String>>;
    fn watch(&self, pattern: &str) -> Result<Receiver<Scroll>>;
    fn close(&self) -> Result<()>;
}
```

**Never a sixth operation.** Every domain problem is solved through composition of these five.

### 2. Layer Separation

```
┌─────────────────────────────────────────────────────────────┐
│                    Application Layer                         │
│              (Flutter, Tauri, CLI, Tests)                   │
├─────────────────────────────────────────────────────────────┤
│                    9S Kernel (Mount Table)                   │
│         Scroll.invoke("/path", data) → Namespace.op()       │
├──────────────┬──────────────┬──────────────┬────────────────┤
│   /wallet/*  │   /vault/*   │  /identity/* │   /nostr/*     │
│  SparkBackend│  VaultStore  │  MasterKey   │  NostrSigner   │
├──────────────┴──────────────┴──────────────┴────────────────┤
│                    Cryptographic Primitives                  │
│           Argon2id, AES-256-GCM, HKDF, BIP39                │
└─────────────────────────────────────────────────────────────┘
```

### 3. Scroll: Universal Data Envelope

```rust
pub struct Scroll {
    pub key: String,          // Path (ontology encoded)
    pub type_: String,        // Schema: "domain/entity@version"
    pub metadata: Metadata,   // Rich semantic layer
    pub data: Value,          // Opaque payload
}
```

Every piece of data in the system is a Scroll. No exceptions.

## Module Structure

```
src/
├── lib.rs              # Public API surface
├── nine_s/             # 9S Protocol (always available)
│   ├── scroll.rs       # Universal data envelope
│   ├── namespace.rs    # 5-operation trait
│   ├── kernel.rs       # Mount table composition
│   ├── store.rs        # Encrypted reactor
│   ├── patch.rs        # Git-like diffs
│   ├── anchor.rs       # Immutable checkpoints
│   ├── sealed.rs       # Encrypted sharing
│   └── backends/       # Namespace implementations
│       ├── memory.rs   # In-memory (testing)
│       └── file.rs     # JSON file persistence
├── vault/              # Encryption layer (crypto feature)
│   ├── crypto.rs       # AES-256-GCM, Argon2id
│   ├── session.rs      # Rate limiting, timeout
│   └── store.rs        # VaultStore
├── keys/               # Key derivation (keys feature)
│   └── mod.rs          # MasterKey, BIP39, Nostr
├── wallet_trait.rs     # WalletBackend interface
└── wallet_spark/       # Spark implementation (wallet feature)
    ├── mod.rs          # WalletManager
    └── signing.rs      # Message signing
```

## Feature Flags

```
default
   └── std-channel (watch channels via std::sync)

crypto
   └── AES-256-GCM, Argon2id, HKDF, zeroize

keys
   └── crypto + BIP39 + Nostr (NIP-06, NIP-44)

wallet
   └── keys + breez-sdk-spark + tokio
```

## WalletBackend Trait

Interface abstraction for pluggable backends:

```rust
pub trait WalletBackend: Send + Sync {
    fn connect(&self, mnemonic: &str, passphrase: Option<&str>) -> Result<()>;
    fn disconnect(&self) -> Result<()>;
    fn balance(&self) -> Result<WalletBalance>;
    fn new_address(&self) -> Result<String>;
    fn send(&self, dest: &str, amount: u64, fee: Option<f64>) -> Result<String>;
    fn transactions(&self, limit: usize) -> Result<Vec<TransactionDetails>>;
    fn create_invoice(&self, amount: u64, desc: Option<&str>) -> Result<String>;
    fn sign_message(&self, message: &str) -> Result<SignedMessage>;
    fn backend_name(&self) -> &'static str;  // "spark"
}
```

## Spark SDK Integration

```rust
// Breez SDK Spark API mapping:
BreezSdk::connect()           → WalletManager::connect()
BreezSdk::get_info()          → balance()
BreezSdk::receive_payment()   → new_address(), create_invoice()
BreezSdk::prepare_send()      → estimate_fee()
BreezSdk::send_payment()      → send()
BreezSdk::list_payments()     → transactions()
BreezSdk::add_event_listener()→ event streaming
```

## Security Model

### Encryption at Rest
- All vault data encrypted with AES-256-GCM
- Key derivation: Argon2id (64 MiB, 3 iterations)
- App isolation: HKDF derives unique keys per app

### Key Hierarchy
```
BIP39 Mnemonic (12/24 words)
    │
    ├── Argon2id + salt → Master Key (32 bytes)
    │       │
    │       ├── HKDF("beewallet") → App Key (isolated)
    │       └── HKDF("vault") → Vault Key (isolated)
    │
    ├── BIP84 (m/84'/0'/0') → Bitcoin addresses
    ├── NIP-06 (m/44'/1237'/0'/0/0) → Nostr identity
    └── Raw entropy → Lightning (Spark)
```

### Session Management
- Rate limiting: 3 attempts, exponential backoff
- Session timeout: 5 minutes default
- Zeroization: All secrets cleared on drop

## Integration with BeeWallet

### Interface Parity

Must export identical API to beewallet-core-breez:

```rust
// Network
pub type LiquidNetwork = SparkNetwork;  // Alias for compat

// Config
pub struct WalletConfig { ... }
impl WalletConfig {
    pub fn new(network: SparkNetwork) -> Self;
    pub fn with_api_key(self, key: String) -> Self;
    pub fn with_working_dir(self, dir: PathBuf) -> Self;
}

// Manager
pub struct WalletManager { ... }
impl WalletManager {
    pub async fn connect(config, mnemonic, passphrase) -> Result<Self>;
    pub async fn balance(&self) -> Result<WalletBalance>;
    // ... all other methods
}

// Namespace (orchestrated mode)
pub struct WalletNamespace { ... }
impl WalletNamespace {
    pub fn with_wallet(
        wallet: Arc<RwLock<Option<WalletManager>>>,
        mnemonic: Arc<RwLock<Option<String>>>,
        // ...
    ) -> Self;
}
impl Namespace for WalletNamespace { ... }

// Events
pub trait EventListener: Send + Sync {
    async fn on_event(&self, event: SdkEvent);
}

pub enum SdkEvent {
    Synced,
    PaymentSucceeded { payment },
    PaymentPending { payment },
    PaymentFailed { payment },
    // ...
}
```

### Swap Test

To swap backends in megab:

```toml
# Cargo.toml
# Current (Liquid):
beewallet-core = { path = "../beewallet-core-breez" }

# Swap to Spark:
beewallet-core = { path = "../beewallet-core-spark", package = "beewallet-core-spark" }
```

Should compile with minimal changes (network enum, config keys).

## Testing Strategy

### Unit Tests
- Each module has unit tests
- Crypto verification (seal/unseal roundtrip)
- Key derivation vectors (BIP39, NIP-06)

### Integration Tests
- Regtest with faucet (free, unlimited)
- Full payment flow testing
- Event handling verification

### Property Tests
- Proptest for scroll serialization
- Path validation edge cases

## Future: beewallet-tauri

```
beewallet-tauri/
├── src-tauri/          # Rust backend (this crate)
│   └── Cargo.toml      # depends on beewallet-core-spark
├── src/                # Web frontend
│   └── App.tsx
└── tauri.conf.json
```

Use this core to build the "platonic form" of BeeWallet in Tauri, then port learnings back to Flutter.
