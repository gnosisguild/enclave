#!/usr/bin/env bash

set -euo pipefail

# nuke past installations as we are adding these nodes to the contract
rm -rf ./enclave/data
rm -rf ./enclave/config

PRIVATE_KEY_AG="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
PRIVATE_KEY_CN1="0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d"
PRIVATE_KEY_CN2="0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a"
PRIVATE_KEY_CN3="0x7c852118294e51e653712a81e05800f419141751be58f605c371e15141b007a6"

enclave wallet set --name ag --private-key "$PRIVATE_KEY_AG"
enclave wallet set --name cn1 --private-key "$PRIVATE_KEY_CN1"
enclave wallet set --name cn2 --private-key "$PRIVATE_KEY_CN2"
enclave wallet set --name cn3 --private-key "$PRIVATE_KEY_CN3"

# using & instead of -d so that wait works below
enclave nodes up -v &

sleep 2

CN1=$(cat ./enclave.config.yaml | yq -r '.nodes.cn1.address')
CN2=$(cat ./enclave.config.yaml | yq -r '.nodes.cn2.address')
CN3=$(cat ./enclave.config.yaml | yq -r '.nodes.cn3.address')

# Add ciphernodes using variables from config.sh
pnpm ciphernode:add --ciphernode-address "$CN1" --network "localhost"
pnpm ciphernode:add --ciphernode-address "$CN2" --network "localhost"
pnpm ciphernode:add --ciphernode-address "$CN3" --network "localhost"

wait
