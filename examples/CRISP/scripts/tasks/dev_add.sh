#!/usr/bin/env bash

set -euo pipefail

CN1=$(cat enclave.config.yaml | yq '.nodes.cn1.address')
CN2=$(cat enclave.config.yaml | yq '.nodes.cn2.address')
CN3=$(cat enclave.config.yaml | yq '.nodes.cn3.address')

cd /app

# Add ciphernodes using variables from config.sh
pnpm ciphernode:add --ciphernode-address "$CN1" --network "localhost"
pnpm ciphernode:add --ciphernode-address "$CN2" --network "localhost"
pnpm ciphernode:add --ciphernode-address "$CN3" --network "localhost"

