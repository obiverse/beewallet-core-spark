# BeeWallet Tauri - Spark Wallet Test UI

Minimal Tauri app for testing `beewallet-core-spark` with the Spark Bitcoin L2.

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│                    Tauri Frontend (HTML/JS)              │
│  ┌────────────┐  ┌────────────┐  ┌────────────────────┐  │
│  │  Connect   │  │  Balance   │  │  Send/Receive      │  │
│  └─────┬──────┘  └─────┬──────┘  └─────────┬──────────┘  │
└────────┼───────────────┼───────────────────┼─────────────┘
         │               │                   │
         ▼               ▼                   ▼
┌──────────────────────────────────────────────────────────┐
│                    Tauri IPC Commands                     │
│  connect, disconnect, read, write, list                   │
└────────────────────────────────────────────────────────────┘
         │               │                   │
         ▼               ▼                   ▼
┌──────────────────────────────────────────────────────────┐
│                beewallet-core-spark                       │
│  ┌────────────────┐  ┌────────────────┐                   │
│  │  WalletManager │  │  SparkSdkWrapper │                 │
│  │  (Namespace)   │  │  (Breez SDK)   │                   │
│  └────────────────┘  └────────────────┘                   │
└──────────────────────────────────────────────────────────┘
         │
         ▼
┌──────────────────────────────────────────────────────────┐
│                    Spark Network                          │
│               (Regtest / Testnet / Mainnet)               │
└──────────────────────────────────────────────────────────┘
```

## 9S Protocol

Everything is a path, everything is a Scroll.

| Operation | Path | Description |
|-----------|------|-------------|
| Read balance | `read("/balance")` | Get wallet balance in sats |
| Read address | `read("/address")` | Get Spark address |
| Read payments | `read("/payments")` | List recent payments |
| Send payment | `write("/send", {destination, amount_sat?})` | Send payment |
| Create invoice | `write("/invoice", {amount_sat, description?})` | Create invoice |
| Sync | `write("/sync", {})` | Sync wallet state |

## Development

### Prerequisites

- Rust (stable)
- Node.js (for Tauri CLI if not using cargo-tauri)

### Run Development Build

```bash
cd beewallet-tauri

# Check compilation
cargo check

# Run the app (requires Tauri CLI)
cargo tauri dev
```

### Build Release

```bash
cargo tauri build
```

## Testing with Regtest Faucet

1. Generate a new mnemonic (or use existing)
2. Connect with the mnemonic
3. Copy your Spark address
4. Go to https://app.lightspark.com/regtest-faucet
5. Paste your address and request regtest sats
6. Refresh to see your balance

## Commands

### System
- `system_info()` - Get app info
- `is_connected()` - Check connection status

### Wallet Lifecycle
- `connect(mnemonic, passphrase?, network?)` - Connect to Spark
- `disconnect()` - Disconnect from Spark
- `generate_mnemonic(word_count?)` - Generate new mnemonic
- `validate_mnemonic(phrase)` - Validate mnemonic

### 9S Protocol
- `read(path)` - Read from path
- `write(path, data)` - Write to path
- `list(prefix)` - List paths under prefix

### Convenience
- `get_balance()` - Get balance in sats
- `get_address()` - Get Spark address
- `create_invoice(amount_sat, description?)` - Create invoice
- `send_payment(destination, amount_sat?)` - Send payment
- `list_payments(limit?)` - List recent payments
- `sync_wallet()` - Sync wallet state
