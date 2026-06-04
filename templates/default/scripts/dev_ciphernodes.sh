#!/usr/bin/env bash

set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

# shellcheck disable=SC1091
source "$(dirname "${BASH_SOURCE[0]}")/lib/dev_config.sh"
load_template_dev_config

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
pnpm wait-on tcp:localhost:8545

if [ ! -f './.enclave/generated/contracts/ImageID.sol' ]; then
  echo "Compiling guest program (ImageID)..."
  enclave program compile
fi

# Fresh node state for this deploy
rm -rf .enclave/data
rm -rf .enclave/config

PRIVATE_KEY_CN1="0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d"
PRIVATE_KEY_CN2="0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a"
PRIVATE_KEY_CN3="0x7c852118294e51e653712a81e05800f419141751be58f605c371e15141b007a6"
PRIVATE_KEY_CN4="0x47e179ec197488593b187f80a00eb0da91f1b9d0b13f8733639f19c30a34926a"
PRIVATE_KEY_CN5="0x8b3a350cf5c34c9194ca85829a2df0ec3153be0318b5e2d3348e872092edffba"

enclave wallet set --name cn1 --private-key "$PRIVATE_KEY_CN1"
enclave wallet set --name cn2 --private-key "$PRIVATE_KEY_CN2"
enclave wallet set --name cn3 --private-key "$PRIVATE_KEY_CN3"
enclave wallet set --name cn4 --private-key "$PRIVATE_KEY_CN4"
enclave wallet set --name cn5 --private-key "$PRIVATE_KEY_CN5"

echo "Setting up ZK prover..."
enclave noir setup

sync_enclave_circuit_artifacts

# Deploy before starting nodes so enclave.config.yaml addresses match the chain.
echo "Deploying protocol + MyProgram..."
pnpm exec hardhat utils:clean-deployments --network localhost
pnpm exec hardhat run scripts/deploy-local.ts --network localhost
if ! grep -q '"MyProgram"' deployed_contracts.json; then
  echo "deployTemplate did not record MyProgram — check deploy logs above"
  exit 1
fi

CN1=$(grep -A 1 'cn1:' enclave.config.yaml | grep 'address:' | sed 's/.*address: *"\([^"]*\)".*/\1/')
CN2=$(grep -A 1 'cn2:' enclave.config.yaml | grep 'address:' | sed 's/.*address: *"\([^"]*\)".*/\1/')
CN3=$(grep -A 1 'cn3:' enclave.config.yaml | grep 'address:' | sed 's/.*address: *"\([^"]*\)".*/\1/')
CN4=$(grep -A 1 'cn4:' enclave.config.yaml | grep 'address:' | sed 's/.*address: *"\([^"]*\)".*/\1/')
CN5=$(grep -A 1 'cn5:' enclave.config.yaml | grep 'address:' | sed 's/.*address: *"\([^"]*\)".*/\1/')

echo "Starting ciphernodes (post-deploy config)..."
enclave nodes up -v &

sleep 4

pnpm hardhat ciphernode:admin-add --ciphernode-address $CN1 --network localhost
pnpm hardhat ciphernode:admin-add --ciphernode-address $CN2 --network localhost
pnpm hardhat ciphernode:admin-add --ciphernode-address $CN3 --network localhost
pnpm hardhat ciphernode:admin-add --ciphernode-address $CN4 --network localhost
pnpm hardhat ciphernode:admin-add --ciphernode-address $CN5 --network localhost

touch $SIGNAL_FILE

wait
