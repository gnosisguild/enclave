#!/usr/bin/env bash

set -eu  # Exit immediately if a command exits with a non-zero status

# Get the script's location
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
PLAINTEXT="1234,567890"

if [[ "$ROOT_DIR" != "$(pwd)" ]]; then 
  echo "This script must be run from the root"
  exit 1
fi

export RUST_LOG=info

# Environment variables
RPC_URL="ws://localhost:8545"

PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
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
      --config "$SCRIPT_DIR/lib/$name/config.yaml" &
}

set_private_key() {
  local name="$1"
  local private_key="$2"

  yarn enclave wallet set \
    --config "$SCRIPT_DIR/lib/$name/config.yaml" \
    --private-key "$private_key"
}

launch_aggregator() {
    local name="$1"
    heading "Launch aggregator $name"

    yarn enclave aggregator start \
      --config "$SCRIPT_DIR/lib/$name/config.yaml" \
      --pubkey-write-path "$SCRIPT_DIR/output/pubkey.bin" \
      --plaintext-write-path "$SCRIPT_DIR/output/plaintext.txt" &
}



pkill -9 -f "target/debug/enclave" || true
pkill -9 -f "hardhat node" || true

# Set up trap to catch errors and interrupts
trap 'cleanup $?' ERR INT TERM


$SCRIPT_DIR/lib/clean_folders.sh "$SCRIPT_DIR"
$SCRIPT_DIR/lib/prebuild.sh

heading "Start the EVM node"

yarn evm:node &

until curl -f -s "http://localhost:8545" > /dev/null; do
  sleep 1
done

# Set the password for all ciphernodes
set_password cn1 "$CIPHERNODE_SECRET"
set_password cn2 "$CIPHERNODE_SECRET"
set_password cn3 "$CIPHERNODE_SECRET"
set_password cn4 "$CIPHERNODE_SECRET"
set_password ag "$CIPHERNODE_SECRET"

set_private_key ag "$PRIVATE_KEY"

# Launch 4 ciphernodes
launch_ciphernode cn1
launch_ciphernode cn2
launch_ciphernode cn3
launch_ciphernode cn4
launch_aggregator ag

sleep 1

waiton-files "$ROOT_DIR/packages/ciphernode/target/debug/enclave" "$ROOT_DIR/packages/ciphernode/target/debug/fake_encrypt"

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
echo -e "\033[32m                                                              
                                               ██████         
                                             ██████           
                                           ██████             
                                         ██████               
                                       ██████                 
                                     ██████                   
                       ██          ██████                     
                       ████      ██████                       
                       ██████  ██████                         
                        ██████████                            
                         ████████                             
                          ██████                              
                           ████                               
                            ██                                
                                                              \033[0m"

pkill -15 -f "target/debug/enclave" || true
pkill -15 -f "target/debug/aggregator" || true

sleep 4

cleanup 0

