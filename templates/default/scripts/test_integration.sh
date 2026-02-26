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

export $(enclave print-env --chain localhost)
(pnpm concurrently \
  --names "TEST,EVM,CIPHER,SERVER,PROGRAM" \
  --prefix-colors "blue,cyan,magenta,yellow,green" \
  --kill-others \
  --success first \
  "wait-on http://localhost:13151/health && pnpm vitest run ./tests/integration.spec.ts" \
  "anvil --host 0.0.0.0 --chain-id 31337 --block-time 1 --mnemonic 'test test test test test test test test test test test junk' --silent" \
  "pnpm dev:ciphernodes" \
  "TEST_MODE=1 pnpm dev:server" \
  "pnpm dev:program" && passed_message) || failed_message
