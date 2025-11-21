#!/usr/bin/env bash

# Clear any existing environment variables
unset RISC0_DEV_MODE RPC_URL PRIVATE_KEY PINATA_JWT PROGRAM_URL BOUNDLESS_ONCHAIN

# Parse command line arguments
POSITIONAL=()
while [[ $# -gt 0 ]]; do
  case $1 in
    --risc0-dev-mode)
      export RISC0_DEV_MODE="$2"
      shift 2
      ;;
    --rpc-url)
      export RPC_URL="$2"
      shift 2
      ;;
    --private-key)
      export PRIVATE_KEY="$2"
      shift 2
      ;;
    --pinata-jwt)
      export PINATA_JWT="$2"
      shift 2
      ;;
    --program-url)
      export PROGRAM_URL="$2"
      shift 2
      ;;
    --boundless-onchain)
      export BOUNDLESS_ONCHAIN="$2"
      shift 2
      ;;
    *)
      POSITIONAL+=("$1")
      shift
      ;;
  esac
done

set -- "${POSITIONAL[@]}" 

CARGO_INCREMENTAL=1

# Default to dev mode if no Boundless configuration provided
if [ -z "$RISC0_DEV_MODE" ]; then
  if [ -z "$RPC_URL" ]; then
    export RISC0_DEV_MODE=1
    echo "No Boundless config found, defaulting to dev mode"
  fi
fi

echo "RISC0_DEV_MODE=$RISC0_DEV_MODE"
[ -n "$RPC_URL" ] && echo "Using Boundless (RPC: $RPC_URL)"

exec cargo run --bin e3-support-app "$@"
