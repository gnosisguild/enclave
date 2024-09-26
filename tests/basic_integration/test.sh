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
export RPC_URL="ws://localhost:8545"
# These contracts are based on the deterministic order of hardhat deploy
export ENCLAVE_CONTRACT="0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0"
export REGISTRY_CONTRACT="0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9"
# These are random addresses for now
export CIPHERNODE_ADDRESS_1="0x2546BcD3c84621e976D8185a91A922aE77ECEc30"
export CIPHERNODE_ADDRESS_2="0xbDA5747bFD65F08deb54cb465eB87D40e51B197E"
export CIPHERNODE_ADDRESS_3="0xdD2FD4581271e230360230F9337D5c0430Bf44C0"
export CIPHERNODE_ADDRESS_4="0x8626f6940E2eb28930eFb4CeF49B2d1F2C9C1199"

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

heading "Launch ciphernode $CIPHERNODE_ADDRESS_1"
yarn ciphernode:launch --address $CIPHERNODE_ADDRESS_1 --rpc "$RPC_URL" --enclave-contract $ENCLAVE_CONTRACT --registry-contract $REGISTRY_CONTRACT &

heading "Launch ciphernode $CIPHERNODE_ADDRESS_2"
yarn ciphernode:launch --address $CIPHERNODE_ADDRESS_2 --rpc "$RPC_URL" --enclave-contract $ENCLAVE_CONTRACT --registry-contract $REGISTRY_CONTRACT &

heading "Launch ciphernode $CIPHERNODE_ADDRESS_3"
yarn ciphernode:launch --address $CIPHERNODE_ADDRESS_3 --rpc "$RPC_URL" --enclave-contract $ENCLAVE_CONTRACT --registry-contract $REGISTRY_CONTRACT &

heading "Launch ciphernode $CIPHERNODE_ADDRESS_4"
yarn ciphernode:launch --address $CIPHERNODE_ADDRESS_4 --rpc "$RPC_URL" --enclave-contract $ENCLAVE_CONTRACT --registry-contract $REGISTRY_CONTRACT &

yarn ciphernode:aggregator --rpc "$RPC_URL" --enclave-contract $ENCLAVE_CONTRACT --registry-contract $REGISTRY_CONTRACT --pubkey-write-path "$SCRIPT_DIR/output/pubkey.bin" --plaintext-write-path "$SCRIPT_DIR/output/plaintext.txt" &

sleep 1

waiton-files "$ROOT_DIR/packages/ciphernode/target/debug/node" "$ROOT_DIR/packages/ciphernode/target/debug/aggregator" "$ROOT_DIR/packages/ciphernode/target/debug/test_encryptor"

heading "Add ciphernode $CIPHERNODE_ADDRESS_1"
yarn ciphernode:add --ciphernode-address $CIPHERNODE_ADDRESS_1 --network localhost

heading "Add ciphernode $CIPHERNODE_ADDRESS_2"
yarn ciphernode:add --ciphernode-address $CIPHERNODE_ADDRESS_2 --network localhost

heading "Add ciphernode $CIPHERNODE_ADDRESS_3"
yarn ciphernode:add --ciphernode-address $CIPHERNODE_ADDRESS_3 --network localhost

heading "Add ciphernode $CIPHERNODE_ADDRESS_4"
yarn ciphernode:add --ciphernode-address $CIPHERNODE_ADDRESS_4 --network localhost

heading "Request Committee"

yarn committee:new --network localhost --duration 4

waiton "$SCRIPT_DIR/output/pubkey.bin"

heading "Mock encrypted plaintext"

$SCRIPT_DIR/lib/fake_encrypt.sh --input "$SCRIPT_DIR/output/pubkey.bin" --output "$SCRIPT_DIR/output/output.bin" --plaintext $PLAINTEXT

heading "Mock publish committee key"

yarn committee:publish --e3-id 0 --nodes $CIPHERNODE_ADDRESS_1,$CIPHERNODE_ADDRESS_2,$CIPHERNODE_ADDRESS_3,$CIPHERNODE_ADDRESS_4 --public-key 0x12345678 --network localhost

heading "Mock activate e3-id"

yarn e3:activate --e3-id 0 --network localhost

heading "Mock publish input e3-id"
yarn e3:publishInput --network localhost  --e3-id 0 --data 0x12345678

sleep 4 # wait for input deadline to pass

waiton "$SCRIPT_DIR/output/output.bin"

heading "Publish ciphertext to EVM"

yarn e3:publishCiphertext --e3-id 0 --network localhost --data-file "$SCRIPT_DIR/output/output.bin" --proof 0x12345678

waiton "$SCRIPT_DIR/output/plaintext.txt"

ACTUAL=$(cat $SCRIPT_DIR/output/plaintext.txt)

if [[ "$ACTUAL" != "$PLAINTEXT" ]]; then
  echo "Invalid plaintext decrypted: actual='$ACTUAL' expected='$PLAINTEXT'"
  exit 1
fi

echo "Test PASSED"

cleanup 0

