#!/usr/bin/env bash

set -euo pipefail

SIGNAL_FILE=/tmp/enclave_ciphernodes_ready

cleanup() {
  echo "Cleaning up processes..."
  pkill -9 -f "enclave start"
  sleep 2
  pkill enclave
  echo "Cleanup complete"
  exit 0
}

rm -rf $SIGNAL_FILE

trap cleanup INT TERM

echo "Waiting for local evm node..."
pnpm wait-on http://localhost:8545

# nuke past installations as we are adding these nodes to the contract
rm -rf .enclave/data
rm -rf .enclave/config

PRIVATE_KEY_AG="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
PRIVATE_KEY_CN1="0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d"
PRIVATE_KEY_CN2="0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a"
PRIVATE_KEY_CN3="0x7c852118294e51e653712a81e05800f419141751be58f605c371e15141b007a6"

enclave wallet set --name ag --private-key "$PRIVATE_KEY_AG" 
enclave wallet set --name cn1 --private-key "$PRIVATE_KEY_CN1" 
enclave wallet set --name cn2 --private-key "$PRIVATE_KEY_CN2" 
enclave wallet set --name cn3 --private-key "$PRIVATE_KEY_CN3" 

# using & instead of -d so that wait works below
enclave nodes up -v --experimental-trbfv &

sleep 2

CN1=$(grep -A 1 'cn1:' enclave.config.yaml | grep 'address:' | sed 's/.*address: *"\([^"]*\)".*/\1/')
CN2=$(grep -A 1 'cn2:' enclave.config.yaml | grep 'address:' | sed 's/.*address: *"\([^"]*\)".*/\1/')
CN3=$(grep -A 1 'cn3:' enclave.config.yaml | grep 'address:' | sed 's/.*address: *"\([^"]*\)".*/\1/')

# Add ciphernodes using variables from config.sh
pnpm run deploy && sleep 2
pnpm hardhat ciphernode:admin-add --ciphernode-address $CN1 --network localhost
pnpm hardhat ciphernode:admin-add --ciphernode-address $CN2 --network localhost
pnpm hardhat ciphernode:admin-add --ciphernode-address $CN3 --network localhost

# Function to send RPC request.
send_rpc() {
    local method="$1"
    local params="$2"
    curl -X POST \
        -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"method\":\"$method\",\"params\":$params,\"id\":1}" \
        http://localhost:8545 > /dev/null 2>&1
}

# Configure mining settings for development environment
# Disable automatic mining and set interval mining to 1 second for predictable block times.
send_rpc "evm_setAutomine" "[false]"
send_rpc "evm_increaseTime" "[10]"
send_rpc "evm_setIntervalMining" "[1000]"

touch $SIGNAL_FILE

wait
