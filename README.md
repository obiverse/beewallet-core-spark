# beewallet-core-spark

Sovereign Bitcoin + Lightning wallet core using Breez SDK Spark.

## Vision

Build the **platonic form** of a Bitcoin wallet backend:
- **Simple**: 5 frozen operations (9S Protocol)
- **Deterministic**: Same input = same output, always
- **Optimal**: Minimal abstraction, maximal sovereignty

This core powers [BeeWallet](https://github.com/obiverse/beewallet) and future `beewallet-tauri` implementations.

## Philosophy

### Dialectics
- **Thesis**: Breez Liquid (production, federation-wrapped)
- **Antithesis**: Pure on-chain (sovereign, slow)
- **Synthesis**: Spark (native BTC, statechains, instant, sovereign)

### Plan 9 / 9S Protocol
Everything is a path. Everything is a Scroll.

```
Flutter/Tauri UI  -->  Scroll.invoke("/wallet/send", data)
                             |
                             v
                      9S Kernel (mount table)
                             |
            +----------------+----------------+
            |                |                |
       /wallet/*        /vault/*        /identity/*
      SparkBackend     VaultStore      NostrSigner
```

### SICP Principles
- Programs for humans, incidentally for machines
- Abstraction barriers between layers
- Interface over implementation
- Test at boundaries, not internals

## Why Spark over Liquid?

| Aspect | Spark | Liquid |
|--------|-------|--------|
| Architecture | Statechains (native BTC) | Sidechain (L-BTC wrapping) |
| Trust Model | 1-of-n operators | 15 federation members |
| Exit | Unilateral, permissionless | Federation-controlled |
| Minimum Amount | None (zaps work!) | Federation overhead |
| SQLite | Compatible | Conflicts with BDK |
| Maturity | Experimental (2024) | Production (2018) |

## Quick Start

```bash
# Clone
git clone https://github.com/obiverse/beewallet-core-spark
cd beewallet-core-spark

# Build (minimal - 9S only)
cargo build

# Build with wallet
cargo build --features wallet

# Test
cargo test --all-features
```

## Features

| Feature | What it enables |
|---------|-----------------|
| `default` | 9S Protocol + std-channel |
| `crypto` | AES-256-GCM, Argon2id, HKDF |
| `keys` | BIP39, Nostr (NIP-06), Mobinumber |
| `wallet` | Full Spark SDK integration |

## Development with Faucet

Spark provides a **free regtest network** with faucet:

```rust
// No API key needed for regtest!
let config = DefaultConfig(Network::Regtest);
let sdk = BreezSdk::connect(ConnectRequest {
    config,
    mnemonic: seed_phrase,
    storage_dir: "./.data",
}).await?;

// Get test BTC: https://app.lightspark.com/regtest-faucet
```

## Roadmap

See [GitHub Issues](https://github.com/obiverse/beewallet-core-spark/issues) for detailed roadmap.

**Phase 1**: Interface parity with beewallet-core-breez
**Phase 2**: Wire Spark SDK, test on regtest with faucet
**Phase 3**: Megab swap test (change Cargo.toml, compile, run)
**Phase 4**: beewallet-tauri prototype
**Phase 5**: Production hardening

## Architecture

See [ARCHITECTURE.md](./ARCHITECTURE.md) for detailed design.

## Related Projects

- [beewallet](https://github.com/obiverse/beewallet) - Flutter wallet using this core
- [beewallet-core-breez](https://github.com/obiverse/beewallet-core-breez) - Liquid-based alternative
- [9S](https://github.com/obiverse/9s) - The 9S Protocol CLI

## License

MIT OR Apache-2.0
