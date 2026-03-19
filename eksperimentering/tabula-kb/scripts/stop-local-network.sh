#!/bin/bash
# Stop all running Tabula node processes.

echo "Stopping all Tabula node processes..."
pkill -f "target/release/node" 2>/dev/null || true
pkill -f "target/debug/node" 2>/dev/null || true
echo "Done."
