# Tabula — Charta Tertii Millennii

An open, deterministic knowledge base on a PoW blockchain. Anyone can store, query, and verify structured knowledge (foods, countries, materials, physics constants, etc.) with full Merkle-provable state.

## Repo Layout

| Directory | What | Stack |
|---|---|---|
| `eksperimentering/tabula-kb/` | **Blockchain node** — the core engine | Rust workspace, libp2p, sled, axum |
| `core/` | **Desktop app** — Tauri GUI client | Tauri v2 + React + Vite + TypeScript |
| `app/` | Mobile app scaffold | Kotlin Multiplatform (Compose) |
| `forskning/` | Research notes | — |

The blockchain node (`tabula-kb`) is the source of truth. The desktop app (`core/`) talks to it via HTTP API.

## Build & Test

### Blockchain node
```bash
cd eksperimentering/tabula-kb

# Build
cargo build --release -p node

# Run tests
cargo test --workspace

# Start single node (genesis + mining + KB data)
cargo run -p node --release -- --data-dir ./data --genesis --seed-genesis --mine --api-port 8080

# Query KB
cargo run -p node --release -- --data-dir ./data query 'food:"Salad".EnergyPerServing'
```

### Local 3-node network
```bash
# From repo root:
./test-local-network.sh              # Start fresh 3-node network
./test-local-network.sh --status     # Check running nodes
./test-local-network.sh --stop       # Stop all nodes

# Or from tabula-kb:
cd eksperimentering/tabula-kb
./scripts/run-local-network.sh --clean
```

### Desktop app
```bash
cd core
npm install
npm run tauri:dev     # Dev mode with hot reload
npm run build         # Build frontend only
npx tsc --noEmit      # Type check
```

## Architecture (tabula-kb)

See `eksperimentering/tabula-kb/CLAUDE.md` for full crate-by-crate docs.

Key crates: `node` (CLI + HTTP API), `chain` (state), `consensus` (PoW), `network` (libp2p), `vm` (deterministic execution), `contracts` (KB system contract), `entity`/`units`/`eval` (knowledge base logic), `persistence` (sled storage).

## Code Conventions

- **No floating point** in consensus/KB code — use `FixI128` (i128, 32 fractional bits)
- **No serde for consensus types** — use canonical codec for determinism
- **Determinism** — no system clock, no randomness, no IO in contracts
- **Merkle roots** — SHA-256 over sorted `len(k)||k||len(v)||v`
- **Error handling** — `Result` + `thiserror`, avoid `unwrap()` in production paths
- **Frontend** — glassmorphism/liquid glass design, 160ms ease transitions

## Current Status

- **Milestone 1 (Single-node blockchain)**: COMPLETE
- **Milestone 2 (Multi-node networking)**: Partially done — libp2p, gossipsub, mDNS, block propagation work. Still needs full sync protocol and fork resolution.
- **Desktop app**: Has blockchain/mining/network views. Not yet connected to the real `tabula-kb` node (uses in-process mini-chain). KB tab stubbed but not in nav.

## Verification

After making changes, always verify:
```bash
# Rust
cd eksperimentering/tabula-kb && cargo test --workspace && cargo clippy --workspace

# TypeScript
cd core && npx tsc --noEmit
```
