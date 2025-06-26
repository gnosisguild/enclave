#!/usr/bin/env bash

# Clear any existing environment variables
unset BONSAI_API_KEY BONSAI_API_URL

# Parse command line arguments
POSITIONAL=()
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
    --risc0-dev-mode)
      export RISC0_DEV_MODE="$2"
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

if [ -z "$RISC0_DEV_MODE" ]; then
  [ -z "$BONSAI_API_KEY" ] && export RISC0_DEV_MODE=1
fi

echo "RISC0_DEV_MODE=$RISC0_DEV_MODE"

exec cargo run --bin e3-support-app "$@"
