#!/usr/bin/env bash
set -euo pipefail  # Stricter error handling

# Get the script's location
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
PLAINTEXT="1234,567890"
ID=$(date +%s)

if [[ "$SCRIPT_DIR" != "$(pwd)" ]]; then 
  echo "This script must be run from the test folder"
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

if command -v enclave >/dev/null 2>&1; then
   ENCLAVE_BIN="enclave"
elif [[ -f "$ROOT_DIR/target/debug/enclave" ]]; then
   ENCLAVE_BIN="$ROOT_DIR/target/debug/enclave"
else
   cargo build --bin enclave
   ENCLAVE_BIN="$ROOT_DIR/target/debug/enclave"
fi
echo "Enclave binary: $ENCLAVE_BIN"

# Function to clean up background processes
cleanup() {
    echo "Cleaning up processes..."
    jobs -p | xargs -r kill -9 2>/dev/null || true
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

strip_ansi() {
    sed 's/\x1b\[[0-9;]*m//g'
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

enclave_password_set() {
  local name="$1"
  local password="$2"
  $ENCLAVE_BIN password set \
    --name $name \
    --config "$SCRIPT_DIR/enclave.config.yaml" \
    --password "$password"
}

enclave_start() {
   local name="$1"
   heading "Launch ciphernode $name"

   # convert OTEL env var to args
   local extra_args=""
   if [[ -n "${OTEL+x}" ]] && [[ -n "$OTEL" ]]; then
      extra_args="--otel=${OTEL}"
   fi

   $ENCLAVE_BIN start -v \
     --name "$name" \
     --config "$SCRIPT_DIR/enclave.config.yaml" $extra_args & 
}

enclave_nodes_up() {
   $ENCLAVE_BIN nodes up -v \
     --config "$SCRIPT_DIR/enclave.config.yaml" & 
}

enclave_nodes_down() {
  $ENCLAVE_BIN nodes down  
}

enclave_wallet_set() {
  local name="$1"
  local private_key="$2"

  $ENCLAVE_BIN wallet set \
    --name $name \
    --config "$SCRIPT_DIR/enclave.config.yaml" \
    --private-key "$private_key"
}

enclave_net_set_key() {
  local name="$1"
  local private_key="$2"

  $ENCLAVE_BIN net set-key \
    --name $name \
    --config "$SCRIPT_DIR/enclave.config.yaml" \
    --net-keypair "$private_key"
}

enclave_nodes_stop() {
  local name="$1"

  $ENCLAVE_BIN nodes stop $name -v \
    --config "$SCRIPT_DIR/enclave.config.yaml"
}

enclave_nodes_start() {
  local name="$1"

  $ENCLAVE_BIN nodes start $name -v \
    --config "$SCRIPT_DIR/enclave.config.yaml"
}

kill_proc() {
  local name=$1
  local pid=$(ps aux | grep 'enclave' | grep "\--name $name" | awk '{ print $2 }')
  echo "Killing $pid"
  kill $pid
}

kill_em_all() {
  pkill -9 -f "target/debug/enclave" || true
  pkill -9 -f "hardhat" || true
}

launch_evm() {
  if [ ! -z "${SILENT_EVM:-}" ]; then
    pnpm evm:node &> /dev/null &
  else
    pnpm evm:node &
  fi
}

ensure_process_count_equals() {
  local process_name="$1"
  local expected_count="$2"
  local actual_count=$(pgrep -f "$process_name" | wc -l)
  [ "$actual_count" -eq "$expected_count" ]
  return $?
}

gracefull_shutdown() {
  enclave_nodes_down
  echo "waiting 5 seconds for processes to shutdown"
  sleep 5
  ensure_process_count_equals "target/debug/enclave" 0 || return 1
  kill_em_all
}

# Run this at the start of every test to ensure we start with a clean slate
kill_em_all

# Set up trap to catch errors and interrupts
trap 'cleanup $?' ERR INT TERM

$SCRIPT_DIR/lib/clean_folders.sh "$SCRIPT_DIR"


