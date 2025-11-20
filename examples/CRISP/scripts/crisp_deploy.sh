#!/usr/bin/env bash

set -euo pipefail

echo "Deploying CRISP Contracts..."

export PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"

cd packages/crisp-contracts

pnpm clean:deployments --network localhost
pnpm deploy:contracts:full --network localhost
