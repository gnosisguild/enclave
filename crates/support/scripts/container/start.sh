#!/usr/bin/env bash

# Clear any existing environment variables
unset BONSAI_API_KEY BONSAI_API_URL

# Parse command line arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    --api-key)
      export BONSAI_API_KEY="$2"
      shift 2
      ;;
    --api-url)
      export BONSAI_API_URL="$2"
      shift 2
      ;;
    *)
      echo "Unknown argument: $1"
      exit 1
      ;;
  esac
done

CARGO_INCREMENTAL=1

[ -z "$BONSAI_API_KEY" ] && export RISC0_DEV_MODE=1

echo "RISC0_DEV_MODE=$RISC0_DEV_MODE"

exec cargo run --bin e3-support-app "$@"
