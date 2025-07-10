#!/usr/bin/env bash

set -euo pipefail

passed_message() {
  echo ""
  echo "------------------------"
  echo "  ✅ Test has passed!   "
  echo "------------------------"
  echo ""
}

failed_message() {
  echo ""
  echo "------------------------"
  echo "  ❌ Test failed  "
  echo "------------------------"
  echo ""
  exit 1
}

export $(enclave print-env --chain hardhat)
enclave program compile && (pnpm concurrently \
  --names "TEST,EVM,CIPHER,SERVER,PROGRAM" \
  --prefix-colors "blue,cyan,magenta,yellow,green" \
  --kill-others \
  --success first \
  "wait-on http://localhost:13151/health && pnpm ts-node ./tests/integration.spec.ts" \
  "pnpm dev:evm" \
  "pnpm dev:ciphernodes" \
  "TEST_MODE=1 pnpm dev:server" \
  "wait-on file:/tmp/enclave_ciphernodes_ready && enclave program start" && passed_message) || failed_message
