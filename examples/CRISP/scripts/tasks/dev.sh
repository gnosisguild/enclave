#!/usr/bin/env bash

set -euo pipefail

cleanup() {
  echo "Cleaning up processes..."
  enclave nodes down
  sleep 1

  pkill -9 -f "anvil" 2>/dev/null || true
  
  # Kill any remaining background jobs from this script
  jobs -p | xargs -r kill -9 2>/dev/null || true
  
  # Give processes a moment to terminate
  sleep 1
  
  # Double-check if anvil is still running and force kill it
  if pgrep -f "anvil" > /dev/null; then
    echo "Anvil still running, force killing..."
    pkill -9 -f "anvil" || true
  fi
  
  echo "Cleanup complete"
  exit 0
}

trap cleanup INT TERM

(cd /app && pnpm install -y --frozen-lockfile)

concurrently \
  --names "ANVIL,DEPLOY,NODES" \
  --prefix-colors "blue,green,yellow" \
  "anvil" \
  "./scripts/tasks/evm_deploy.sh && ./scripts/tasks/risc0_deploy.sh && ./scripts/tasks/dev_services.sh"

