#!/usr/bin/env bash

set -euo pipefail
READYFILE=$1

# nuke past installations as we are adding these nodes to the contract
rm -rf ./.enclave/data
rm -rf ./.enclave/config
rm -rf $READYFILE

PRIVATE_KEY_AG="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
PRIVATE_KEY_CN1="0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d"
PRIVATE_KEY_CN2="0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a"
PRIVATE_KEY_CN3="0x7c852118294e51e653712a81e05800f419141751be58f605c371e15141b007a6"
PRIVATE_KEY_CN4="0x47e179ec197488593b187f80a00eb0da91f1b9d0b13f8733639f19c30a34926a"
PRIVATE_KEY_CN5="0x8b3a350cf5c34c9194ca85829a2df0ec3153be0318b5e2d3348e872092edffba"

enclave wallet set --name ag --private-key "$PRIVATE_KEY_AG"
enclave wallet set --name cn1 --private-key "$PRIVATE_KEY_CN1"
enclave wallet set --name cn2 --private-key "$PRIVATE_KEY_CN2"
enclave wallet set --name cn3 --private-key "$PRIVATE_KEY_CN3"
enclave wallet set --name cn4 --private-key "$PRIVATE_KEY_CN4"
enclave wallet set --name cn5 --private-key "$PRIVATE_KEY_CN5"

# using & instead of -d so that wait works below
# TODO: add --experimental-trbfv after testing
enclave nodes up -v --experimental-trbfv &

sleep 2

CN1=$(cat ./enclave.config.yaml | yq -r '.nodes.cn1.address')
CN2=$(cat ./enclave.config.yaml | yq -r '.nodes.cn2.address')
CN3=$(cat ./enclave.config.yaml | yq -r '.nodes.cn3.address')
CN4=$(cat ./enclave.config.yaml | yq -r '.nodes.cn4.address')
CN5=$(cat ./enclave.config.yaml | yq -r '.nodes.cn5.address')

# Add ciphernodes using variables from config.sh
pnpm ciphernode:add --ciphernode-address "$CN1" --network "localhost"
pnpm ciphernode:add --ciphernode-address "$CN2" --network "localhost"
pnpm ciphernode:add --ciphernode-address "$CN3" --network "localhost"
pnpm ciphernode:add --ciphernode-address "$CN4" --network "localhost"
pnpm ciphernode:add --ciphernode-address "$CN5" --network "localhost"

echo 1 > $READYFILE

echo "CIPHERNODES HAVE BEEN ADDED."

wait
