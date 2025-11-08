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

PRIVATE_KEY_AG="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
PRIVATE_KEY_CN1="0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d"
PRIVATE_KEY_CN2="0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a"
PRIVATE_KEY_CN3="0x7c852118294e51e653712a81e05800f419141751be58f605c371e15141b007a6"
PRIVATE_KEY_CN4="0x47e179ec197488593b187f80a00eb0da91f1b9d0b13f8733639f19c30a34926a"
PRIVATE_KEY_CN5="0x8b3a350cf5c34c9194ca85829a2df0ec3153be0318b5e2d3348e872092edffba"
CIPHERNODE_SECRET="We are the music makers and we are the dreamers of the dreams."

# These are random addresses for now
CIPHERNODE_ADDRESS_1="0x70997970C51812dc3A010C7d01b50e0d17dc79C8"
CIPHERNODE_ADDRESS_2="0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC"
CIPHERNODE_ADDRESS_3="0x90F79bf6EB2c4f870365E785982E1f101E93b906"
CIPHERNODE_ADDRESS_4="0x15d34AAf54267DB7D7c367839AAf71A00a2C6A65"
CIPHERNODE_ADDRESS_5="0x9965507D1a55bcC2695C58ba16FB37d819B0A4dc"


if command -v enclave >/dev/null 2>&1; then
   ENCLAVE_BIN="enclave"
elif [[ -f "$ROOT_DIR/target/debug/enclave" ]]; then
   ENCLAVE_BIN="$ROOT_DIR/target/debug/enclave"
else
   cargo build --locked --bin enclave
   ENCLAVE_BIN="$ROOT_DIR/target/debug/enclave"
fi
echo "Enclave binary: $ENCLAVE_BIN"

# Function to clean up background processes
cleanup() {
    echo "Cleaning up processes..."
    jobs -p | xargs -r kill -9 2>/dev/null || true
    pkill -9 -f "target/debug/enclave" || true
    pkill -9 -f "hardhat" || true
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
     --config "$SCRIPT_DIR/enclave.config.yaml" --experimental-trbfv & 
}

# enclave_nodes_up() {
#    $ENCLAVE_BIN nodes up -v \
#      --config "$SCRIPT_DIR/enclave.config.yaml" & 
# }

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
  echo "Killing enclave"
  pkill -9 -f "target/debug/enclave" || true
  pkill -9 -f "enclave start" || true
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
