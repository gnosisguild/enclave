#!/usr/bin/env bash

set -euxo pipefail

# TODO: put the following somewhere central
export ETH_WALLET_PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"

(cd ./risc0 && forge script --rpc-url http://localhost:8545 --broadcast script/Deploy.s.sol)
