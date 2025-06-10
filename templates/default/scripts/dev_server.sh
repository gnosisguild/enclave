#!/usr/bin/env bash

set -euo pipefail

pnpm wait-on http://localhost:8545 && \
  (export PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80" CHAIN_ID=31337 && \
  export $(enclave print-env --chain hardhat) && \
  export RPC_URL="http://localhost:8545" && \
  pnpm ts-node ./server)
