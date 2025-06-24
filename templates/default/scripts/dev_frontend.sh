#!/usr/bin/env bash

set -euo pipefail

echo "Waiting for local evm node..."
pnpm wait-on http://localhost:8545

echo "Waiting for program runner..."
pnpm wait-on http://localhost:13151/health

cd client && (export $(enclave print-env --vite --chain hardhat) && pnpm dev)
