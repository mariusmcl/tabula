#!/bin/bash
# =============================================================================
# Tabula Local Testing — Quick Start
# =============================================================================
#
# Usage:
#   ./test-local-network.sh              # Start single node (genesis + mining + API)
#   ./test-local-network.sh --multi      # Start 3-node network (peer discovery works,
#                                        #   block sync still WIP — nodes mine independently)
#   ./test-local-network.sh --demo       # Start single node + run demo queries
#   ./test-local-network.sh --status     # Check status of running node(s)
#   ./test-local-network.sh --stop       # Stop all nodes
#
# Single node:
#   API at http://localhost:8080
#
# Multi-node:
#   Node 1 — genesis + mining  | API http://localhost:8001
#   Node 2 — sync only         | API http://localhost:8002
#   Node 3 — mining            | API http://localhost:8003
#
# NOTE: Multi-node block sync is still in development (Milestone 2).
#       Nodes discover each other via mDNS but don't fully sync chains yet.
#       Use single-node mode for testing the KB, mining, and HTTP API.
# =============================================================================

set -e

KB_DIR="$(cd "$(dirname "$0")/eksperimentering/tabula-kb" && pwd)"
DATA_DIR="${KB_DIR}/local-testnet"
BINARY="${KB_DIR}/target/release/node"
PID_FILE="/tmp/tabula-testnet.pids"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# --- Helper functions ---

build_node() {
    echo -e "${YELLOW}Building node (release)...${NC}"
    cd "$KB_DIR"
    cargo build --release -p node 2>&1 | grep -E "(Compiling node|Finished|error)" || true
    if [ ! -f "$BINARY" ]; then
        echo -e "${RED}Build failed!${NC}"
        exit 1
    fi
    echo -e "${GREEN}Build OK${NC}"
}

stop_nodes() {
    echo -e "${RED}Stopping all Tabula nodes...${NC}"
    if [ -f "$PID_FILE" ]; then
        while read -r pid; do
            kill "$pid" 2>/dev/null || true
        done < "$PID_FILE"
        rm -f "$PID_FILE"
    fi
    pkill -f "tabula-kb/target/release/node" 2>/dev/null || true
    pkill -f "tabula-kb/local-testnet" 2>/dev/null || true
    sleep 1
    echo -e "${GREEN}Stopped.${NC}"
}

check_status() {
    for PORT in 8080 8001 8002 8003; do
        RESPONSE=$(curl -s --max-time 2 "http://localhost:$PORT/api/status" 2>/dev/null)
        if [ $? -eq 0 ] && [ -n "$RESPONSE" ]; then
            echo -e "${BLUE}--- localhost:$PORT ---${NC}"
            echo -e "${GREEN}ONLINE${NC}"
            echo "$RESPONSE" | python3 -m json.tool 2>/dev/null || echo "$RESPONSE"
            echo ""
        fi
    done
}

run_demo() {
    PORT=${1:-8080}
    echo ""
    echo -e "${CYAN}=== Tabula Demo (localhost:$PORT) ===${NC}"
    echo ""

    echo -e "${BLUE}1. Chain status:${NC}"
    curl -s "http://localhost:$PORT/api/status" | python3 -m json.tool
    echo ""

    echo -e "${BLUE}2. KB query — Salad energy per serving:${NC}"
    curl -s -X POST "http://localhost:$PORT/api/query" \
        -H 'Content-Type: application/json' \
        -d '{"query": "food:\"Salad\".EnergyPerServing"}' | python3 -m json.tool
    echo ""

    echo -e "${BLUE}3. KB query — France population density:${NC}"
    curl -s -X POST "http://localhost:$PORT/api/query" \
        -H 'Content-Type: application/json' \
        -d '{"query": "country:\"France\".PopulationDensity"}' | python3 -m json.tool
    echo ""

    echo -e "${BLUE}4. KB query — Water molar mass:${NC}"
    curl -s -X POST "http://localhost:$PORT/api/query" \
        -H 'Content-Type: application/json' \
        -d '{"query": "material:\"Water\".MolarMass"}' | python3 -m json.tool
    echo ""

    echo -e "${BLUE}5. Put new data — Pizza calories:${NC}"
    curl -s -X POST "http://localhost:$PORT/api/put" \
        -H 'Content-Type: application/json' \
        -d '{"entity_type": "food", "key": "Pizza", "property": "Calories", "value": "285"}' | python3 -m json.tool
    echo ""

    echo -e "${BLUE}6. List stored subreddits:${NC}"
    curl -s "http://localhost:$PORT/api/subreddits" | python3 -m json.tool
    echo ""

    echo -e "${GREEN}Demo complete!${NC}"
}

