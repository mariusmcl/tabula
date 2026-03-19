#!/bin/bash
# Run a local 3-node Tabula blockchain network for testing.
# Uses mDNS for peer discovery (all nodes find each other automatically).
#
# Usage:
#   ./scripts/run-local-network.sh          # start (resumes existing chain)
#   ./scripts/run-local-network.sh --clean  # wipe data and start fresh
#
# Nodes:
#   node1 — genesis + mining  | P2P :9001 | HTTP API :8001
#   node2 — sync only         | P2P :9002 | HTTP API :8002
#   node3 — mining            | P2P :9003 | HTTP API :8003
#
# All nodes discover each other via mDNS and sync the blockchain.

set -e

# Configuration
PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DATA_DIR="${PROJECT_ROOT}/local-testnet"
BINARY="${PROJECT_ROOT}/target/release/node"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Tabula Local Network ===${NC}"

# Build in release mode
echo -e "${YELLOW}Building node in release mode...${NC}"
cd "$PROJECT_ROOT"
cargo build --release -p node

# Clean up old data (optional)
if [ "$1" == "--clean" ]; then
    echo -e "${YELLOW}Cleaning old testnet data...${NC}"
    rm -rf "${DATA_DIR}"
fi

# Create data directories
mkdir -p "${DATA_DIR}/node1"
mkdir -p "${DATA_DIR}/node2"
mkdir -p "${DATA_DIR}/node3"

# Check if genesis already exists for node1 (sled creates a directory, not a file)
NEEDS_GENESIS=""
if [ ! -d "${DATA_DIR}/node1/db" ]; then
    NEEDS_GENESIS="--genesis --seed-genesis"
fi

# Trap Ctrl+C to clean up all child processes
cleanup() {
    echo ""
    echo -e "${RED}Stopping all nodes...${NC}"
    kill $NODE1_PID $NODE2_PID $NODE3_PID 2>/dev/null || true
    wait 2>/dev/null
    echo "Done."
    exit 0
}
trap cleanup SIGINT SIGTERM

# Start nodes
echo -e "${GREEN}Starting 3-node local network...${NC}"
echo ""

# Node 1: Genesis node with mining + API
echo -e "${BLUE}[Node 1]${NC} Genesis + Mining | P2P :9001 | API :8001"
$BINARY \
    --data-dir "${DATA_DIR}/node1" \
    --port 9001 \
    --api-port 8001 \
    --mine \
    --key-seed "node1" \
    $NEEDS_GENESIS \
    2>&1 | sed 's/^/[node1] /' &
NODE1_PID=$!

# Give node1 time to create genesis and start listening
sleep 3

# Node 2: Sync only + API
echo -e "${BLUE}[Node 2]${NC} Sync only | P2P :9002 | API :8002"
$BINARY \
    --data-dir "${DATA_DIR}/node2" \
    --port 9002 \
    --api-port 8002 \
    --key-seed "node2" \
    2>&1 | sed 's/^/[node2] /' &
NODE2_PID=$!

# Node 3: Mining + API
echo -e "${BLUE}[Node 3]${NC} Mining | P2P :9003 | API :8003"
$BINARY \
    --data-dir "${DATA_DIR}/node3" \
    --port 9003 \
    --api-port 8003 \
    --mine \
    --key-seed "node3" \
    2>&1 | sed 's/^/[node3] /' &
NODE3_PID=$!

echo ""
echo -e "${GREEN}All nodes started!${NC}"
echo ""
echo "PIDs: node1=$NODE1_PID, node2=$NODE2_PID, node3=$NODE3_PID"
echo ""
echo "HTTP APIs:"
echo "  Node 1: http://localhost:8001/api/status"
echo "  Node 2: http://localhost:8002/api/status"
echo "  Node 3: http://localhost:8003/api/status"
echo ""
echo "Try these:"
echo "  curl http://localhost:8001/api/status"
echo "  curl -X POST http://localhost:8001/api/query -H 'Content-Type: application/json' -d '{\"query\": \"food:\\\"Salad\\\".EnergyPerServing\"}'"
echo "  curl -X POST http://localhost:8001/api/put -H 'Content-Type: application/json' -d '{\"entity_type\": \"food\", \"key\": \"Pizza\", \"property\": \"Calories\", \"value\": \"285\"}'"
echo ""
echo -e "${YELLOW}Press Ctrl+C to stop all nodes${NC}"

# Wait for all nodes
wait
