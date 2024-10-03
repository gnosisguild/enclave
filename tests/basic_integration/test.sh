#!/usr/bin/env bash

set -eu  # Exit immediately if a command exits with a non-zero status
#
# Get the script's location
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
PLAINTEXT="1234,567890"

if [[ "$ROOT_DIR" != "$(pwd)" ]]; then 
  echo "This script must be run from the root"
  exit 1
fi


# Environment variables
RPC_URL="ws://localhost:8545"

PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"

# These contracts are based on the deterministic order of hardhat deploy
# We _may_ wish to get these off the hardhat environment somehow?
ENCLAVE_CONTRACT="0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0"
REGISTRY_CONTRACT="0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9"
REGISTRY_FILTER_CONTRACT="0xDc64a140Aa3E981100a9becA4E685f962f0cF6C9"
INPUT_VALIDATOR_CONTRACT="0x8A791620dd6260079BF849Dc5567aDC3F2FdC318"
# These are random addresses for now
CIPHERNODE_ADDRESS_1="0x2546BcD3c84621e976D8185a91A922aE77ECEc30"
CIPHERNODE_ADDRESS_2="0xbDA5747bFD65F08deb54cb465eB87D40e51B197E"
CIPHERNODE_ADDRESS_3="0xdD2FD4581271e230360230F9337D5c0430Bf44C0"
CIPHERNODE_ADDRESS_4="0x8626f6940E2eb28930eFb4CeF49B2d1F2C9C1199"

# Function to clean up background processes
cleanup() {
    echo "Cleaning up processes..."
    kill $(jobs -p) 2>/dev/null
    exit ${1:-1}
}

heading() {
    echo ""
    echo ""
    echo "--------------------------------------------------------------"
    echo " $1     "
    echo "--------------------------------------------------------------"
    echo ""
}

waiton() {
    local file_path="$1"
    until [ -f "$file_path" ]; do
        sleep 1
    done
}

waiton-files() {
  local timeout=600  # 10 minutes timeout
  local start_time=$(date +%s)
  while true; do
    all_exist=true
    for file in "$@"; do
      if [ ! -f "$file" ]; then
        all_exist=false
        break
      fi
    done
    if $all_exist; then
      break
    fi
    if [ $(($(date +%s) - start_time)) -ge $timeout ]; then
      echo "Timeout waiting for files: $@" >&2
      return 1
    fi
    sleep 1
  done
}

pkill -9 -f "target/debug/node" || true
pkill -9 -f "hardhat node" || true
pkill -9 -f "target/debug/aggregator" || true

# Set up trap to catch errors and interrupts
trap 'cleanup $?' ERR INT TERM

# Delete output artifacts
rm -rf $ROOT_DIR/tests/basic_integration/output/*

$SCRIPT_DIR/lib/prebuild.sh

heading "Start the EVM node"

yarn evm:node &

until curl -f -s "http://localhost:8545" > /dev/null; do
  sleep 1
done

# Launch 4 ciphernodes

heading "Launch ciphernode $CIPHERNODE_ADDRESS_1"
yarn ciphernode:launch --address $CIPHERNODE_ADDRESS_1 --rpc "$RPC_URL" --enclave-contract $ENCLAVE_CONTRACT --registry-contract $REGISTRY_CONTRACT &

heading "Launch ciphernode $CIPHERNODE_ADDRESS_2"
yarn ciphernode:launch --address $CIPHERNODE_ADDRESS_2 --rpc "$RPC_URL" --enclave-contract $ENCLAVE_CONTRACT --registry-contract $REGISTRY_CONTRACT &

heading "Launch ciphernode $CIPHERNODE_ADDRESS_3"
yarn ciphernode:launch --address $CIPHERNODE_ADDRESS_3 --rpc "$RPC_URL" --enclave-contract $ENCLAVE_CONTRACT --registry-contract $REGISTRY_CONTRACT &

heading "Launch ciphernode $CIPHERNODE_ADDRESS_4"
yarn ciphernode:launch --address $CIPHERNODE_ADDRESS_4 --rpc "$RPC_URL" --enclave-contract $ENCLAVE_CONTRACT --registry-contract $REGISTRY_CONTRACT &

# NOTE: This node is configured to be an aggregator
PRIVATE_KEY=$PRIVATE_KEY yarn ciphernode:aggregator --rpc "$RPC_URL" --enclave-contract $ENCLAVE_CONTRACT --registry-contract $REGISTRY_CONTRACT  --registry-filter-contract $REGISTRY_FILTER_CONTRACT --pubkey-write-path "$SCRIPT_DIR/output/pubkey.bin" --plaintext-write-path "$SCRIPT_DIR/output/plaintext.txt" &

sleep 1

waiton-files "$ROOT_DIR/packages/ciphernode/target/debug/node" "$ROOT_DIR/packages/ciphernode/target/debug/aggregator" "$ROOT_DIR/packages/ciphernode/target/debug/fake_encrypt"

heading "Add ciphernode $CIPHERNODE_ADDRESS_1"
yarn ciphernode:add --ciphernode-address $CIPHERNODE_ADDRESS_1 --network localhost

heading "Add ciphernode $CIPHERNODE_ADDRESS_2"
yarn ciphernode:add --ciphernode-address $CIPHERNODE_ADDRESS_2 --network localhost

heading "Add ciphernode $CIPHERNODE_ADDRESS_3"
yarn ciphernode:add --ciphernode-address $CIPHERNODE_ADDRESS_3 --network localhost

heading "Add ciphernode $CIPHERNODE_ADDRESS_4"
yarn ciphernode:add --ciphernode-address $CIPHERNODE_ADDRESS_4 --network localhost

heading "Request Committee"

ENCODED_PARAMS=0x$($SCRIPT_DIR/lib/pack_e3_params.sh --moduli 0x3FFFFFFF000001 --degree 2048 --plaintext-modulus 1032193)

yarn committee:new --network localhost --duration 4 --e3-params "$ENCODED_PARAMS"

waiton "$SCRIPT_DIR/output/pubkey.bin"
PUBLIC_KEY=$(xxd -p -c 10000000 "$SCRIPT_DIR/output/pubkey.bin")

heading "Mock encrypted plaintext"
$SCRIPT_DIR/lib/fake_encrypt.sh --input "$SCRIPT_DIR/output/pubkey.bin" --output "$SCRIPT_DIR/output/output.bin" --plaintext $PLAINTEXT

heading "Mock activate e3-id"
yarn e3:activate --e3-id 0 --public-key "0x$PUBLIC_KEY" --network localhost

heading "Mock publish input e3-id"
yarn e3:publishInput --network localhost  --e3-id 0 --data 0x12345678

sleep 4 # wait for input deadline to pass

waiton "$SCRIPT_DIR/output/output.bin"

heading "Publish ciphertext to EVM"
yarn e3:publishCiphertext --e3-id 0 --network localhost --data-file "$SCRIPT_DIR/output/output.bin" --proof 0x12345678

waiton "$SCRIPT_DIR/output/plaintext.txt"

ACTUAL=$(cat $SCRIPT_DIR/output/plaintext.txt)
 

# Assume plaintext is shorter

if [[ "$ACTUAL" != "$PLAINTEXT"* ]]; then
  echo "Invalid plaintext decrypted: actual='$ACTUAL' expected='$PLAINTEXT'"
  echo "Test FAILED"
  exit 1
fi

heading "Test PASSED !"

cleanup 0

