#!/usr/bin/env bash

set -euo pipefail

cleanup() {
  echo "Cleaning up processes..."
  enclave nodes down
  echo "Cleanup complete"
  exit 0
}

trap cleanup INT TERM

echo "Waiting for local evm node..."
pnpm wait-on http://localhost:8545

# nuke past installations as we are adding these nodes to the contract
rm -rf .enclave/data
rm -rf .enclave/config

PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"

enclave wallet set --name ag --private-key "$PRIVATE_KEY" 

# using & instead of -d so that wait works below
enclave nodes up -v &

sleep 2

CN1=$(cat enclave.config.yaml | yq '.nodes.cn1.address')
CN2=$(cat enclave.config.yaml | yq '.nodes.cn2.address')
CN3=$(cat enclave.config.yaml | yq '.nodes.cn3.address')

# Add ciphernodes using variables from config.sh
pnpm hardhat ciphernode:add --ciphernode-address $CN1 --network localhost
pnpm hardhat ciphernode:add --ciphernode-address $CN2 --network localhost
pnpm hardhat ciphernode:add --ciphernode-address $CN3 --network localhost

wait
