⏺ Great questions about the architecture:

  1. Clients & P2P Discovery

  What clients can run this?
  - Any machine that can run the Rust binary (./target/release/node)
  - Currently: macOS, Linux (anywhere Rust compiles)
  - All nodes are equal - no master node

  How does discovery work?

  Two mechanisms:

  ┌─────────────────────────────────────────────────────────────┐
  │  mDNS (Multicast DNS) - Local Network Discovery             │
  ├─────────────────────────────────────────────────────────────┤
  │  • Nodes broadcast "I exist" on local network (224.0.0.251) │
  │  • Other nodes on same LAN automatically discover them      │
  │  • Zero configuration needed                                │
  │  • Only works within same network/subnet                    │
  └─────────────────────────────────────────────────────────────┘

  ┌─────────────────────────────────────────────────────────────┐
  │  Bootstrap Peers - Cross-Network Discovery                  │
  ├─────────────────────────────────────────────────────────────┤
  │  • Manually specify known peer addresses:                   │
  │    --bootstrap /ip4/1.2.3.4/tcp/9001/p2p/<peer-id>         │
  │  • Connect to those peers, learn about other peers          │
  │  • Required for internet-wide networks                      │
  └─────────────────────────────────────────────────────────────┘

  No master node - it's fully decentralized:
  - Any node can create genesis (first block)
  - Any node can mine
  - Nodes sync to the longest valid chain
  - If a node goes down, others continue

  2. Data Storage

  What's stored on each client?

  Currently: Full node only - every node stores everything:

  ./data/
  ├── db/                    # sled database
  │   ├── blocks/           # All blocks (hash → block)
  │   ├── state/            # Full KV state snapshot
  │   ├── nonces/           # Account nonces
  │   └── meta/             # Chain tip, height

  Can a client decide what 'subpart' to store?

  Not currently implemented. Right now it's all-or-nothing.

  But this is a common blockchain feature that could be added:

  | Node Type           | Stores                         | Use Case           |
  |---------------------|--------------------------------|--------------------|
  | Full Node (current) | All blocks + full state        | Miners, validators |
  | Pruned Node         | Recent blocks + current state  | Save disk space    |
  | Light Client        | Block headers only             | Mobile wallets     |
  | Archive Node        | Everything + historical states | Block explorers    |

  To add selective storage, you'd need:
  1. State pruning - Only keep latest state, discard old
  2. Block pruning - Only keep last N blocks
  3. Sharding - Split state across nodes (complex)