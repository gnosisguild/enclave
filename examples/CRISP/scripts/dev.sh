#!/usr/bin/env bash

set -euo pipefail

export CARGO_INCREMENTAL=1

cleanup() {
  echo "Cleaning up processes..."
  pkill -9 -f "enclave start"
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

concurrently \
  -ks first \
  --names "ANVIL,DEPLOY,NODES" \
  --prefix-colors "blue,green,yellow" \
  "anvil" \
  "./scripts/evm_deploy.sh && ./scripts/crisp_deploy.sh && ./scripts/dev_services.sh"

