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
  echo "❌❌❌❌❌❌❌❌❌❌❌❌❌❌❌"
  echo "  ❌ Test failed  "
  echo "❌❌❌❌❌❌❌❌❌❌❌❌❌❌❌"
  echo ""
}

export $(enclave print-env --chain hardhat)

(pnpm concurrently \
  --names "TEST,EVM,CIPHER,SERVER,PROGRAM" \
  --prefix-colors "blue,cyan,magenta,yellow,green" \
  --kill-others \
  --success first \
  "wait-on http://localhost:13151/health && pnpm ts-node ./tests/integration.spec.ts" \
  "pnpm dev:evm" \
  "pnpm dev:ciphernodes" \
  "TEST_MODE=1 pnpm dev:server" \
  "enclave program start" && passed_message) || failed_message
