#!/usr/bin/env bash

set -euo pipefail

pnpm wait-on http://localhost:8545 && \
  concurrently -r \
    "pnpm dev:server" \
    "enclave program start --json-rpc-server http://localhost:8080 --chain hardhat"
