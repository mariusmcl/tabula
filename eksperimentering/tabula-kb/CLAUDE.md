# Tabula KB - Deterministic PoW Blockchain

## Project Overview

Tabula KB is a deterministic PoW blockchain with an on-chain knowledge base. It's inspired by Wolfram-style knowledge bases, designed to run as a system contract on a blockchain with verifiable state roots.

**Status**: Milestone 1 complete - Single-node PoW blockchain with persistence
**Next**: Milestone 2 - Multi-node networking with libp2p

## Quick Start

```bash
# Build
cargo build --release -p node

# Create a new chain with seeded KB data and start mining
cargo run -p node --release -- --data-dir ./data --genesis --seed-genesis --mine

# Query the knowledge base
cargo run -p node --release -- --data-dir ./data query 'food:"Salad".EnergyPerServing'
cargo run -p node --release -- --data-dir ./data query 'country:"France".PopulationDensity'
cargo run -p node --release -- --data-dir ./data query 'material:"Water".MolarMass'

# Check chain status
cargo run -p node --release -- --data-dir ./data status
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                           node                                   │
│  (main entry point - blockchain node with mining + persistence) │
└─────────────────────────────────────────────────────────────────┘
                                │
        ┌───────────────────────┼───────────────────────┐
        │                       │                       │
        ▼                       ▼                       ▼
   ┌─────────┐           ┌───────────┐           ┌───────────┐
   │ network │           │   chain   │           │ consensus │
   │ (TODO)  │           │  (state)  │           │   (PoW)   │
   └─────────┘           └─────┬─────┘           └───────────┘
        │                      │                       │
        │              ┌───────┴───────┐               │
        │              ▼               ▼               │
        │         ┌────────┐     ┌─────────┐          │
        │         │mempool │     │   vm    │          │
        │         └────────┘     └────┬────┘          │
        │                             │               │
        │         ┌───────────────────┼───────────────┘
        │         │                   │
        ▼         ▼                   ▼
   ┌─────────────────┐         ┌───────────┐
   │   persistence   │         │ contracts │
   │     (sled)      │         └─────┬─────┘
   └─────────────────┘               │
                              ┌──────┴──────┐
                              ▼             ▼
                         ┌────────┐   ┌─────────┐
                         │  eval  │   │ ingest  │
                         └────────┘   └─────────┘
```

## Crates

| Crate | Purpose |
|-------|---------|
| `node` | CLI blockchain node with mining |
| `chain` | Block storage, chain tip, state management |
| `consensus` | PoW mining, difficulty adjustment, block validation |
| `mempool` | Pending transaction pool |
| `persistence` | sled-based disk storage |
| `types` | Block, Transaction, BlockHeader, SignedTransaction |
| `crypto` | Ed25519 signing, SHA-256 hashing |
| `vm` | Deterministic VM with Contract trait |
| `contracts` | KbContract (knowledge base system contract) |
| `store` | BTreeMap KV store with SHA-256 Merkle root |
| `entity` | Entity types (Food, Country, etc.), Values, canonical codec |
| `units` | Fixed-point arithmetic (`FixI128`), SI dimensions |
| `eval` | Computed property evaluation |
| `query` | Query parsing (`food:"Salad".EnergyPerServing`) |
| `ingest` | Demo data seeding |

## Node CLI

```bash
# Show help
cargo run -p node -- --help

# Options:
#   -d, --data-dir <PATH>   Data directory [default: ./data]
#   -m, --mine              Enable mining
#   --genesis               Create genesis block
#   --seed-genesis          Seed KB data at genesis
#   --key-seed <SEED>       Deterministic key from seed

# Commands:
#   run       Run the node (default)
#   query     Query the knowledge base
#   submit    Submit a transaction
#   status    Show chain status
#   seed      Submit seed transaction to mempool
```

## Key Design Principles

1. **Determinism**: No floats, no randomness, no IO in contracts
2. **Fixed-point math**: `FixI128` (i128 with 32 fractional bits), truncate toward zero
3. **Canonical encoding**: Custom codec in `entity::codec`, NOT serde
4. **Merkle roots**: SHA-256 over sorted keys: `len(k)||k||len(v)||v`
5. **PoW consensus**: 5-second target block time, SHA-256 hash

## Development Commands

```bash
# Build everything
cargo build --workspace

# Run tests
cargo test --workspace

# Build release
cargo build --release --workspace

# Check compilation
cargo check --workspace

# Format code
cargo fmt --all

# Lint
cargo clippy --workspace
```

## Key Interfaces

### Contract Trait (crates/vm/src/lib.rs)
```rust
pub trait Contract {
    fn call(&mut self, state: &mut KV, method: u32, calldata: &[u8]) -> Vec<u8>;
}
```

### Block Header (crates/types/src/lib.rs)
```rust
pub struct BlockHeader {
    pub version: u32,
    pub height: u64,
    pub timestamp: u64,
    pub difficulty: u64,
    pub nonce: u64,
    pub parent_hash: Hash,
    pub state_root: Hash,
    pub tx_root: Hash,
}
```

### Signed Transaction (crates/types/src/lib.rs)
```rust
pub struct SignedTransaction {
    pub tx: Transaction,
    pub signature: [u8; 64],   // Ed25519
    pub public_key: [u8; 32],
}
```

## Development Progress

### Milestone 1: Single-Node Blockchain [COMPLETE]
- [x] CLAUDE.md
- [x] `crates/types` - Block, Transaction structs
- [x] `crates/crypto` - Ed25519 + SHA-256
- [x] `crates/consensus` - PoW mining (5s target block time)
- [x] `crates/chain` - State management
- [x] `crates/store` modifications - Clone, serialization
- [x] `crates/mempool` - Transaction pool
- [x] `crates/persistence` - sled storage
- [x] `crates/node` rewrite - Full blockchain node with CLI

### Milestone 2: Multi-Node Networking [TODO]
- [ ] `crates/network` - libp2p gossipsub + mDNS
- [ ] Block propagation
- [ ] Transaction propagation
- [ ] Block sync protocol
- [ ] Local multi-node testing scripts

## Code Conventions

- **No floating point**: Use `FixI128` for all numeric computation
- **No `serde` for consensus types**: Use canonical codec for determinism
- **Test determinism**: Same inputs must produce identical outputs and state roots
- **Error handling**: Use `Result` and `thiserror`, avoid `unwrap()` in production paths
