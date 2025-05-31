#!/usr/bin/env bash

set -euo pipefail

# nuke past installations as we are adding these nodes to the contract
rm -rf /app/examples/CRISP/.enclave/*

PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"

enclave wallet set --name ag --private-key "$PRIVATE_KEY" 

# using & instead of -d so that wait works below
enclave nodes up -v &

sleep 2

CN1=$(cat enclave.config.yaml | yq '.nodes.cn1.address')
CN2=$(cat enclave.config.yaml | yq '.nodes.cn2.address')
CN3=$(cat enclave.config.yaml | yq '.nodes.cn3.address')

cd /app


# Add ciphernodes using variables from config.sh
pnpm ciphernode:add --ciphernode-address "$CN1" --network "localhost"
pnpm ciphernode:add --ciphernode-address "$CN2" --network "localhost"
pnpm ciphernode:add --ciphernode-address "$CN3" --network "localhost"

wait
