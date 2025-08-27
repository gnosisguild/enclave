#!/usr/bin/env bash

set -euo pipefail

echo "Deploying CRISP Contracts..."
# TODO: put the following somewhere central
export ETH_WALLET_PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"

(FOUNDRY_PROFILE=local USE_MOCK_VERIFIER=true forge script --rpc-url http://localhost:8545 --broadcast deploy/Deploy.s.sol)
echo "CRISP Contracts deployed."
