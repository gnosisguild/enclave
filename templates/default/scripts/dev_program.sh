#!/usr/bin/env bash

set -euo pipefail

pnpm wait-on http://localhost:8545 && \
  concurrently -r \
    "pnpm server" \
    "enclave program listen --json-rpc-server http://localhost:8080 --chain hardhat"
