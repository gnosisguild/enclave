#!/usr/bin/env bash

set -euo pipefail

(export PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80" CHAIN_ID=31337 $(enclave print-env --chain hardhat) && pnpm ts-node ./server)