start_single() {
    stop_nodes
    build_node

    echo ""
    echo -e "${BLUE}=== Starting Tabula Node ===${NC}"
    echo ""

    rm -rf "${DATA_DIR}/single"
    mkdir -p "${DATA_DIR}/single"

    echo -e "${GREEN}Starting node with genesis + mining + KB seed data${NC}"
    echo -e "API: ${CYAN}http://localhost:8080${NC}"
    echo ""
    $BINARY \
        --data-dir "${DATA_DIR}/single" \
        --port 0 \
        --api-port 8080 \
        --mine \
        --key-seed "local-test" \
        --genesis --seed-genesis \
        --no-mdns \
        2>&1 &
    echo $! >> "$PID_FILE"

    echo ""
    echo "Try these in another terminal:"
    echo ""
    echo "  # Chain status"
    echo "  curl http://localhost:8080/api/status | python3 -m json.tool"
    echo ""
    echo "  # Query the knowledge base"
    echo "  curl -X POST http://localhost:8080/api/query -H 'Content-Type: application/json' \\"
    echo "    -d '{\"query\": \"food:\\\"Salad\\\".EnergyPerServing\"}'"
    echo ""
    echo "  # Store new data"
    echo "  curl -X POST http://localhost:8080/api/put -H 'Content-Type: application/json' \\"
    echo "    -d '{\"entity_type\": \"food\", \"key\": \"Pizza\", \"property\": \"Calories\", \"value\": \"285\"}'"
    echo ""
    echo -e "${YELLOW}Press Ctrl+C to stop${NC}"

    trap 'stop_nodes; exit 0' SIGINT SIGTERM
    wait
}

start_multi() {
    stop_nodes
    build_node

    echo ""
    echo -e "${BLUE}=== Starting Tabula 3-Node Network ===${NC}"
    echo -e "${YELLOW}NOTE: Block sync between nodes is WIP (Milestone 2).${NC}"
    echo -e "${YELLOW}      Nodes discover peers via mDNS but mine independently.${NC}"
    echo ""

    rm -rf "${DATA_DIR}"
    mkdir -p "${DATA_DIR}/node1" "${DATA_DIR}/node2" "${DATA_DIR}/node3"

    # Node 1: Genesis + mining
    echo -e "${GREEN}[Node 1]${NC} Genesis + Mining | API :8001"
    $BINARY \
        --data-dir "${DATA_DIR}/node1" \
        --port 9001 \
        --api-port 8001 \
        --mine \
        --key-seed "node1" \
        --genesis --seed-genesis \
        2>&1 | sed "s/^/[node1] /" &
    echo $! >> "$PID_FILE"

    sleep 3

    # Node 2: Sync only
    echo -e "${GREEN}[Node 2]${NC} Sync Only       | API :8002"
    $BINARY \
        --data-dir "${DATA_DIR}/node2" \
        --port 9002 \
        --api-port 8002 \
        --key-seed "node2" \
        2>&1 | sed "s/^/[node2] /" &
    echo $! >> "$PID_FILE"

    # Node 3: Mining
    echo -e "${GREEN}[Node 3]${NC} Mining          | API :8003"
    $BINARY \
        --data-dir "${DATA_DIR}/node3" \
        --port 9003 \
        --api-port 8003 \
        --mine \
        --key-seed "node3" \
        2>&1 | sed "s/^/[node3] /" &
    echo $! >> "$PID_FILE"

    echo ""
    echo -e "${GREEN}All 3 nodes started!${NC}"
    echo ""
    echo "  Node 1 (genesis+mining): http://localhost:8001/api/status"
    echo "  Node 2 (sync):           http://localhost:8002/api/status"
    echo "  Node 3 (mining):         http://localhost:8003/api/status"
    echo ""
    echo -e "${YELLOW}Press Ctrl+C to stop all nodes${NC}"

    trap 'stop_nodes; exit 0' SIGINT SIGTERM
    wait
}

# --- Main ---

case "${1:-}" in
    --status)
        check_status
        ;;
    --stop)
        stop_nodes
        ;;
    --multi)
        start_multi
        ;;
    --demo)
        start_single &
        BG_PID=$!
        sleep 5
        run_demo 8080
        echo ""
        echo -e "${YELLOW}Node is still running. Use ./test-local-network.sh --stop to shut down.${NC}"
        echo -e "${YELLOW}Or press Ctrl+C.${NC}"
        wait $BG_PID
        ;;
    *)
        start_single
        ;;
esac
