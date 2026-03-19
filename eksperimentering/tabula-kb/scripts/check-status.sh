#!/bin/bash
# Check the status of all local testnet nodes via their HTTP APIs.
#
# Usage:
#   ./scripts/check-status.sh

BLUE='\033[0;34m'
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

for i in 1 2 3; do
    PORT=$((8000 + i))
    echo -e "${BLUE}=== Node $i (localhost:$PORT) ===${NC}"
    RESPONSE=$(curl -s "http://localhost:$PORT/api/status" 2>/dev/null)
    if [ $? -eq 0 ] && [ -n "$RESPONSE" ]; then
        echo -e "${GREEN}Online${NC}"
        echo "$RESPONSE" | python3 -m json.tool 2>/dev/null || echo "$RESPONSE"
    else
        echo -e "${RED}Offline or unreachable${NC}"
    fi
    echo ""
done
