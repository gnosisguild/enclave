#!/usr/bin/env bash
set -euo pipefail  # Stricter error handling

# Get the script's location
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
PLAINTEXT="1234,567890"
ID=$(date +%s)

if [[ "$ROOT_DIR" != "$(pwd)" ]]; then 
  echo "This script must be run from the root"
  exit 1
fi

# Environment variables
RPC_URL="ws://localhost:8545"

PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
NETWORK_PRIVATE_KEY_AG="0x51a1e500a548b70d88184a1e042900c0ed6c57f8710bcc35dc8c85fa33d3f580"
CIPHERNODE_SECRET="We are the music makers and we are the dreamers of the dreams."

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

# These are the network private keys for the ciphernodes
NETWORK_PRIVATE_KEY_1="0x11a1e500a548b70d88184a1e042900c0ed6c57f8710bcc35dc8c85fa33d3f580"
NETWORK_PRIVATE_KEY_2="0x21a1e500a548b70d88184a1e042900c0ed6c57f8710bcc35dc8c85fa33d3f580"
NETWORK_PRIVATE_KEY_3="0x31a1e500a548b70d88184a1e042900c0ed6c57f8710bcc35dc8c85fa33d3f580"
NETWORK_PRIVATE_KEY_4="0x41a1e500a548b70d88184a1e042900c0ed6c57f8710bcc35dc8c85fa33d3f580"


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

set_password() {
  local name="$1"
  local password="$2"
  yarn enclave password create \
    --config "$SCRIPT_DIR/lib/$name/config.yaml" \
    --password "$password"
}

launch_ciphernode() {
    local name="$1"
    heading "Launch ciphernode $name"
    yarn enclave start \
      --tag "$name" \
      --config "$SCRIPT_DIR/lib/$name/config.yaml" & echo $! > "/tmp/enclave.${ID}_${name}.pid"
}

set_private_key() {
  local name="$1"
  local private_key="$2"

  yarn enclave wallet set \
    --config "$SCRIPT_DIR/lib/$name/config.yaml" \
    --private-key "$private_key"
}

set_network_private_key() {
  local name="$1"
  local private_key="$2"

  yarn enclave net set-key \
    --config "$SCRIPT_DIR/lib/$name/config.yaml" \
    --net-keypair "$private_key"
}

launch_aggregator() {
    local name="$1"
    heading "Launch aggregator $name"

    yarn enclave aggregator start \
      --tag "$name" \
      --config "$SCRIPT_DIR/lib/$name/config.yaml" \
      --pubkey-write-path "$SCRIPT_DIR/output/pubkey.bin" \
      --plaintext-write-path "$SCRIPT_DIR/output/plaintext.txt" & echo $! > "/tmp/enclave.${ID}_${name}.pid"

    ps aux | grep aggregator
}

kill_proc() {
  local name=$1
  local pid=$(ps aux | grep 'enclave' | grep "\--tag $name" | awk '{ print $2 }')
  echo "Killing $pid"
  kill $pid
}

metallica() {
  pkill -9 -f "target/debug/enclave" || true
  pkill -9 -f "hardhat node" || true
}

launch_evm() {
  if [ ! -z "${SILENT_EVM:-}" ]; then
    yarn evm:node &> /dev/null &
  else
    yarn evm:node &
  fi
}

metallica

# Set up trap to catch errors and interrupts
trap 'cleanup $?' ERR INT TERM

$SCRIPT_DIR/lib/clean_folders.sh "$SCRIPT_DIR"
$SCRIPT_DIR/lib/prebuild.sh


